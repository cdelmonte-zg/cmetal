mod app_state;
mod compiler;
mod exercise;
mod info_file;
mod solutions;
mod term;
mod watch;

use anyhow::{Context, Result};
use app_state::AppState;
use clap::{Parser, Subcommand};
use compiler::{Compiler, CompilerKind};
use exercise::Exercise;
use info_file::InfoFile;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "clings",
    version,
    about = "Small exercises to learn advanced C concepts"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Compiler to use (gcc or clang)
    #[arg(long, global = true, default_value = "gcc")]
    compiler: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a self-contained clings workspace (no git clone needed)
    Init {
        /// Directory to create the workspace in (default: ./clings-workspace)
        dir: Option<PathBuf>,
    },
    /// Run a specific exercise
    Run {
        /// Exercise name
        name: Option<String>,
    },
    /// Show hint for an exercise (use --level N to reveal up to hint N)
    Hint {
        /// Exercise name (defaults to current)
        name: Option<String>,
        /// Hint level to show (1 = first hint, 2 = first two, etc.)
        #[arg(short, long, default_value = "1")]
        level: usize,
    },
    /// Reveal the official solution (only after the exercise passes)
    Solution {
        /// Exercise name (defaults to current)
        name: Option<String>,
    },
    /// List all exercises and their status
    List,
    /// Verify all exercises
    Verify,
    /// Reset progress, or a single exercise's working copy
    Reset {
        /// Exercise name: restore only its working copy (progress kept)
        name: Option<String>,
    },
    /// Update the workspace to the curriculum embedded in this binary
    Update,
    /// Show how your working copy differs from the pristine exercise
    Diff {
        /// Exercise name
        name: String,
    },
}

/// Curriculum archive produced by build.rs from exercises/, solutions/
/// (obfuscated), include/ and info.toml: the binary carries its own
/// course, so a workspace can be materialized without the git clone.
static CURRICULUM_TGZ: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/curriculum.tar.gz"));

/// `clings init` extracts the embedded curriculum into a directory.
/// The layout is exactly the repo layout the engine already understands
/// (info.toml at the root), so a workspace and a git clone are
/// interchangeable — the clone remains the contributor/compat mode.
fn cmd_init(dir: Option<PathBuf>) -> Result<()> {
    let target = dir.unwrap_or_else(|| PathBuf::from("clings-workspace"));
    // Never extract into a directory that already has content: the
    // archive contains entries named exercises/, solutions/, include/
    // and info.toml, and unpacking must not be able to clobber a
    // user's files (think `clings init .` in a project directory).
    // No --force escape hatch until an explicit update semantics
    // exists.
    if target.exists() {
        let mut entries = std::fs::read_dir(&target)
            .with_context(|| format!("Failed to inspect {}", target.display()))?;
        if entries.next().transpose()?.is_some() {
            anyhow::bail!("{} already exists and is not empty", target.display());
        }
    } else {
        std::fs::create_dir_all(&target)
            .with_context(|| format!("Failed to create {}", target.display()))?;
    }

    let decoder = flate2::read::GzDecoder::new(CURRICULUM_TGZ);
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(&target)
        .context("Failed to extract the embedded curriculum")?;

    // Record which curriculum this workspace was materialized from —
    // a future `clings update` will use this to reconcile versions.
    let meta_dir = target.join(".clings");
    std::fs::create_dir_all(&meta_dir)?;
    std::fs::write(
        meta_dir.join("manifest.json"),
        format!(
            "{{\n  \"curriculum_version\": \"{}\",\n  \"format_version\": 1\n}}\n",
            env!("CARGO_PKG_VERSION")
        ),
    )?;

    println!();
    term::print_success(&format!("Workspace created in {}", target.display()));
    println!();
    println!("  Next steps:");
    println!("    cd {}", target.display());
    println!("    clings");
    println!();
    println!("  clings copies the exercises into my_exercises/ on first run —");
    println!("  that's where you work. Your progress lives in this directory.");
    println!();
    Ok(())
}

/// Collects the relative paths of all files under `dir` (recursively).
fn relative_files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            relative_files(root, &path, out)?;
        } else {
            out.push(path.strip_prefix(root).unwrap().to_path_buf());
        }
    }
    Ok(())
}

/// `clings update` reconciles an init-created workspace with the
/// curriculum embedded in THIS binary (upgrading the binary is how new
/// exercises arrive — no git involved).
///
/// Rules:
///   - my_exercises/ is never overwritten with one exception: a working
///     copy the learner never touched (identical to the OLD pristine
///     file) is refreshed to the new pristine version;
///   - working copies with edits are kept; if their exercise changed
///     upstream, that is reported (see `clings diff` / `clings reset`);
///   - the pristine curriculum (exercises/, solutions/, include/,
///     info.toml) is replaced via a staging directory and renames, so a
///     failed update does not leave a half-written curriculum.
fn cmd_update(base_dir: &Path) -> Result<()> {
    let meta_dir = base_dir.join(".clings");
    if !meta_dir.join("manifest.json").exists() {
        anyhow::bail!(
            "No .clings/manifest.json here — this looks like a git checkout.\n\
             Update a checkout with `git pull`; `clings update` is for \
             workspaces created by `clings init`."
        );
    }

    // 1. Stage the embedded curriculum.
    let staging = meta_dir.join("staging");
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir_all(&staging)?;
    let decoder = flate2::read::GzDecoder::new(CURRICULUM_TGZ);
    tar::Archive::new(decoder)
        .unpack(&staging)
        .context("Failed to extract the embedded curriculum")?;

    // 2. Reconcile working copies while the OLD pristine files are
    //    still in place.
    let mut new_exercises = Vec::new();
    let mut refreshed = Vec::new();
    let mut kept = Vec::new();
    let mut staged_exercise_files = Vec::new();
    relative_files(
        &staging,
        &staging.join("exercises"),
        &mut staged_exercise_files,
    )?;
    for rel in &staged_exercise_files {
        let display_name = rel
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let staged = staging.join(rel);
        let old_pristine = base_dir.join(rel);
        if !old_pristine.exists() {
            new_exercises.push(display_name);
            continue; // appears in my_exercises/ on the next run
        }
        let new_bytes = std::fs::read(&staged)?;
        let old_bytes = std::fs::read(&old_pristine)?;
        if new_bytes == old_bytes {
            continue; // unchanged upstream
        }
        let my_copy = base_dir
            .join("my_exercises")
            .join(rel.strip_prefix("exercises").unwrap());
        if !my_copy.exists() {
            continue; // will be copied fresh on the next run
        }
        if std::fs::read(&my_copy)? == old_bytes {
            // Never touched by the learner: safe to refresh.
            std::fs::write(&my_copy, &new_bytes)?;
            refreshed.push(display_name);
        } else {
            kept.push(display_name);
        }
    }

    // 3. Swap the pristine curriculum: old parts move to a backup dir,
    //    staged parts move into place. Renames within the workspace
    //    keep this close to atomic; on failure the backup remains.
    let backup = meta_dir.join("backup");
    if backup.exists() {
        std::fs::remove_dir_all(&backup)?;
    }
    std::fs::create_dir_all(&backup)?;
    for part in ["exercises", "solutions", "include", "info.toml"] {
        let old = base_dir.join(part);
        if old.exists() {
            std::fs::rename(&old, backup.join(part))
                .with_context(|| format!("Failed to back up {part}"))?;
        }
        std::fs::rename(staging.join(part), &old).with_context(|| {
            format!(
                "Failed to install new {part} — the previous curriculum \
                 is preserved in {}",
                backup.display()
            )
        })?;
    }
    std::fs::remove_dir_all(&backup)?;
    std::fs::remove_dir_all(&staging)?;

    // 4. Stamp the manifest with this binary's curriculum version.
    std::fs::write(
        meta_dir.join("manifest.json"),
        format!(
            "{{\n  \"curriculum_version\": \"{}\",\n  \"format_version\": 1\n}}\n",
            env!("CARGO_PKG_VERSION")
        ),
    )?;

    println!();
    term::print_success(&format!(
        "Workspace updated to curriculum {}.",
        env!("CARGO_PKG_VERSION")
    ));
    if !new_exercises.is_empty() {
        term::print_info(&format!(
            "{} new exercise(s) — they appear in my_exercises/ on the next run: {}",
            new_exercises.len(),
            new_exercises.join(", ")
        ));
    }
    if !refreshed.is_empty() {
        term::print_info(&format!(
            "{} untouched working cop(ies) refreshed: {}",
            refreshed.len(),
            refreshed.join(", ")
        ));
    }
    for name in &kept {
        term::print_warning(&format!(
            "{name} changed upstream but you have edits — your copy is kept. \
             Compare with `clings diff {name}`, or take the new version with \
             `clings reset {name}`."
        ));
    }
    if new_exercises.is_empty() && refreshed.is_empty() && kept.is_empty() {
        term::print_info("Curriculum already up to date; nothing to reconcile.");
    }
    println!();
    Ok(())
}

fn resolve_base_dir() -> Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join("info.toml").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            break;
        }
    }
    anyhow::bail!("Could not find info.toml. Are you in a clings project directory?")
}

fn load_exercises(
    info: &InfoFile,
    base_dir: &Path,
    work_dir: &Path,
    compiler: CompilerKind,
) -> Vec<Exercise> {
    let solutions_dir = base_dir.join("solutions");
    info.exercises
        .iter()
        .map(|ei| {
            Exercise::new(
                ei.clone(),
                work_dir,
                &solutions_dir,
                compiler.command_name(),
            )
        })
        .collect()
}

/// Learners work on copies in `my_exercises/` (gitignored), so the pristine
/// files in `exercises/` are never modified and can't be pushed in their
/// solved state. Copies any exercise that is not yet in the workspace,
/// leaving files the learner already edited untouched.
fn prepare_workspace(info: &InfoFile, base_dir: &Path) -> Result<PathBuf> {
    let pristine_dir = base_dir.join("exercises");
    let work_dir = base_dir.join("my_exercises");
    for ei in &info.exercises {
        let file = format!("{}.c", ei.name);
        let src = pristine_dir.join(&ei.dir).join(&file);
        let dst = work_dir.join(&ei.dir).join(&file);
        if src.exists() && !dst.exists() {
            std::fs::create_dir_all(dst.parent().unwrap())?;
            std::fs::copy(&src, &dst)
                .with_context(|| format!("Failed to copy {} into the workspace", src.display()))?;
        }
    }
    Ok(work_dir)
}

/// Overwrite every workspace file with its pristine version from `exercises/`.
fn restore_workspace(info: &InfoFile, base_dir: &Path, work_dir: &Path) -> Result<()> {
    let pristine_dir = base_dir.join("exercises");
    for ei in &info.exercises {
        let file = format!("{}.c", ei.name);
        let src = pristine_dir.join(&ei.dir).join(&file);
        let dst = work_dir.join(&ei.dir).join(&file);
        if src.exists() {
            std::fs::create_dir_all(dst.parent().unwrap())?;
            std::fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // `init` creates a workspace, so it must work OUTSIDE any existing
    // project: handle it before base-dir resolution.
    if let Some(Commands::Init { dir }) = &cli.command {
        return cmd_init(dir.clone());
    }

    let base_dir = resolve_base_dir()
        .context("Could not find clings project. Make sure you're inside the clings directory.")?;

    // `update` only reconciles files: no compiler, no exercise state.
    if let Some(Commands::Update) = &cli.command {
        return cmd_update(&base_dir);
    }

    let info = InfoFile::parse(&base_dir.join("info.toml"))?;

    let compiler_kind = match cli.compiler.to_lowercase().as_str() {
        "gcc" => CompilerKind::Gcc,
        "clang" => CompilerKind::Clang,
        other => anyhow::bail!("Unknown compiler: {other}. Use 'gcc' or 'clang'."),
    };

    let compiler = Compiler::new(compiler_kind, &base_dir)?;
    let work_dir = prepare_workspace(&info, &base_dir)?;
    let exercises = load_exercises(&info, &base_dir, &work_dir, compiler_kind);
    let build_dir = base_dir.join("target").join("clings");
    let mut state = AppState::new(exercises, &base_dir)?;

    match cli.command {
        // Handled before base-dir resolution above.
        Some(Commands::Init { .. }) => unreachable!("init is dispatched early"),
        None => {
            // Default: watch mode
            let welcome = info.welcome_message.as_deref();
            watch::run_watch(&mut state, &compiler, &work_dir, &build_dir, welcome)?;
        }
        Some(Commands::Run { name }) => {
            let name = name.unwrap_or_else(|| {
                state
                    .current_exercise()
                    .map(|e| e.name().to_string())
                    .unwrap_or_default()
            });

            let idx = state
                .find_exercise(&name)
                .context(format!("Exercise '{name}' not found"))?;

            let exercise = &state.exercises[idx];
            println!();
            term::print_header(&format!("Running: {}", exercise.name()));
            println!();

            if !exercise.supported {
                term::print_warning(&format!(
                    "{} requires {} (current compiler: {}). Re-run with --compiler.",
                    exercise.name(),
                    exercise.required_compilers(),
                    compiler.kind()
                ));
                println!();
                return Ok(());
            }

            let result = exercise.verify(&compiler, &build_dir)?;
            if result.success {
                term::print_success(&format!("{} passed!", exercise.name()));
                if !result.output.is_empty() {
                    term::print_stage_output("Output", &result.output);
                }
                if let Ok(path) = exercise.reveal_solution() {
                    term::print_info(&format!("Official solution revealed: {}", path.display()));
                }
                state.mark_done(&name);
                state.save()?;
            } else {
                term::print_error(&format!(
                    "{} failed at stage: {}",
                    exercise.name(),
                    result.stage
                ));
                term::print_stage_output(result.stage, &result.output);
                std::process::exit(1);
            }
        }
        Some(Commands::Hint { name, level }) => {
            let name = name.unwrap_or_else(|| {
                state
                    .current_exercise()
                    .map(|e| e.name().to_string())
                    .unwrap_or_default()
            });

            let idx = state
                .find_exercise(&name)
                .context(format!("Exercise '{name}' not found"))?;

            let exercise = &state.exercises[idx];
            let hints = exercise.hints();

            println!();
            if hints.is_empty() {
                term::print_warning(&format!("No hints available for {}.", exercise.name()));
            } else {
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
            }
            println!();
        }
        Some(Commands::Solution { name }) => {
            let name = name.unwrap_or_else(|| {
                state
                    .current_exercise()
                    .map(|e| e.name().to_string())
                    .unwrap_or_default()
            });

            let idx = state
                .find_exercise(&name)
                .context(format!("Exercise '{name}' not found"))?;

            let exercise = &state.exercises[idx];
            // For exercises that don't support the current compiler a verify
            // pass would be meaningless (the bug may not even be detectable),
            // so only completed ones unlock.
            let already_done = state.is_done(&name);
            let verified_now = !already_done
                && exercise.supported
                && exercise.verify(&compiler, &build_dir)?.success;

            println!();
            if !already_done && !verified_now {
                term::print_warning(&format!(
                    "{name} is not solved yet. Fix it first — then the solution unlocks!"
                ));
                println!();
                std::process::exit(1);
            }

            let path = exercise.reveal_solution()?;
            term::print_success(&format!("Solution for {name}: {}", path.display()));
            println!();

            // The verify pass that just unlocked the solution is a completion
            // like any other: persist it, or `clings list` keeps showing the
            // exercise as pending.
            if verified_now {
                state.mark_done(&name);
                state.save()?;
            }
        }
        Some(Commands::List) => {
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
                let status = if state.is_done(exercise.name()) {
                    "✓"
                } else if !exercise.supported {
                    "−"
                } else if i == state.current_index {
                    "→"
                } else {
                    " "
                };
                let note = if exercise.supported {
                    String::new()
                } else {
                    format!("  (requires {})", exercise.required_compilers())
                };
                println!("    {status} {}{note}", exercise.name());
            }
            println!();
        }
        Some(Commands::Reset { name: Some(name) }) => {
            let idx = state
                .find_exercise(&name)
                .context(format!("Exercise '{name}' not found"))?;
            let exercise = &state.exercises[idx];
            let rel = Path::new(&exercise.info.dir).join(format!("{}.c", exercise.info.name));
            let src = base_dir.join("exercises").join(&rel);
            let dst = work_dir.join(&rel);
            std::fs::create_dir_all(dst.parent().unwrap())?;
            std::fs::copy(&src, &dst)
                .with_context(|| format!("Failed to restore {}", rel.display()))?;
            println!();
            term::print_success(&format!(
                "{name} restored to the pristine exercise (progress kept)."
            ));
            println!();
        }
        Some(Commands::Reset { name: None }) => {
            state.reset()?;
            restore_workspace(&info, &base_dir, &work_dir)?;
            println!();
            term::print_success("Progress reset. Workspace restored to pristine exercises!");
            println!();
        }
        // Handled before compiler setup above.
        Some(Commands::Update) => unreachable!("update is dispatched early"),
        Some(Commands::Diff { name }) => {
            let idx = state
                .find_exercise(&name)
                .context(format!("Exercise '{name}' not found"))?;
            let exercise = &state.exercises[idx];
            let rel = Path::new(&exercise.info.dir).join(format!("{}.c", exercise.info.name));
            let pristine = std::fs::read_to_string(base_dir.join("exercises").join(&rel))
                .with_context(|| format!("Failed to read pristine {}", rel.display()))?;
            let mine = std::fs::read_to_string(work_dir.join(&rel))
                .with_context(|| format!("Failed to read your copy of {}", rel.display()))?;
            println!();
            if pristine == mine {
                term::print_info(&format!(
                    "{name}: your copy is identical to the pristine exercise."
                ));
            } else {
                let diff = similar::TextDiff::from_lines(&pristine, &mine);
                let text = format!(
                    "{}",
                    diff.unified_diff().context_radius(3).header(
                        &format!("exercises/{} (pristine)", rel.display()),
                        &format!("my_exercises/{} (yours)", rel.display()),
                    )
                );
                // Tolerate a closed pipe (`clings diff x | head`):
                // println! would panic on the write error.
                use std::io::Write;
                let _ = std::io::stdout().write_all(text.as_bytes());
            }
            {
                use std::io::Write;
                let _ = writeln!(std::io::stdout());
            }
        }
        Some(Commands::Verify) => {
            println!();
            term::print_header("Verifying all exercises...");
            println!();

            let mut all_passed = true;
            let mut passed_names = Vec::new();
            for exercise in &state.exercises {
                if !exercise.supported {
                    term::print_warning(&format!(
                        "{}: skipped (requires {})",
                        exercise.name(),
                        exercise.required_compilers()
                    ));
                    continue;
                }
                if !exercise.exists() {
                    term::print_warning(&format!("{}: file not found", exercise.name()));
                    continue;
                }
                match exercise.verify(&compiler, &build_dir) {
                    Ok(result) => {
                        if result.success {
                            term::print_success(exercise.name());
                            passed_names.push(exercise.name().to_string());
                        } else {
                            term::print_error(&format!(
                                "{} (failed at: {})",
                                exercise.name(),
                                result.stage
                            ));
                            all_passed = false;
                        }
                    }
                    Err(e) => {
                        term::print_error(&format!("{}: {e}", exercise.name()));
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
        }
    }

    Ok(())
}
