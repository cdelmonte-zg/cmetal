use crate::compiler::Compiler;
use crate::info_file::ExerciseInfo;
use anyhow::Result;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Kill a compiled exercise after this long. Learner code loops
/// forever all the time; watch mode must survive it. Kept in sync
/// with RUN_TIMEOUT in scripts/check_exercises.py (a unit test in
/// compiler.rs enforces the match).
pub(crate) const RUN_TIMEOUT: Duration = Duration::from_secs(10);

/// Cap on captured output, per stream. A program that prints in a
/// loop until the timeout kills it would otherwise hand us hundreds
/// of megabytes. The reader keeps DRAINING past the cap — stopping
/// would re-introduce the pipe-full deadlock — and discards the
/// excess.
const MAX_CAPTURED_OUTPUT: usize = 1_048_576;

pub struct Exercise {
    pub info: ExerciseInfo,
    pub path: PathBuf,
    pub solution_path: PathBuf,
    /// Where the decoded solution is revealed once the exercise is solved.
    pub reveal_path: PathBuf,
    /// Whether this exercise supports the compiler selected for this run.
    pub supported: bool,
}

#[derive(Debug)]
pub struct VerifyResult {
    pub success: bool,
    pub stage: &'static str,
    pub output: String,
}

impl Exercise {
    pub fn new(
        info: ExerciseInfo,
        exercises_dir: &Path,
        solutions_dir: &Path,
        compiler: &str,
    ) -> Self {
        let rel = info.rel_path();
        let path = exercises_dir.join(&rel);
        let solution_path = solutions_dir.join(&rel);
        let base_dir = solutions_dir.parent().unwrap_or(solutions_dir);
        let reveal_path = base_dir.join("my_solutions").join(&rel);
        let supported = info.supports_compiler(compiler);
        Self {
            info,
            path,
            solution_path,
            reveal_path,
            supported,
        }
    }

    /// Human-readable list of compilers this exercise requires,
    /// e.g. "gcc" or "gcc, clang". Empty when unrestricted.
    pub fn required_compilers(&self) -> String {
        self.info
            .compilers
            .as_deref()
            .unwrap_or_default()
            .join(", ")
    }

    /// Decode the official solution and write it to `my_solutions/`.
    /// Prefers the obfuscated `.c.enc` file; falls back to a plaintext
    /// `.c` (the unpacked form contributors work with).
    pub fn reveal_solution(&self) -> Result<PathBuf> {
        let enc_path = self.solution_path.with_extension("c.enc");
        let content = if enc_path.exists() {
            crate::solutions::decode(&std::fs::read_to_string(&enc_path)?)?
        } else if self.solution_path.exists() {
            std::fs::read_to_string(&self.solution_path)?
        } else {
            anyhow::bail!("No solution available for {}", self.name());
        };
        if let Some(parent) = self.reveal_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.reveal_path, content)?;
        Ok(self.reveal_path.clone())
    }

    pub fn name(&self) -> &str {
        &self.info.name
    }

    pub fn hints(&self) -> Vec<&str> {
        self.info.get_hints()
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn verify(&self, compiler: &Compiler, build_dir: &Path) -> Result<VerifyResult> {
        std::fs::create_dir_all(build_dir)?;

        let bin_path = build_dir.join(&self.info.name);

        // Step 1: Compile
        let result = compiler.compile(&self.path, &bin_path, &self.info.flags)?;
        if !result.success {
            return Ok(VerifyResult {
                success: false,
                stage: "compilation",
                output: result.output,
            });
        }

        // Step 2: Run the binary
        let run_result = run_binary(&bin_path)?;
        if !run_result.success {
            return Ok(VerifyResult {
                success: false,
                stage: "execution",
                output: run_result.output,
            });
        }
        let run_output = run_result.output.clone();

        // Step 3: Compile and run with tests (if enabled)
        if self.info.test {
            let test_bin = build_dir.join(format!("{}_test", self.info.name));
            let result = compiler.compile_with_tests(&self.path, &test_bin, &self.info.flags)?;
            if !result.success {
                return Ok(VerifyResult {
                    success: false,
                    stage: "test compilation",
                    output: result.output,
                });
            }

            let test_result = run_binary(&test_bin)?;
            if !test_result.success {
                return Ok(VerifyResult {
                    success: false,
                    stage: "tests",
                    output: test_result.output,
                });
            }
        }

        // Step 4: Compile and run with sanitizers (if enabled)
        if self.info.sanitizers {
            let san_bin = build_dir.join(format!("{}_san", self.info.name));
            let result =
                compiler.compile_with_sanitizers(&self.path, &san_bin, &self.info.flags)?;
            if !result.success {
                return Ok(VerifyResult {
                    success: false,
                    stage: "sanitizer compilation",
                    output: result.output,
                });
            }

            let san_result = run_binary(&san_bin)?;
            if !san_result.success {
                return Ok(VerifyResult {
                    success: false,
                    stage: "sanitizer check",
                    output: san_result.output,
                });
            }
        }

        Ok(VerifyResult {
            success: true,
            stage: "complete",
            output: run_output,
        })
    }
}

struct RunResult {
    success: bool,
    output: String,
}

fn run_binary(path: &Path) -> Result<RunResult> {
    run_binary_with_timeout(path, RUN_TIMEOUT)
}

fn run_binary_with_timeout(path: &Path, timeout: Duration) -> Result<RunResult> {
    let mut child = Command::new(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Drain the pipes on their own threads: a child that fills a pipe
    // while we only poll try_wait() would block forever on write.
    let stdout_reader = spawn_pipe_reader(child.stdout.take());
    let stderr_reader = spawn_pipe_reader(child.stderr.take());

    let deadline = Instant::now() + timeout;
    let (status, timed_out) = loop {
        if let Some(status) = child.try_wait()? {
            break (Some(status), false);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            break (None, true);
        }
        std::thread::sleep(Duration::from_millis(20));
    };

    let stdout = stdout_reader.join().unwrap_or_default();
    let stderr = stderr_reader.join().unwrap_or_default();

    let mut combined = String::new();
    if !stdout.bytes.is_empty() {
        combined.push_str(&String::from_utf8_lossy(&stdout.bytes));
    }
    if !stderr.bytes.is_empty() {
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(&String::from_utf8_lossy(&stderr.bytes));
    }
    if stdout.truncated || stderr.truncated {
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str("[output truncated at 1 MiB per stream]");
    }

    if timed_out {
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(&format!(
            "Timed out after {} seconds and was killed. \
             Is there an infinite loop?",
            timeout.as_secs()
        ));
        return Ok(RunResult {
            success: false,
            output: combined,
        });
    }

    Ok(RunResult {
        success: status.map(|s| s.success()).unwrap_or(false),
        output: combined,
    })
}

#[derive(Default)]
struct CapturedOutput {
    bytes: Vec<u8>,
    truncated: bool,
}

fn spawn_pipe_reader<R: Read + Send + 'static>(
    pipe: Option<R>,
) -> std::thread::JoinHandle<CapturedOutput> {
    std::thread::spawn(move || {
        let mut captured = CapturedOutput::default();
        let Some(mut pipe) = pipe else {
            return captured;
        };
        let mut chunk = [0u8; 8192];
        loop {
            match pipe.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let remaining = MAX_CAPTURED_OUTPUT.saturating_sub(captured.bytes.len());
                    let keep = remaining.min(n);
                    captured.bytes.extend_from_slice(&chunk[..keep]);
                    if keep < n {
                        captured.truncated = true;
                        // keep looping: the pipe must stay drained
                    }
                }
            }
        }
        captured
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The project supports gcc OR clang: honor $CC, then try both.
    /// Returns None (tests skip) when no C compiler is available.
    fn find_cc() -> Option<String> {
        let candidates = match std::env::var("CC") {
            Ok(cc) if !cc.is_empty() => vec![cc],
            _ => vec!["gcc".to_string(), "clang".to_string()],
        };
        candidates.into_iter().find(|cc| {
            Command::new(cc)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        })
    }

    /// Compile a C snippet into `dir`; the caller keeps the TempDir
    /// alive for as long as the binary is needed.
    fn compile_snippet(cc: &str, dir: &Path, name: &str, code: &str) -> PathBuf {
        let src = dir.join(format!("{name}.c"));
        let bin = dir.join(name);
        std::fs::write(&src, code).unwrap();
        let status = Command::new(cc)
            .arg(&src)
            .arg("-o")
            .arg(&bin)
            .status()
            .unwrap();
        assert!(status.success());
        bin
    }

    macro_rules! require_cc {
        () => {
            match find_cc() {
                Some(cc) => cc,
                None => {
                    eprintln!("skipping: no C compiler available");
                    return;
                }
            }
        };
    }

    #[test]
    fn infinite_loop_is_killed_and_reported() {
        let cc = require_cc!();
        let tmp = tempfile::TempDir::new().unwrap();
        let bin = compile_snippet(
            &cc,
            tmp.path(),
            "spin",
            "int main(void) { volatile int x = 1; while (x) {} }",
        );
        let start = Instant::now();
        let result = run_binary_with_timeout(&bin, Duration::from_secs(1)).unwrap();
        assert!(!result.success);
        assert!(result.output.contains("Timed out"));
        // killed near the deadline, not at some multiple of it
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn normal_exit_and_output_are_captured() {
        let cc = require_cc!();
        let tmp = tempfile::TempDir::new().unwrap();
        let bin = compile_snippet(
            &cc,
            tmp.path(),
            "hello",
            "#include <stdio.h>\nint main(void) { puts(\"hi\"); return 0; }",
        );
        let result = run_binary(&bin).unwrap();
        assert!(result.success);
        assert!(result.output.contains("hi"));
    }

    #[test]
    fn large_output_does_not_deadlock() {
        // More than any pipe buffer: without dedicated reader threads
        // the child blocks on write and the timeout kills a healthy
        // program. 680 KB stays under the capture cap: no truncation.
        let cc = require_cc!();
        let tmp = tempfile::TempDir::new().unwrap();
        let bin = compile_snippet(
            &cc,
            tmp.path(),
            "chatty",
            "#include <stdio.h>\nint main(void) {\n\
             for (int i = 0; i < 40000; i++) printf(\"0123456789abcdef\\n\");\n\
             return 0; }",
        );
        let result = run_binary(&bin).unwrap();
        assert!(result.success);
        assert!(result.output.len() > 500_000);
        assert!(!result.output.contains("output truncated"));
    }

    #[test]
    fn output_flood_is_bounded_and_drained() {
        // The worst case both fixes must survive together: a program
        // that prints forever. The reader must keep draining (or the
        // child deadlocks on a full pipe long before the timeout) but
        // keep only MAX_CAPTURED_OUTPUT per stream.
        let cc = require_cc!();
        let tmp = tempfile::TempDir::new().unwrap();
        let bin = compile_snippet(
            &cc,
            tmp.path(),
            "flood",
            "#include <stdio.h>\nint main(void) {\n\
             while (1) puts(\"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\");\n\
             }",
        );
        let result = run_binary_with_timeout(&bin, Duration::from_secs(1)).unwrap();
        assert!(!result.success);
        assert!(result.output.contains("Timed out"));
        assert!(result.output.contains("output truncated"));
        // both streams capped, plus the two notes: nowhere near the
        // hundreds of MB the child produced
        assert!(result.output.len() <= 2 * MAX_CAPTURED_OUTPUT + 1024);
    }
}
