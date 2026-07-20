//! One function per subcommand.
//!
//! These are thin: the verification ladder lives in [`crate::runner`],
//! the rendering in [`crate::view`], and the workspace mechanics in
//! [`crate::workspace`]. What remains here is argument handling,
//! framing, and each command's own persistence policy.
//!
//! Some commands take an [`AppState`] and a [`Compiler`]; others take
//! only a base directory because they never compile anything. That is
//! a property of the command, not of where it is filed — `main`
//! dispatches the compiler-free ones before the engine starts up.

use crate::app_state::AppState;
use crate::compiler::{Compiler, CompilerKind};
use crate::info_file::{ExerciseInfo, InfoFile};
use crate::runner::{self, RunStatus};
use crate::term;
use crate::view;
use crate::workspace;
use anyhow::{Context, Result};
use std::path::Path;

/// `cmetal run [name]` — verify one exercise and persist a pass
/// immediately. Exits non-zero when it does not pass, so the command
/// composes in scripts and CI.
pub fn run(
    state: &mut AppState,
    compiler: &Compiler,
    build_dir: &Path,
    name: Option<String>,
) -> Result<()> {
    let idx = state.resolve(name)?;

    println!();
    term::print_header(&format!("Running: {}", state.exercises[idx].name()));
    println!();

    let exercise = &state.exercises[idx];
    let status = runner::evaluate(exercise, compiler, build_dir)?;
    view::report_outcome(exercise, compiler, &status);

    match status {
        RunStatus::Passed(_) => {
            let name = state.exercises[idx].name().to_string();
            state.complete(&name)?;
        }
        // A skipped exercise is not a failure: the learner asked for an
        // exercise this compiler cannot judge, and was told so.
        RunStatus::Unsupported => {
            term::print_info("Re-run with --compiler to use the required compiler.");
            println!();
        }
        RunStatus::Failed(_) | RunStatus::Missing => std::process::exit(1),
    }
    Ok(())
}

/// `cmetal hint [name] --level N` — progressive hints, no compilation.
pub fn hint(state: &AppState, name: Option<String>, level: usize) -> Result<()> {
    let idx = state.resolve(name)?;
    let exercise = &state.exercises[idx];
    let hints = exercise.hints();

    println!();
    if hints.is_empty() {
        term::print_warning(&format!("No hints available for {}.", exercise.name()));
        println!();
        return Ok(());
    }

    let show_up_to = level.min(hints.len());
    for (i, hint) in hints.iter().take(show_up_to).enumerate() {
        term::print_header(&format!(
            "Hint {} of {} for {}:",
            i + 1,
            hints.len(),
            exercise.name()
        ));
        println!();
        for line in hint.lines() {
            println!("  {line}");
        }
        println!();
    }
    if show_up_to < hints.len() {
        term::print_info(&format!(
            "Use --level {} to see the next hint.",
            show_up_to + 1
        ));
    }
    println!();
    Ok(())
}

/// `cmetal solution [name]` — reveal the official solution, gated on
/// the exercise being solved.
///
/// For an exercise the selected compiler cannot judge, `evaluate`
/// returns `Unsupported` and the gate stays shut: a verify pass would
/// be meaningless there (the bug may not even be detectable), so only
/// a previously recorded completion unlocks it.
pub fn solution(
    state: &mut AppState,
    compiler: &Compiler,
    build_dir: &Path,
    name: Option<String>,
) -> Result<()> {
    let idx = state.resolve(name)?;
    let name = state.exercises[idx].name().to_string();

    let already_done = state.is_done(&name);
    let verified_now =
        !already_done && runner::evaluate(&state.exercises[idx], compiler, build_dir)?.passed();

    println!();
    if !already_done && !verified_now {
        term::print_warning(&format!(
            "{name} is not solved yet. Fix it first — then the solution unlocks!"
        ));
        println!();
        std::process::exit(1);
    }

    let path = state.exercises[idx].reveal_solution()?;
    term::print_success(&format!("Solution for {name}: {}", path.display()));
    println!();

    // The verify pass that just unlocked the solution is a completion
    // like any other: persist it, or `cmetal list` keeps showing the
    // exercise as pending.
    if verified_now {
        state.complete(&name)?;
    }
    Ok(())
}

/// `cmetal list` — every exercise grouped by topic directory.
pub fn list(state: &AppState) {
    println!();
    term::print_header("Exercises");
    println!();
    let (done, total) = state.progress();
    term::print_progress(done, total);
    println!();

    let mut current_dir = String::new();
    for (i, exercise) in state.exercises.iter().enumerate() {
        if exercise.info.dir != current_dir {
            current_dir = exercise.info.dir.clone();
            println!("  {current_dir}/");
        }
        println!(
            "    {} {}{}",
            view::status_marker(state, i, exercise),
            exercise.name(),
            view::compiler_note(exercise)
        );
    }
    println!();
}

/// `cmetal verify` — the whole curriculum, terse. Passes are recorded
/// in one batch at the end so an interrupted sweep does not leave
/// progress half-written.
pub fn verify(state: &mut AppState, compiler: &Compiler, build_dir: &Path) -> Result<()> {
    println!();
    term::print_header("Verifying all exercises...");
    println!();

    let mut all_passed = true;
    let mut passed_names = Vec::new();
    for exercise in &state.exercises {
        match runner::evaluate(exercise, compiler, build_dir) {
            Ok(status) => {
                view::report_terse(exercise, &status);
                match status {
                    RunStatus::Passed(_) => passed_names.push(exercise.name().to_string()),
                    RunStatus::Failed(_) => all_passed = false,
                    // Skipped and missing exercises are reported but do
                    // not fail the sweep: neither is a wrong answer.
                    RunStatus::Unsupported | RunStatus::Missing => {}
                }
            }
            // A broken toolchain is not a wrong answer either, but the
            // sweep cannot claim success without having judged this
            // exercise. `{e:#}` keeps anyhow's cause chain.
            Err(e) => {
                term::print_error(&format!("{}: {e:#}", exercise.name()));
                all_passed = false;
            }
        }
    }

    for name in &passed_names {
        state.mark_done(name);
    }
    state.save()?;

    println!();
    if all_passed {
        term::print_success("All exercises passed!");
    } else {
        term::print_error("Some exercises failed.");
        std::process::exit(1);
    }
    Ok(())
}

/// `cmetal reset` with no name — wipe progress and restore every
/// working copy.
pub fn reset_all(state: &AppState, info: &InfoFile, base_dir: &Path) -> Result<()> {
    state.reset()?;
    workspace::restore_workspace(info, base_dir)?;
    println!();
    term::print_success("Progress reset. Workspace restored to pristine exercises!");
    println!();
    Ok(())
}

/// `cmetal reset <name>` — restore one exercise's working copy to the
/// pristine version and mark it pending again, leaving every other
/// exercise's progress untouched. Compiles nothing.
pub fn reset_one(base_dir: &Path, name: &str, compiler_kind: CompilerKind) -> Result<()> {
    let info = InfoFile::parse(&base_dir.join("info.toml"))?;
    let ei = find_exercise_info(&info, name)?;
    workspace::restore_exercise(base_dir, ei)?;

    // Un-done it, or watch mode would never offer it again and the
    // progress count would keep claiming it solved.
    let work_dir = base_dir.join("my_exercises");
    let exercises = workspace::load_exercises(&info, base_dir, &work_dir, compiler_kind);
    let mut state = AppState::new(exercises, base_dir)?;
    state.mark_pending(name);
    state.save()?;

    println!();
    term::print_success(&format!(
        "{name} restored to the pristine exercise and marked pending again \
         (other progress kept)."
    ));
    println!();
    Ok(())
}

/// `cmetal diff <name>` — pristine exercise vs the learner's working
/// copy. Compiles nothing, and reads no progress state.
pub fn diff(base_dir: &Path, name: &str) -> Result<()> {
    let info = InfoFile::parse(&base_dir.join("info.toml"))?;
    let ei = find_exercise_info(&info, name)?;
    let rel = ei.rel_path();
    let pristine = std::fs::read_to_string(base_dir.join("exercises").join(&rel))
        .with_context(|| format!("Failed to read pristine {}", rel.display()))?;
    let my_path = base_dir.join("my_exercises").join(&rel);
    if !my_path.exists() {
        write_stdout(&format!(
            "\n{name}: no working copy yet — it is created on the next `cmetal` run.\n\n"
        ))?;
        return Ok(());
    }
    let mine = std::fs::read_to_string(&my_path)
        .with_context(|| format!("Failed to read your copy of {}", rel.display()))?;
    if pristine == mine {
        write_stdout(&format!(
            "\n{name}: your copy is identical to the pristine exercise.\n\n"
        ))?;
    } else {
        let diff = similar::TextDiff::from_lines(&pristine, &mine);
        let text = format!(
            "\n{}\n",
            diff.unified_diff().context_radius(3).header(
                &format!("exercises/{} (pristine)", rel.display()),
                &format!("my_exercises/{} (yours)", rel.display()),
            )
        );
        write_stdout(&text)?;
    }
    Ok(())
}

/// Looks up an exercise's metadata by name. The compiler-free commands
/// resolve against the info file because they run without an engine;
/// the rest go through `AppState::resolve`, which also honours "the
/// current exercise" when no name is given.
fn find_exercise_info<'a>(info: &'a InfoFile, name: &str) -> Result<&'a ExerciseInfo> {
    info.exercises
        .iter()
        .find(|ei| ei.name == name)
        .with_context(|| format!("Exercise '{name}' not found"))
}

/// Writes to stdout, tolerating ONLY a closed pipe (`cmetal diff x |
/// head`); any other write failure is a real error and propagates.
fn write_stdout(text: &str) -> Result<()> {
    use std::io::Write;
    match std::io::stdout().write_all(text.as_bytes()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(e).context("Failed to write to stdout"),
    }
}
