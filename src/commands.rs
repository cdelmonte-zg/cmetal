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
/// immediately.
///
/// The exit-code contract scripts and CI compose on: zero for a pass,
/// non-zero for a wrong answer or a missing working copy. An exercise
/// the selected compiler cannot judge is reported as skipped and also
/// exits zero — it is not a failure, so it must not break a sweep run
/// under one compiler over a curriculum that pins another.
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
    let revealed = runner::reveal_if_passed(exercise, &status);
    view::report_outcome(exercise, compiler, &status, revealed.as_deref());

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
    let ex_name = state.exercises[idx].name().to_string();

    let already_done = state.is_done(&ex_name);
    let verified_now =
        !already_done && runner::evaluate(&state.exercises[idx], compiler, build_dir)?.passed();

    println!();
    if !already_done && !verified_now {
        term::print_warning(&format!(
            "{ex_name} is not solved yet. Fix it first — then the solution unlocks!"
        ));
        println!();
        std::process::exit(1);
    }

    let path = state.exercises[idx].reveal_solution()?;
    term::print_success(&format!("Solution for {ex_name}: {}", path.display()));
    println!();

    // The verify pass that just unlocked the solution is a completion
    // like any other: persist it, or `cmetal list` keeps showing the
    // exercise as pending.
    if verified_now {
        state.complete(&ex_name)?;
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
                    // A missing file is not a wrong answer, but the
                    // sweep must not claim success without having
                    // judged the exercise — that is how a curriculum
                    // shipped with an info.toml entry and no .c file
                    // would pass verification silently.
                    RunStatus::Missing => all_passed = false,
                    // Skipped is different: the exercise is fine, this
                    // compiler simply cannot judge it.
                    RunStatus::Unsupported => {}
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
pub fn reset_all(
    state: &mut AppState,
    info: &InfoFile,
    base_dir: &Path,
    force: bool,
) -> Result<()> {
    let targets: Vec<&ExerciseInfo> = info.exercises.iter().collect();
    if !restore_with_consent(base_dir, &targets, force)? {
        return Ok(());
    }
    state.reset()?;

    println!();
    term::print_success("Progress reset. Workspace restored to pristine exercises!");
    // Saying "reset" while leaving files behind would be a lie, and
    // these are the one thing here worth salvaging — so name them
    // rather than delete them.
    for kept in state.damaged_backups() {
        term::print_info(&format!(
            "Left in place: {} (unreadable progress from an earlier run — \
             delete it once you no longer need it).",
            kept.display()
        ));
    }
    println!();
    Ok(())
}

/// Overwrites working copies, never silently.
///
/// Every command that discards a learner's code goes through here, so
/// the notice lives with the destruction rather than with whichever
/// command was reviewed last — `reset` and `reset <name>` cannot drift
/// apart on how dangerous they admit to being. Returns false when the
/// learner declined, in which case nothing was touched.
fn restore_with_consent(base_dir: &Path, targets: &[&ExerciseInfo], force: bool) -> Result<bool> {
    let edited = workspace::edited_among(base_dir, targets.iter().copied());
    if !edited.is_empty() {
        println!();
        term::print_warning(&format!(
            "This discards your work on {} exercise(s): {}.",
            edited.len(),
            edited.join(", ")
        ));
        // --force answers the question; it does not silence the
        // record, or an unattended run leaves no trace of what went.
        if !force && !term::confirm("Continue?")? {
            println!();
            term::print_info("Nothing was changed.");
            println!();
            return Ok(false);
        }
    }
    for ei in targets {
        workspace::restore_exercise(base_dir, ei)?;
    }
    Ok(true)
}

/// `cmetal reset <name>` — restore one exercise's working copy to the
/// pristine version and mark it pending again, leaving every other
/// exercise's progress untouched. Compiles nothing.
pub fn reset_one(
    base_dir: &Path,
    name: String,
    compiler_kind: CompilerKind,
    force: bool,
) -> Result<()> {
    let info = InfoFile::parse(&base_dir.join("info.toml"))?;
    let mut state = state_without_compiler(base_dir, &info, compiler_kind);
    let idx = state.resolve(Some(name))?;
    let ex_name = state.exercises[idx].name().to_string();
    let target = state.exercises[idx].info.clone();
    if !restore_with_consent(base_dir, &[&target], force)? {
        return Ok(());
    }

    // Un-done it, or watch mode would never offer it again and the
    // progress count would keep claiming it solved.
    state.mark_pending(&ex_name);
    state.save()?;

    println!();
    term::print_success(&format!(
        "{ex_name} restored to the pristine exercise and marked pending again \
         (other progress kept)."
    ));
    println!();
    Ok(())
}

/// `cmetal diff [name]` — pristine exercise vs the learner's working
/// copy. Compiles nothing.
///
/// The two forms differ in what they touch. `diff <name>` reads only
/// `info.toml` and the two files being compared. The no-argument form
/// additionally *reads* the progress file to learn which exercise is
/// current, which means it can also migrate a pre-rename
/// `.clings-state.txt` — unavoidable, since "the exercise I am on" is
/// a fact only the progress file holds.
pub fn diff(base_dir: &Path, name: Option<String>, compiler_kind: CompilerKind) -> Result<()> {
    let info = InfoFile::parse(&base_dir.join("info.toml"))?;

    // A named exercise is resolved against info.toml alone. diff is a
    // diagnostic command: it must keep answering when the progress
    // file is unreadable, and it must not migrate or rewrite that file
    // as a side effect of being asked about a specific exercise. Only
    // the no-argument form needs the state, because "the exercise I am
    // on" is a fact only the state knows.
    let ei = match name {
        Some(name) => info.find(&name)?.clone(),
        None => {
            let state = state_without_compiler(base_dir, &info, compiler_kind);
            let idx = state.resolve(None)?;
            state.exercises[idx].info.clone()
        }
    };
    let name = &ei.name;
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

/// The progress state, built without probing a C compiler.
///
/// The compiler-free commands still need the exercise list and the
/// "current exercise" pointer, so they can share `AppState::resolve`
/// with the rest: one lookup, one error message, and `cmetal diff`
/// with no argument means the same thing as `cmetal run` with no
/// argument. `load_exercises` only needs the compiler's *name* to
/// decide which exercises it could judge — nothing here spawns it.
fn state_without_compiler(
    base_dir: &Path,
    info: &InfoFile,
    compiler_kind: CompilerKind,
) -> AppState {
    let work_dir = workspace::work_dir(base_dir);
    let exercises = workspace::load_exercises(info, base_dir, &work_dir, compiler_kind);
    AppState::new(exercises, base_dir, term::warn_stderr)
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
