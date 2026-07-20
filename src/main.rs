mod app_state;
mod commands;
mod compiler;
mod errors;
mod exercise;
mod info_file;
mod runner;
mod solutions;
mod term;
mod view;
mod watch;
mod workspace;

use anyhow::{Context, Result};
use app_state::AppState;
use clap::{Parser, Subcommand};
use compiler::{Compiler, CompilerKind};
use info_file::InfoFile;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "cmetal",
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
    /// Create a self-contained cmetal workspace (no git clone needed)
    Init {
        /// Directory to create the workspace in (default: ./cmetal-workspace)
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
        /// Exercise name (defaults to current)
        name: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let compiler_kind = match cli.compiler.to_lowercase().as_str() {
        "gcc" => CompilerKind::Gcc,
        "clang" => CompilerKind::Clang,
        other => anyhow::bail!("Unknown compiler: {other}. Use 'gcc' or 'clang'."),
    };

    // File-only commands are dispatched before the engine spins up:
    // they must not require a C compiler, a populated workspace — or,
    // for init, any project at all. Their match arms below are
    // unreachable by construction.
    match &cli.command {
        // Creates a workspace: works OUTSIDE any existing project.
        Some(Commands::Init { dir }) => return workspace::init(dir.clone()),
        // Reconciles files; must run even in a workspace whose
        // info.toml sits in .cmetal/backup after an interrupted update,
        // which is why it resolves the workspace by its own marker.
        Some(Commands::Update) => {
            let base_dir = workspace::resolve_workspace_dir()?;
            return workspace::update(&base_dir);
        }
        Some(Commands::Diff { name }) => {
            let base_dir = workspace::resolve_base_dir()?;
            return commands::diff(&base_dir, name.clone(), compiler_kind);
        }
        Some(Commands::Reset { name: Some(name) }) => {
            let base_dir = workspace::resolve_base_dir()?;
            return commands::reset_one(&base_dir, name.clone(), compiler_kind);
        }
        _ => {}
    }

    // Everything below needs the exercise list, the materialized
    // workspace and the learner's progress. Materializing is idempotent
    // and is what every engine command has always done on startup —
    // `cmetal list` is how a learner first gets their working copies.
    let base_dir = workspace::resolve_base_dir()
        .context("Could not find cmetal project. Make sure you're inside the cmetal directory.")?;

    let info = InfoFile::parse(&base_dir.join("info.toml"))?;

    let work_dir = workspace::prepare_workspace(&info, &base_dir)?;
    let exercises = workspace::load_exercises(&info, &base_dir, &work_dir, compiler_kind);
    let build_dir = base_dir.join("target").join("cmetal");
    let mut state = AppState::new(exercises, &base_dir, term::warn_stderr);

    // The C toolchain is NOT part of that: `list`, `hint` and `reset`
    // compile nothing, and probing for a compiler they will never use
    // would lock out a machine that has none. Only the arms below that
    // actually compile call this. Which exercise supports which
    // compiler is decided from `compiler_kind` alone.
    let compiler = || Compiler::new(compiler_kind, &base_dir);

    match cli.command {
        // Dispatched before engine setup; see the match at the top of main.
        Some(Commands::Init { .. })
        | Some(Commands::Update)
        | Some(Commands::Diff { .. })
        | Some(Commands::Reset { name: Some(_) }) => {
            unreachable!("file-only commands are dispatched before engine setup")
        }
        None => watch::run_watch(
            &mut state,
            &compiler()?,
            &work_dir,
            &build_dir,
            info.welcome_message.as_deref(),
        )?,
        Some(Commands::Run { name }) => commands::run(&mut state, &compiler()?, &build_dir, name)?,
        Some(Commands::Hint { name, level }) => commands::hint(&state, name, level)?,
        Some(Commands::Solution { name }) => {
            commands::solution(&mut state, &compiler()?, &build_dir, name)?
        }
        Some(Commands::List) => commands::list(&state),
        Some(Commands::Verify) => commands::verify(&mut state, &compiler()?, &build_dir)?,
        Some(Commands::Reset { name: None }) => commands::reset_all(&state, &info, &base_dir)?,
    }

    Ok(())
}
