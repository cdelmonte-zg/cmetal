//! How exercise state is rendered.
//!
//! Every wording the learner sees for a [`RunStatus`] lives here, in
//! one file, so the two renderings stay reviewable side by side: the
//! full body for a single exercise, and the one-liner batch commands
//! use. They differ in layout on purpose — they must not drift in
//! vocabulary, which is what happened when each command carried its
//! own copy.
//!
//! Everything here goes through [`crate::term`], which terminates lines
//! with `\r\n`; that makes it safe to call from watch mode's raw-mode
//! alternate screen as well as from an ordinary shell.

use crate::app_state::AppState;
use crate::compiler::Compiler;
use crate::exercise::Exercise;
use crate::runner::RunStatus;
use crate::term;
use std::path::Path;

/// The full report for one exercise: verdict, captured output, and —
/// when the caller has already revealed it — where the official
/// solution landed. Callers add their own framing (headers) and
/// navigation advice around it.
///
/// `revealed` is passed in rather than produced here: writing the
/// solution file is an action the command takes (see
/// [`crate::runner::reveal_if_passed`]), and rendering must not have
/// side effects.
pub fn report_outcome(
    exercise: &Exercise,
    compiler: &Compiler,
    status: &RunStatus,
    revealed: Option<&Path>,
) {
    match status {
        RunStatus::Passed(result) => {
            term::print_success(&format!("{} passed!", exercise.name()));
            if !result.output.is_empty() {
                // "Program", not "Output": print_stage_output already
                // appends the word "output" to the label.
                term::print_stage_output("Program", &result.output);
            }
            if let Some(path) = revealed {
                println!("\r");
                term::print_info(&format!(
                    "Official solution revealed: {} — compare it with yours!",
                    path.display()
                ));
            }
        }
        RunStatus::Failed(result) => {
            term::print_error(&format!(
                "{} failed at stage: {}",
                exercise.name(),
                result.stage
            ));
            term::print_stage_output(result.stage, &result.output);
        }
        RunStatus::Unsupported => {
            term::print_warning(&format!(
                "{} requires {} (current compiler: {}).",
                exercise.name(),
                exercise.required_compilers(),
                compiler.kind()
            ));
        }
        RunStatus::Missing => {
            term::print_error(&format!(
                "Exercise file not found: {}",
                exercise.path.display()
            ));
        }
    }
}

/// One line per exercise, for sweeps over the whole curriculum where
/// the full body would bury the result.
pub fn report_terse(exercise: &Exercise, status: &RunStatus) {
    match status {
        RunStatus::Passed(_) => term::print_success(exercise.name()),
        RunStatus::Failed(result) => term::print_error(&format!(
            "{} (failed at: {})",
            exercise.name(),
            result.stage
        )),
        // Skipped is a warning because it does not fail the sweep;
        // missing is an error because it does. The glyphs have to
        // match `verify`'s verdict, or a run exits non-zero with
        // nothing in the per-exercise lines marked as failing.
        RunStatus::Unsupported => term::print_warning(&format!(
            "{}: skipped (requires {})",
            exercise.name(),
            exercise.required_compilers()
        )),
        RunStatus::Missing => {
            term::print_error(&format!("{}: file not found", exercise.name()));
        }
    }
}

/// Status glyph for the exercise lists (`cmetal list` and watch's `l`).
/// The two render different layouts but must agree on what the markers
/// mean, so the marker logic is shared even though the loops are not.
pub fn status_marker(state: &AppState, index: usize, exercise: &Exercise) -> &'static str {
    if state.is_done(exercise.name()) {
        "✓"
    } else if !exercise.supported {
        "−"
    } else if index == state.current_index {
        "→"
    } else {
        " "
    }
}

/// The trailing "(requires gcc)" note, empty when the exercise runs on
/// the selected compiler.
pub fn compiler_note(exercise: &Exercise) -> String {
    if exercise.supported {
        String::new()
    } else {
        format!("  (requires {})", exercise.required_compilers())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::info_file::ExerciseInfo;
    use std::path::{Path, PathBuf};

    fn make_exercise(name: &str, dir: &Path, supported: bool) -> Exercise {
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
            supported,
        }
    }

    /// A solved exercise reads as solved even when the selected
    /// compiler could no longer judge it.
    #[test]
    fn done_beats_unsupported() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = {
            let mut s =
                AppState::from_parts(vec![make_exercise("a", tmp.path(), false)], tmp.path());
            s.mark_done("a");
            s
        };
        assert_eq!(status_marker(&state, 0, &state.exercises[0]), "✓");
    }

    /// An unsupported exercise keeps its glyph even while it is the
    /// current one — otherwise the list would invite the learner to
    /// work on something this compiler cannot verify.
    #[test]
    fn unsupported_beats_current() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut state =
            AppState::from_parts(vec![make_exercise("a", tmp.path(), false)], tmp.path());
        state.current_index = 0;
        assert_eq!(status_marker(&state, 0, &state.exercises[0]), "−");
    }

    #[test]
    fn current_and_plain_exercises() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut state = AppState::from_parts(
            vec![
                make_exercise("a", tmp.path(), true),
                make_exercise("b", tmp.path(), true),
            ],
            tmp.path(),
        );
        state.current_index = 1;
        assert_eq!(status_marker(&state, 0, &state.exercises[0]), " ");
        assert_eq!(status_marker(&state, 1, &state.exercises[1]), "→");
    }

    #[test]
    fn compiler_note_only_for_unsupported() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert_eq!(compiler_note(&make_exercise("a", tmp.path(), true)), "");
        assert_eq!(
            compiler_note(&make_exercise("b", tmp.path(), false)),
            "  (requires gcc)"
        );
    }
}
