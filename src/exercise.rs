use crate::compiler::Compiler;
use crate::info_file::ExerciseInfo;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    let output = Command::new(path).output()?;

    let mut combined = String::new();
    if !output.stdout.is_empty() {
        combined.push_str(&String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(&String::from_utf8_lossy(&output.stderr));
    }

    Ok(RunResult {
        success: output.status.success(),
        output: combined,
    })
}
