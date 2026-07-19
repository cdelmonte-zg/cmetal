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

pub struct Exercise {
    pub info: ExerciseInfo,
    pub path: PathBuf,
    pub solution_path: PathBuf,
    /// Where the decoded solution is revealed once the exercise is solved.
    pub reveal_path: PathBuf,
    /// Whether this exercise supports the compiler selected for this run.
    pub supported: bool,
}

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
    if !stdout.is_empty() {
        combined.push_str(&String::from_utf8_lossy(&stdout));
    }
    if !stderr.is_empty() {
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(&String::from_utf8_lossy(&stderr));
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

fn spawn_pipe_reader<R: Read + Send + 'static>(
    pipe: Option<R>,
) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut buf);
        }
        buf
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile a C snippet with the system gcc into a temp binary.
    fn compile_snippet(name: &str, code: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("cmetal-run-binary-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join(format!("{name}.c"));
        let bin = dir.join(name);
        std::fs::write(&src, code).unwrap();
        let status = Command::new("gcc")
            .arg(&src)
            .arg("-o")
            .arg(&bin)
            .status()
            .expect("gcc must be available for these tests");
        assert!(status.success());
        bin
    }

    #[test]
    fn infinite_loop_is_killed_and_reported() {
        let bin = compile_snippet(
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
        let bin = compile_snippet(
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
        // program.
        let bin = compile_snippet(
            "chatty",
            "#include <stdio.h>\nint main(void) {\n\
             for (int i = 0; i < 40000; i++) printf(\"0123456789abcdef\\n\");\n\
             return 0; }",
        );
        let result = run_binary(&bin).unwrap();
        assert!(result.success);
        assert!(result.output.len() > 500_000);
    }
}
