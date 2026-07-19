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
    /// Reset progress (start from scratch)
    Reset,
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
        Some(Commands::Reset) => {
            state.reset()?;
            restore_workspace(&info, &base_dir, &work_dir)?;
            println!();
            term::print_success("Progress reset. Workspace restored to pristine exercises!");
            println!();
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
