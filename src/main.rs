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

/// Extracts the embedded curriculum archive into `dest`.
fn unpack_curriculum(dest: &Path) -> Result<()> {
    let decoder = flate2::read::GzDecoder::new(CURRICULUM_TGZ);
    tar::Archive::new(decoder)
        .unpack(dest)
        .context("Failed to extract the embedded curriculum")
}

/// Stamps `.clings/manifest.json` with this binary's curriculum
/// version. The manifest is what `clings update` reads to decide
/// whether (and in which direction) an update applies.
fn write_manifest(meta_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(meta_dir)?;
    std::fs::write(
        meta_dir.join("manifest.json"),
        format!(
            "{{\n  \"curriculum_version\": \"{}\",\n  \"format_version\": 1\n}}\n",
            env!("CARGO_PKG_VERSION")
        ),
    )?;
    Ok(())
}

/// Extracts the string value of `"key": ...` from the (very small,
/// self-produced) manifest JSON without pulling in a JSON parser.
fn manifest_field<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\":");
    let rest = content[content.find(&pat)? + pat.len()..].trim_start();
    if let Some(stripped) = rest.strip_prefix('"') {
        stripped.split('"').next()
    } else {
        rest.split(|c: char| c == ',' || c == '}' || c.is_whitespace())
            .next()
    }
}

fn semver_triple(v: &str) -> Option<(u64, u64, u64)> {
    let mut it = v.split('.').map(|p| p.parse::<u64>().ok());
    Some((it.next()??, it.next()??, it.next()??))
}

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

    // The guard above proved the target started empty, so on any
    // failure we can remove it wholesale: a half-extracted workspace
    // would otherwise trip the same guard on retry and be orphaned.
    let result = unpack_curriculum(&target).and_then(|()| write_manifest(&target.join(".clings")));
    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&target);
        return Err(e.context(format!(
            "init failed; {} was removed so you can retry",
            target.display()
        )));
    }

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

/// A previous `clings update` interrupted mid-swap leaves complete old
/// parts in `.clings/backup` (each part moves with a single rename, so
/// a part is never half-copied). Restore every part the workspace is
/// missing; where both exist the workspace's version is the complete
/// new part, so it wins. Only then is the backup safe to drop.
fn recover_interrupted_update(base_dir: &Path, backup: &Path) -> Result<()> {
    if !backup.exists() {
        return Ok(());
    }
    term::print_warning("Found leftovers of an interrupted update — recovering.");
    for entry in std::fs::read_dir(backup)? {
        let entry = entry?.path();
        let name = entry.file_name().expect("backup entries have names");
        let dest = base_dir.join(name);
        if !dest.exists() {
            std::fs::rename(&entry, &dest)
                .with_context(|| format!("Failed to restore {} from backup", dest.display()))?;
        }
    }
    std::fs::remove_dir_all(backup)?;
    Ok(())
}

/// Best-effort rollback helpers: a failed rename here leaves the part
/// in backup/staging, where the recovery pass of the next update finds
/// it. Never panics, never masks the original error.
fn rollback_renames(pairs: &[(PathBuf, PathBuf)]) {
    for (from, to) in pairs {
        let _ = std::fs::rename(from, to);
    }
}

fn remove_if_empty(dir: &Path) {
    if let Ok(mut entries) = std::fs::read_dir(dir) {
        if entries.next().is_none() {
            let _ = std::fs::remove_dir(dir);
        }
    }
}

/// Replaces the pristine curriculum with the staged one in two phases
/// of atomic per-part renames: every old part moves to `backup/`, then
/// every staged part moves into place. On failure both phases are
/// rolled back (best effort); whatever a failed rollback leaves behind
/// sits in `backup/` and is picked up by `recover_interrupted_update`
/// on the next run. The part list comes from the staged archive itself,
/// so a directory added to the curriculum later can never be silently
/// dropped by an older hardcoded list.
fn swap_curriculum(base_dir: &Path, staging: &Path, backup: &Path) -> Result<()> {
    let mut parts: Vec<std::ffi::OsString> = std::fs::read_dir(staging)?
        .map(|e| e.map(|e| e.file_name()))
        .collect::<std::io::Result<_>>()?;
    parts.sort();

    std::fs::create_dir_all(backup)?;

    // Phase A: old parts out of the way.
    let mut undo_moves = Vec::new();
    for part in &parts {
        let old = base_dir.join(part);
        if old.exists() {
            if let Err(e) = std::fs::rename(&old, backup.join(part)) {
                rollback_renames(&undo_moves);
                remove_if_empty(backup);
                return Err(e).context(format!("Failed to back up {}", part.to_string_lossy()));
            }
            undo_moves.push((backup.join(part), old));
        }
    }

    // Phase B: staged parts into place.
    let mut undo_installs = Vec::new();
    for part in &parts {
        // Test seam: CLINGS_TEST_FAIL_INSTALL=<part> simulates a rename
        // failure mid-swap (same philosophy as clings_alloc.h).
        let install = if std::env::var_os("CLINGS_TEST_FAIL_INSTALL")
            .is_some_and(|v| v.as_os_str() == part.as_os_str())
        {
            Err(std::io::Error::other("injected failure (test seam)"))
        } else {
            std::fs::rename(staging.join(part), base_dir.join(part))
        };
        if let Err(e) = install {
            rollback_renames(&undo_installs);
            rollback_renames(&undo_moves);
            remove_if_empty(backup);
            return Err(e).context(format!(
                "Failed to install {} — the update was rolled back; if \
                 anything is left in {}, the next `clings update` recovers it",
                part.to_string_lossy(),
                backup.display()
            ));
        }
        undo_installs.push((base_dir.join(part), staging.join(part)));
    }

    // Success: the backup now only holds fully-replaced old parts.
    std::fs::remove_dir_all(backup)?;
    Ok(())
}

/// `clings update` reconciles an init-created workspace with the
/// curriculum embedded in THIS binary (upgrading the binary is how new
/// exercises arrive — no git involved).
///
/// Rules:
///   - my_exercises/ is never overwritten with one exception: a working
///     copy the learner never touched (identical to the OLD pristine
///     file) is refreshed to the new pristine version — and only AFTER
///     the curriculum swap has succeeded;
///   - working copies with edits are kept; if their exercise changed
///     upstream, that is reported (see `clings diff` / `clings reset`);
///   - the pristine curriculum is replaced by `swap_curriculum` (staged,
///     rolled back on failure), and leftovers of a previously
///     interrupted update are recovered before anything else happens.
fn cmd_update(base_dir: &Path) -> Result<()> {
    let meta_dir = base_dir.join(".clings");
    if !meta_dir.join("manifest.json").exists() {
        anyhow::bail!(
            "No .clings/manifest.json here — this looks like a git checkout.\n\
             Update a checkout with `git pull`; `clings update` is for \
             workspaces created by `clings init`."
        );
    }

    // 0. If a previous update was interrupted, make the workspace whole
    //    before touching anything else. Never delete a backup blindly:
    //    it may hold the only copy of parts of the old curriculum.
    let backup = meta_dir.join("backup");
    recover_interrupted_update(base_dir, &backup)?;

    // 1. Version guard: the manifest is the source of truth for what
    //    the workspace holds. Never downgrade, never touch a format we
    //    don't understand, skip the work when already current.
    let manifest = std::fs::read_to_string(meta_dir.join("manifest.json"))?;
    let format = manifest_field(&manifest, "format_version").and_then(|v| v.parse::<u32>().ok());
    if format != Some(1) {
        anyhow::bail!(
            "This workspace uses manifest format {} — this clings only \
             understands format 1. Upgrade clings and retry.",
            format.map_or_else(|| "?".to_string(), |f| f.to_string())
        );
    }
    let bin_version = env!("CARGO_PKG_VERSION");
    let ws_version = manifest_field(&manifest, "curriculum_version").unwrap_or("0.0.0");
    match (semver_triple(ws_version), semver_triple(bin_version)) {
        (Some(ws), Some(bin)) if ws > bin => anyhow::bail!(
            "Workspace curriculum {ws_version} is NEWER than this binary's \
             ({bin_version}) — updating would downgrade it. Upgrade clings instead."
        ),
        (Some(ws), Some(bin)) if ws == bin => {
            println!();
            term::print_info(&format!(
                "Workspace already on curriculum {bin_version}; nothing to update."
            ));
            println!();
            return Ok(());
        }
        _ => {}
    }

    // 2. Stage the embedded curriculum.
    let staging = meta_dir.join("staging");
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir_all(&staging)?;
    unpack_curriculum(&staging)?;

    // 3. Reconciliation SCAN, no writes: decide what to do while the
    //    old pristine files are still in place. The staged info.toml is
    //    the authoritative exercise list, so reported names are the
    //    canonical ones run/diff/reset accept.
    let staged_info = InfoFile::parse(&staging.join("info.toml"))?;
    let mut new_exercises = Vec::new();
    let mut kept = Vec::new();
    let mut pending_refresh: Vec<(PathBuf, Vec<u8>, String)> = Vec::new();
    for ei in &staged_info.exercises {
        let rel = Path::new("exercises").join(ei.rel_path());
        let staged = staging.join(&rel);
        let old_pristine = base_dir.join(&rel);
        if !old_pristine.exists() {
            new_exercises.push(ei.name.clone());
            continue; // appears in my_exercises/ on the next run
        }
        let new_bytes = std::fs::read(&staged)?;
        let old_bytes = std::fs::read(&old_pristine)?;
        if new_bytes == old_bytes {
            continue; // unchanged upstream
        }
        let my_copy = base_dir.join("my_exercises").join(ei.rel_path());
        if !my_copy.exists() {
            continue; // will be copied fresh on the next run
        }
        if std::fs::read(&my_copy)? == old_bytes {
            // Never touched by the learner: refresh after the swap.
            pending_refresh.push((my_copy, new_bytes, ei.name.clone()));
        } else {
            kept.push(ei.name.clone());
        }
    }

    // 4. Swap the pristine curriculum (staged, rolled back on failure).
    swap_curriculum(base_dir, &staging, &backup)?;

    // 5. With the new curriculum fully in place, refresh the untouched
    //    working copies FIRST — this is the only remaining state the
    //    learner could lose, so shrink the window before any cleanup.
    //    (If interrupted exactly here, the copies stay on the old text
    //    and the next update reports them as edited — degraded but
    //    honest, and `clings reset <name>` recovers.)
    let mut refreshed = Vec::new();
    for (my_copy, new_bytes, display_name) in pending_refresh {
        std::fs::write(&my_copy, &new_bytes)?;
        refreshed.push(display_name);
    }
    std::fs::remove_dir_all(&staging)?;

    // 6. Stamp the manifest with this binary's curriculum version.
    write_manifest(&meta_dir)?;

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
        term::print_info("Curriculum files updated; no working-copy changes to reconcile.");
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

/// Like `resolve_base_dir`, but for `clings update`: a workspace whose
/// previous update was interrupted mid-swap may transiently have NO
/// info.toml (it sits in .clings/backup), so the workspace metadata
/// directory is accepted as a marker too — otherwise the recovery pass
/// could never run for exactly the crash it exists to repair.
fn resolve_workspace_dir() -> Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join("info.toml").exists() || dir.join(".clings").join("manifest.json").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            break;
        }
    }
    anyhow::bail!("Could not find a clings workspace here (no info.toml or .clings/manifest.json).")
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
        let rel = ei.rel_path();
        let src = pristine_dir.join(&rel);
        let dst = work_dir.join(&rel);
        if src.exists() && !dst.exists() {
            std::fs::create_dir_all(dst.parent().unwrap())?;
            std::fs::copy(&src, &dst)
                .with_context(|| format!("Failed to copy {} into the workspace", src.display()))?;
        }
    }
    Ok(work_dir)
}

/// Copies one exercise's pristine file over its working copy. Missing
/// pristine files are tolerated (same semantics as a full restore).
fn restore_exercise(base_dir: &Path, ei: &info_file::ExerciseInfo) -> Result<()> {
    let rel = ei.rel_path();
    let src = base_dir.join("exercises").join(&rel);
    let dst = base_dir.join("my_exercises").join(&rel);
    if src.exists() {
        std::fs::create_dir_all(dst.parent().unwrap())?;
        std::fs::copy(&src, &dst)
            .with_context(|| format!("Failed to restore {}", rel.display()))?;
    }
    Ok(())
}

/// Overwrite every workspace file with its pristine version from `exercises/`.
fn restore_workspace(info: &InfoFile, base_dir: &Path) -> Result<()> {
    for ei in &info.exercises {
        restore_exercise(base_dir, ei)?;
    }
    Ok(())
}

/// Looks up an exercise's metadata by name.
fn find_exercise_info<'a>(info: &'a InfoFile, name: &str) -> Result<&'a info_file::ExerciseInfo> {
    info.exercises
        .iter()
        .find(|ei| ei.name == name)
        .with_context(|| format!("Exercise '{name}' not found"))
}

/// Writes to stdout, tolerating ONLY a closed pipe (`clings diff x |
/// head`); any other write failure is a real error and propagates.
fn write_stdout(text: &str) -> Result<()> {
    use std::io::Write;
    match std::io::stdout().write_all(text.as_bytes()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(e).context("Failed to write to stdout"),
    }
}

/// `clings diff <name>`: pristine exercise vs the learner's working
/// copy. Pure file operation — needs no compiler, no state.
fn cmd_diff(base_dir: &Path, name: &str) -> Result<()> {
    let info = InfoFile::parse(&base_dir.join("info.toml"))?;
    let ei = find_exercise_info(&info, name)?;
    let rel = ei.rel_path();
    let pristine = std::fs::read_to_string(base_dir.join("exercises").join(&rel))
        .with_context(|| format!("Failed to read pristine {}", rel.display()))?;
    let my_path = base_dir.join("my_exercises").join(&rel);
    if !my_path.exists() {
        write_stdout(&format!(
            "\n{name}: no working copy yet — it is created on the next `clings` run.\n\n"
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

/// `clings reset <name>`: restore one exercise's working copy to the
/// pristine version and mark it pending again, leaving every other
/// exercise's progress untouched. Needs no compiler.
fn cmd_reset_one(base_dir: &Path, name: &str, compiler_kind: CompilerKind) -> Result<()> {
    let info = InfoFile::parse(&base_dir.join("info.toml"))?;
    let ei = find_exercise_info(&info, name)?;
    restore_exercise(base_dir, ei)?;

    // Un-done it, or watch mode would never offer it again and the
    // progress count would keep claiming it solved.
    let work_dir = base_dir.join("my_exercises");
    let exercises = load_exercises(&info, base_dir, &work_dir, compiler_kind);
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
        Some(Commands::Init { dir }) => return cmd_init(dir.clone()),
        // Reconciles files; must run even in a workspace whose
        // info.toml sits in .clings/backup after an interrupted update,
        // which is why it resolves the workspace by its own marker.
        Some(Commands::Update) => {
            let base_dir = resolve_workspace_dir()?;
            return cmd_update(&base_dir);
        }
        Some(Commands::Diff { name }) => {
            let base_dir = resolve_base_dir()?;
            return cmd_diff(&base_dir, name);
        }
        Some(Commands::Reset { name: Some(name) }) => {
            let base_dir = resolve_base_dir()?;
            return cmd_reset_one(&base_dir, name, compiler_kind);
        }
        _ => {}
    }

    let base_dir = resolve_base_dir()
        .context("Could not find clings project. Make sure you're inside the clings directory.")?;

    let info = InfoFile::parse(&base_dir.join("info.toml"))?;

    let compiler = Compiler::new(compiler_kind, &base_dir)?;
    let work_dir = prepare_workspace(&info, &base_dir)?;
    let exercises = load_exercises(&info, &base_dir, &work_dir, compiler_kind);
    let build_dir = base_dir.join("target").join("clings");
    let mut state = AppState::new(exercises, &base_dir)?;

    match cli.command {
        // Dispatched before engine setup; see the match at the top of main.
        Some(Commands::Init { .. })
        | Some(Commands::Update)
        | Some(Commands::Diff { .. })
        | Some(Commands::Reset { name: Some(_) }) => {
            unreachable!("file-only commands are dispatched before engine setup")
        }
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
                state.complete(&name)?;
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
                state.complete(&name)?;
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
        Some(Commands::Reset { name: None }) => {
            state.reset()?;
            restore_workspace(&info, &base_dir)?;
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
