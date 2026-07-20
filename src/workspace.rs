//! The workspace as a *place on disk*: materializing it from the
//! curriculum embedded in this binary, reconciling it with a newer
//! curriculum, and locating or repairing it.
//!
//! Nothing here compiles or runs C, and nothing here is a subcommand —
//! those live in [`crate::commands`] and call into this module. The
//! two exceptions, `init` and `update`, are here because they *are*
//! the curriculum machinery rather than users of it.

use crate::compiler::CompilerKind;
use crate::exercise::Exercise;
use crate::info_file::{ExerciseInfo, InfoFile};
use crate::term;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

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

/// Stamps `.cmetal/manifest.json` with this binary's curriculum
/// version. The manifest is what `cmetal update` reads to decide
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

/// `cmetal init` extracts the embedded curriculum into a directory.
/// The layout is exactly the repo layout the engine already understands
/// (info.toml at the root), so a workspace and a git clone are
/// interchangeable — the clone remains the contributor/compat mode.
pub fn init(dir: Option<PathBuf>) -> Result<()> {
    let target = dir.unwrap_or_else(|| PathBuf::from("cmetal-workspace"));
    // Never extract into a directory that already has content: the
    // archive contains entries named exercises/, solutions/, include/
    // and info.toml, and unpacking must not be able to clobber a
    // user's files (think `cmetal init .` in a project directory).
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
    let result = unpack_curriculum(&target).and_then(|()| write_manifest(&target.join(".cmetal")));
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
    println!("    cmetal");
    println!();
    println!("  cmetal copies the exercises into my_exercises/ on first run —");
    println!("  that's where you work. Your progress lives in this directory.");
    println!();
    Ok(())
}

/// A previous `cmetal update` interrupted mid-swap leaves complete old
/// parts in `.cmetal/backup` (each part moves with a single rename, so
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
        // Test seam: CMETAL_TEST_FAIL_INSTALL=<part> simulates a rename
        // failure mid-swap (same philosophy as cmetal_alloc.h).
        let install = if std::env::var_os("CMETAL_TEST_FAIL_INSTALL")
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
                 anything is left in {}, the next `cmetal update` recovers it",
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

/// `cmetal update` reconciles an init-created workspace with the
/// curriculum embedded in THIS binary (upgrading the binary is how new
/// exercises arrive — no git involved).
///
/// Rules:
///   - my_exercises/ is never overwritten with one exception: a working
///     copy the learner never touched (identical to the OLD pristine
///     file) is refreshed to the new pristine version — and only AFTER
///     the curriculum swap has succeeded;
///   - working copies with edits are kept; if their exercise changed
///     upstream, that is reported (see `cmetal diff` / `cmetal reset`);
///   - the pristine curriculum is replaced by `swap_curriculum` (staged,
///     rolled back on failure), and leftovers of a previously
///     interrupted update are recovered before anything else happens.
pub fn update(base_dir: &Path) -> Result<()> {
    let meta_dir = base_dir.join(".cmetal");
    // Pre-rename workspaces carry their metadata in .clings: adopt it
    // before anything else looks for it.
    let legacy_meta = base_dir.join(".clings");
    if !meta_dir.exists() && legacy_meta.join("manifest.json").exists() {
        std::fs::rename(&legacy_meta, &meta_dir)
            .context("Failed to migrate the pre-rename .clings directory")?;
        term::print_info("Migrated workspace metadata from .clings/ to .cmetal/.");
    }
    if !meta_dir.join("manifest.json").exists() {
        anyhow::bail!(
            "No .cmetal/manifest.json here — this looks like a git checkout.\n\
             Update a checkout with `git pull`; `cmetal update` is for \
             workspaces created by `cmetal init`."
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
            "This workspace uses manifest format {} — this cmetal only \
             understands format 1. Upgrade cmetal and retry.",
            format.map_or_else(|| "?".to_string(), |f| f.to_string())
        );
    }
    let bin_version = env!("CARGO_PKG_VERSION");
    let ws_version = manifest_field(&manifest, "curriculum_version").unwrap_or("0.0.0");
    match (semver_triple(ws_version), semver_triple(bin_version)) {
        (Some(ws), Some(bin)) if ws > bin => anyhow::bail!(
            "Workspace curriculum {ws_version} is NEWER than this binary's \
             ({bin_version}) — updating would downgrade it. Upgrade cmetal instead."
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
    //    honest, and `cmetal reset <name>` recovers.)
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
             Compare with `cmetal diff {name}`, or take the new version with \
             `cmetal reset {name}`."
        ));
    }
    if new_exercises.is_empty() && refreshed.is_empty() && kept.is_empty() {
        term::print_info("Curriculum files updated; no working-copy changes to reconcile.");
    }
    println!();
    Ok(())
}

/// Walks up from the working directory to the first ancestor matching
/// `marker`. The two resolvers below differ only in what counts as a
/// marker.
///
/// The two failures are kept apart: `Err` means the walk could not
/// start (an unreadable or deleted working directory), `Ok(None)` that
/// it ran and found nothing. Collapsing them would answer "you are not
/// in a cmetal project" to a question the process could not even ask.
fn find_upwards(marker: impl Fn(&Path) -> bool) -> Result<Option<PathBuf>> {
    let mut dir = std::env::current_dir().context("Failed to read the working directory")?;
    loop {
        if marker(&dir) {
            return Ok(Some(dir));
        }
        if !dir.pop() {
            return Ok(None);
        }
    }
}

pub fn resolve_base_dir() -> Result<PathBuf> {
    find_upwards(|dir| dir.join("info.toml").exists())?
        .context("Could not find info.toml. Are you in a cmetal project directory?")
}

/// Like `resolve_base_dir`, but for `cmetal update`: a workspace whose
/// previous update was interrupted mid-swap may transiently have NO
/// info.toml (it sits in .cmetal/backup), so the workspace metadata
/// directory is accepted as a marker too — otherwise the recovery pass
/// could never run for exactly the crash it exists to repair.
/// (.clings is the pre-rename metadata directory; update migrates it.)
pub fn resolve_workspace_dir() -> Result<PathBuf> {
    find_upwards(|dir| {
        dir.join("info.toml").exists()
            || dir.join(".cmetal").join("manifest.json").exists()
            || dir.join(".clings").join("manifest.json").exists()
    })?
    .context("Could not find a cmetal workspace here (no info.toml or .cmetal/manifest.json).")
}

pub fn load_exercises(
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

/// Where the learner's working copies live. A pure path: knowing it
/// must not require creating it, so commands that never touch the
/// files can still name them.
pub fn work_dir(base_dir: &Path) -> PathBuf {
    base_dir.join("my_exercises")
}

/// Learners work on copies in `my_exercises/` (gitignored), so the pristine
/// files in `exercises/` are never modified and can't be pushed in their
/// solved state. Copies any exercise that is not yet in the workspace,
/// leaving files the learner already edited untouched.
pub fn prepare_workspace(info: &InfoFile, base_dir: &Path) -> Result<PathBuf> {
    let pristine_dir = base_dir.join("exercises");
    let work_dir = work_dir(base_dir);
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
pub fn restore_exercise(base_dir: &Path, ei: &ExerciseInfo) -> Result<()> {
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
pub fn restore_workspace(info: &InfoFile, base_dir: &Path) -> Result<()> {
    for ei in &info.exercises {
        restore_exercise(base_dir, ei)?;
    }
    Ok(())
}
