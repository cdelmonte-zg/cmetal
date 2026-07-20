//! What "running an exercise" means — the decision, and nothing else.
//!
//! This ladder used to be copied in four places (watch mode, `run`,
//! `verify`, and the gate in `solution`), which is how their wording
//! drifted apart. It now exists once, and it never prints: rendering
//! lives in [`crate::view`], so the decision can be unit-tested without
//! capturing stdout.
//!
//! Two things deliberately stay with the caller. *Presentation* varies
//! (a batch command wants one terse line where `run` wants the full
//! body), and so does *persistence* of a pass: `run` records it
//! immediately, watch mode only when the learner presses `n`, and
//! `verify` in one batch at the end.

use crate::compiler::Compiler;
use crate::exercise::{Exercise, VerifyResult};
use anyhow::Result;
use std::path::Path;

/// The verdict on one exercise.
///
/// Every variant is a statement *about the exercise*. A failure of the
/// tooling itself is not one of them: [`evaluate`] returns `Err` for
/// that, so a broken toolchain never masquerades as a wrong answer.
#[derive(Debug)]
pub enum RunStatus {
    Passed(VerifyResult),
    Failed(VerifyResult),
    /// The exercise requires a compiler other than the selected one.
    Unsupported,
    /// The working copy is not on disk.
    Missing,
}

impl RunStatus {
    pub fn passed(&self) -> bool {
        matches!(self, RunStatus::Passed(_))
    }
}

/// The supported → exists → verify ladder. A change to what counts as
/// "solved" happens here and nowhere else.
pub fn evaluate(exercise: &Exercise, compiler: &Compiler, build_dir: &Path) -> Result<RunStatus> {
    if !exercise.supported {
        return Ok(RunStatus::Unsupported);
    }
    if !exercise.exists() {
        return Ok(RunStatus::Missing);
    }
    let result = exercise.verify(compiler, build_dir)?;
    Ok(if result.success {
        RunStatus::Passed(result)
    } else {
        RunStatus::Failed(result)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CompilerKind;
    use crate::info_file::ExerciseInfo;
    use std::path::PathBuf;

    fn make_exercise(name: &str, dir: &Path) -> Exercise {
        Exercise {
            info: ExerciseInfo {
                name: name.to_string(),
                dir: "test".to_string(),
                test: false,
                sanitizers: false,
                flags: Vec::new(),
                compilers: Some(vec!["gcc".to_string()]),
                hint: None,
                hints: None,
            },
            path: dir.join(format!("{name}.c")),
            solution_path: PathBuf::from("/nonexistent"),
            reveal_path: dir.join(format!("{name}_revealed.c")),
            supported: true,
        }
    }

    /// A `Compiler` value is needed to call `evaluate`, but these tests
    /// never reach a compile: the ladder returns first.
    fn compiler_for(dir: &Path) -> Option<Compiler> {
        Compiler::new(CompilerKind::Gcc, dir).ok()
    }

    /// The ladder must reject an unsupported exercise BEFORE looking at
    /// the filesystem: an exercise this compiler cannot judge is
    /// "skipped", never "file not found", however its files look.
    #[test]
    fn unsupported_wins_over_missing_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let Some(compiler) = compiler_for(tmp.path()) else {
            eprintln!("skipping: no gcc available");
            return;
        };
        let mut ex = make_exercise("ghost", tmp.path());
        ex.supported = false;
        assert!(!ex.exists(), "the test relies on the file being absent");

        let status = evaluate(&ex, &compiler, tmp.path()).unwrap();
        assert!(matches!(status, RunStatus::Unsupported));
        assert!(!status.passed());
    }

    /// A supported exercise whose working copy is gone is reported as
    /// missing rather than handed to the compiler.
    #[test]
    fn missing_working_copy_is_not_compiled() {
        let tmp = tempfile::TempDir::new().unwrap();
        let Some(compiler) = compiler_for(tmp.path()) else {
            eprintln!("skipping: no gcc available");
            return;
        };
        let ex = make_exercise("ghost", tmp.path());

        let status = evaluate(&ex, &compiler, tmp.path()).unwrap();
        assert!(matches!(status, RunStatus::Missing));
    }
}
