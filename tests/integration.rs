use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Check if gcc is available on this system.
fn has_gcc() -> bool {
    Command::new("gcc")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// A PATH guaranteed to contain no compiler.
///
/// An empty PATH string is not enough: POSIX leaves the lookup
/// undefined when PATH is set but empty, and glibc treats the empty
/// entry as the current directory. A real empty directory is
/// unambiguous everywhere — and it lives in its own TempDir, so the
/// fixture project stays exactly as a real workspace would be.
fn no_toolchain() -> TempDir {
    TempDir::new().unwrap()
}

/// Path to the cmetal binary built for THIS test run.
///
/// Cargo sets CARGO_BIN_EXE_<name> for integration tests precisely so
/// the test cannot end up invoking a stale or foreign binary; deriving
/// the path from current_exe() by hand also broke under --target and a
/// custom CARGO_TARGET_DIR.
fn cmetal_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_cmetal"))
}

/// Overwrite a workspace manifest with a given curriculum version.
fn stamp_manifest(ws: &Path, version: &str) {
    std::fs::write(
        ws.join(".cmetal/manifest.json"),
        format!("{{\n  \"curriculum_version\": \"{version}\",\n  \"format_version\": 1\n}}\n"),
    )
    .unwrap();
}

/// Create a minimal cmetal project in a temp directory.
fn setup_project(tmp: &Path, exercises: &[(&str, &str, &str)]) {
    let with_compilers: Vec<_> = exercises
        .iter()
        .map(|(n, d, c)| (*n, *d, *c, None))
        .collect();
    setup_project_with(tmp, &with_compilers);
}

/// Like `setup_project`, but each exercise may pin a `compilers` list —
/// the only way to reach the "requires another compiler" paths.
fn setup_project_with(tmp: &Path, exercises: &[(&str, &str, &str, Option<&str>)]) {
    // exercises: [(name, dir, c_code, compilers)]
    let include_dir = tmp.join("include");
    std::fs::create_dir_all(&include_dir).unwrap();
    std::fs::copy(
        concat!(env!("CARGO_MANIFEST_DIR"), "/include/cmetal_test.h"),
        include_dir.join("cmetal_test.h"),
    )
    .unwrap();

    let mut toml = String::from("format_version = 1\n\n");
    for (name, dir, code, compilers) in exercises {
        toml.push_str(&format!(
            "[[exercises]]\nname = \"{name}\"\ndir = \"{dir}\"\ntest = false\nsanitizers = false\n"
        ));
        if let Some(list) = compilers {
            toml.push_str(&format!("compilers = [\"{list}\"]\n"));
        }
        toml.push('\n');

        let ex_dir = tmp.join("exercises").join(dir);
        std::fs::create_dir_all(&ex_dir).unwrap();
        std::fs::write(ex_dir.join(format!("{name}.c")), code).unwrap();

        let sol_dir = tmp.join("solutions").join(dir);
        std::fs::create_dir_all(&sol_dir).unwrap();
        std::fs::write(sol_dir.join(format!("{name}.c")), code).unwrap();
    }

    std::fs::write(tmp.join("info.toml"), toml).unwrap();
}

// ---- Compiler integration tests ----

#[test]
fn compile_valid_c_file() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }

    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("hello.c");
    std::fs::write(
        &source,
        r#"#include <stdio.h>
int main(void) { printf("hello\n"); return 0; }
"#,
    )
    .unwrap();

    let output = tmp.path().join("hello");
    let result = Command::new("gcc")
        .args(["-std=c11", "-o"])
        .arg(&output)
        .arg(&source)
        .output()
        .unwrap();

    assert!(result.status.success(), "gcc should compile valid C");
    assert!(output.exists());
}

#[test]
fn compile_invalid_c_file() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }

    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("bad.c");
    std::fs::write(&source, "this is not valid C code;\n").unwrap();

    let output = tmp.path().join("bad");
    let result = Command::new("gcc")
        .args(["-std=c11", "-o"])
        .arg(&output)
        .arg(&source)
        .output()
        .unwrap();

    assert!(!result.status.success(), "gcc should fail on invalid C");
}

// ---- CLI integration tests ----

#[test]
fn cli_list_shows_exercises() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }

    let tmp = TempDir::new().unwrap();
    setup_project(
        tmp.path(),
        &[(
            "hello",
            "00_intro",
            "#include <stdio.h>\nint main(void) { return 0; }\n",
        )],
    );

    let output = Command::new(cmetal_bin())
        .arg("list")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("hello"),
        "list output should contain exercise name, got: {stdout}"
    );
}

#[test]
fn cli_verify_valid_exercises() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }

    let tmp = TempDir::new().unwrap();
    setup_project(
        tmp.path(),
        &[(
            "ok1",
            "00_intro",
            "#include <stdio.h>\nint main(void) { printf(\"ok\\n\"); return 0; }\n",
        )],
    );

    let output = Command::new(cmetal_bin())
        .arg("verify")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "verify should pass for valid exercises, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn cli_verify_fails_on_broken_exercise() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }

    let tmp = TempDir::new().unwrap();
    setup_project(tmp.path(), &[("broken", "00_intro", "not valid C;\n")]);

    let output = Command::new(cmetal_bin())
        .arg("verify")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "verify should fail for broken exercises"
    );
}

#[test]
fn cli_reset_clears_state() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }

    let tmp = TempDir::new().unwrap();
    setup_project(
        tmp.path(),
        &[(
            "ex1",
            "00_intro",
            "#include <stdio.h>\nint main(void) { return 0; }\n",
        )],
    );

    // Create a fake state file
    std::fs::write(
        tmp.path().join(".cmetal-state.txt"),
        "DON'T EDIT THIS FILE!\n\nex1\n\nex1\n",
    )
    .unwrap();

    let output = Command::new(cmetal_bin())
        .arg("reset")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "reset should succeed");
    assert!(
        !tmp.path().join(".cmetal-state.txt").exists(),
        "state file should be removed after reset"
    );
}

#[test]
fn cli_solution_gated_until_solved() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }

    let tmp = TempDir::new().unwrap();
    setup_project(
        tmp.path(),
        &[("ex1", "00_intro", "int main(void) { return 1; }\n")],
    );
    // The solution differs from the (broken) exercise
    let solution = "int main(void) { return 0; }\n";
    std::fs::write(tmp.path().join("solutions/00_intro/ex1.c"), solution).unwrap();

    // Exercise still broken: solution must stay locked
    let output = Command::new(cmetal_bin())
        .args(["solution", "ex1"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "solution must be locked while failing"
    );
    assert!(
        !tmp.path().join("my_solutions/00_intro/ex1.c").exists(),
        "solution must not be revealed while the exercise fails"
    );

    // "Solve" the exercise in the workspace, then ask again
    std::fs::write(tmp.path().join("my_exercises/00_intro/ex1.c"), solution).unwrap();
    let output = Command::new(cmetal_bin())
        .args(["solution", "ex1"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "solution should unlock once the exercise passes, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let revealed = tmp.path().join("my_solutions/00_intro/ex1.c");
    assert_eq!(std::fs::read_to_string(&revealed).unwrap(), solution);
}

#[test]
fn cli_creates_workspace_copies() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }

    let tmp = TempDir::new().unwrap();
    let code = "#include <stdio.h>\nint main(void) { return 0; }\n";
    setup_project(tmp.path(), &[("hello", "00_intro", code)]);

    let output = Command::new(cmetal_bin())
        .arg("list")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    let work_file = tmp.path().join("my_exercises/00_intro/hello.c");
    assert!(work_file.exists(), "workspace copy should be created");
    assert_eq!(std::fs::read_to_string(&work_file).unwrap(), code);

    // A second run must not overwrite the learner's edits
    std::fs::write(&work_file, "// my work\n").unwrap();
    let output = Command::new(cmetal_bin())
        .arg("list")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        std::fs::read_to_string(&work_file).unwrap(),
        "// my work\n",
        "existing workspace files must not be overwritten"
    );
}

#[test]
fn cli_reset_restores_workspace() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }

    let tmp = TempDir::new().unwrap();
    let code = "#include <stdio.h>\nint main(void) { return 0; }\n";
    setup_project(tmp.path(), &[("hello", "00_intro", code)]);

    let work_file = tmp.path().join("my_exercises/00_intro/hello.c");
    std::fs::create_dir_all(work_file.parent().unwrap()).unwrap();
    std::fs::write(&work_file, "// solved\n").unwrap();

    let output = Command::new(cmetal_bin())
        .arg("reset")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "reset should succeed");
    assert_eq!(
        std::fs::read_to_string(&work_file).unwrap(),
        code,
        "reset should restore the pristine exercise"
    );
}

#[test]
fn cli_run_specific_exercise() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }

    let tmp = TempDir::new().unwrap();
    setup_project(
        tmp.path(),
        &[(
            "hello",
            "00_intro",
            "#include <stdio.h>\nint main(void) { printf(\"hello world\\n\"); return 0; }\n",
        )],
    );

    let output = Command::new(cmetal_bin())
        .args(["run", "hello"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "run should succeed for valid exercise, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn cli_run_and_verify_persist_state() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }

    let tmp = TempDir::new().unwrap();
    setup_project(
        tmp.path(),
        &[
            ("ok1", "00_intro", "int main(void) { return 0; }\n"),
            ("ok2", "00_intro", "int main(void) { return 0; }\n"),
            ("ok3", "00_intro", "int main(void) { return 0; }\n"),
        ],
    );

    // A passing `cmetal run` must mark the exercise done...
    let output = Command::new(cmetal_bin())
        .args(["run", "ok1"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let state = std::fs::read_to_string(tmp.path().join(".cmetal-state.txt"))
        .expect("run must write the state file");
    assert!(state.contains("ok1"), "run must mark the exercise done");

    // ...a `cmetal solution` unlocked by an on-the-spot verify pass must
    // persist that completion too (before `verify` runs, so the unlock
    // really goes through the verified-now path)...
    let output = Command::new(cmetal_bin())
        .args(["solution", "ok3"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let state = std::fs::read_to_string(tmp.path().join(".cmetal-state.txt")).unwrap();
    assert!(
        state.contains("ok3"),
        "solution must mark a verified-now exercise done, state:\n{state}"
    );
    assert!(
        !state.contains("ok2"),
        "ok2 has not been verified yet, state:\n{state}"
    );

    // ...and `cmetal verify` must persist every exercise that passed.
    let output = Command::new(cmetal_bin())
        .arg("verify")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let state = std::fs::read_to_string(tmp.path().join(".cmetal-state.txt")).unwrap();
    assert!(
        state.contains("ok1") && state.contains("ok2") && state.contains("ok3"),
        "verify must mark passing exercises done, state:\n{state}"
    );
}

#[test]
fn cli_init_creates_selfcontained_workspace() {
    let tmp = TempDir::new().unwrap();
    let ws = tmp.path().join("course");

    let output = Command::new(cmetal_bin())
        .arg("init")
        .arg(&ws)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(ws.join("info.toml").exists());
    assert!(ws.join("exercises/00_intro/intro1.c").exists());
    assert!(ws.join("include/cmetal_test.h").exists());
    assert!(ws.join("include/cmetal_alloc.h").exists());
    assert!(ws.join("solutions/00_intro/intro1.c.enc").exists());
    assert!(ws.join(".cmetal/manifest.json").exists());
    // The archive must never ship plaintext solutions
    assert!(!ws.join("solutions/00_intro/intro1.c").exists());

    // init must refuse to clobber an existing workspace
    let output = Command::new(cmetal_bin())
        .arg("init")
        .arg(&ws)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "init must not overwrite a workspace"
    );

    // ...and must refuse ANY non-empty directory, leaving it untouched
    let occupied = tmp.path().join("occupied");
    std::fs::create_dir(&occupied).unwrap();
    std::fs::write(occupied.join("keep.txt"), "do not overwrite").unwrap();
    let output = Command::new(cmetal_bin())
        .arg("init")
        .arg(&occupied)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "init must refuse a non-empty directory"
    );
    assert_eq!(
        std::fs::read_to_string(occupied.join("keep.txt")).unwrap(),
        "do not overwrite"
    );
    assert!(!occupied.join("info.toml").exists());

    // An existing but EMPTY directory is fine
    let empty = tmp.path().join("empty");
    std::fs::create_dir(&empty).unwrap();
    let output = Command::new(cmetal_bin())
        .arg("init")
        .arg(&empty)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "init must accept an empty directory: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(empty.join("info.toml").exists());

    // The engine treats the workspace exactly like a repo checkout
    if has_gcc() {
        let output = Command::new(cmetal_bin())
            .arg("list")
            .current_dir(&ws)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "list in workspace failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("intro1"), "list should show the curriculum");
    }
}

#[test]
fn cli_update_reconciles_workspace() {
    let tmp = TempDir::new().unwrap();
    let ws = tmp.path().join("course");
    let output = Command::new(cmetal_bin())
        .arg("init")
        .arg(&ws)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // Pretend the workspace came from an older curriculum, or the
    // version guard would short-circuit the update.
    stamp_manifest(&ws, "0.0.1");

    let pristine_intro = ws.join("exercises/00_intro/intro1.c");
    let pristine_p1 = ws.join("exercises/01_pointers/pointers1.c");
    let embedded_intro = std::fs::read_to_string(&pristine_intro).unwrap();
    let embedded_p1 = std::fs::read_to_string(&pristine_p1).unwrap();

    // Simulate an OLDER curriculum than the embedded one:
    // - intro1 differed upstream, learner copy untouched (== old pristine)
    let old_intro = format!("{embedded_intro}\n// old curriculum\n");
    std::fs::write(&pristine_intro, &old_intro).unwrap();
    std::fs::create_dir_all(ws.join("my_exercises/00_intro")).unwrap();
    std::fs::write(ws.join("my_exercises/00_intro/intro1.c"), &old_intro).unwrap();
    // - pointers1 differed upstream, learner copy has real edits
    std::fs::write(&pristine_p1, format!("{embedded_p1}\n// old\n")).unwrap();
    std::fs::create_dir_all(ws.join("my_exercises/01_pointers")).unwrap();
    std::fs::write(
        ws.join("my_exercises/01_pointers/pointers1.c"),
        "// my work\n",
    )
    .unwrap();
    // - bitwise3 did not exist in the old curriculum
    std::fs::remove_file(ws.join("exercises/11_bitwise/bitwise3.c")).unwrap();

    let output = Command::new(cmetal_bin())
        .arg("update")
        .current_dir(&ws)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "update failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Pristine curriculum fully replaced by the embedded one
    assert_eq!(
        std::fs::read_to_string(&pristine_intro).unwrap(),
        embedded_intro
    );
    assert_eq!(std::fs::read_to_string(&pristine_p1).unwrap(), embedded_p1);
    assert!(ws.join("exercises/11_bitwise/bitwise3.c").exists());
    // Untouched working copy refreshed to the new pristine version
    assert_eq!(
        std::fs::read_to_string(ws.join("my_exercises/00_intro/intro1.c")).unwrap(),
        embedded_intro
    );
    // Edited working copy kept, and reported
    assert_eq!(
        std::fs::read_to_string(ws.join("my_exercises/01_pointers/pointers1.c")).unwrap(),
        "// my work\n"
    );
    assert!(stdout.contains("pointers1"), "kept edits must be reported");
    assert!(
        stdout.contains("bitwise3"),
        "new exercises must be reported"
    );
    // No staging/backup leftovers
    assert!(!ws.join(".cmetal/staging").exists());
    assert!(!ws.join(".cmetal/backup").exists());

    // In a git-checkout-like directory (no manifest) update must refuse
    std::fs::remove_file(ws.join(".cmetal/manifest.json")).unwrap();
    let output = Command::new(cmetal_bin())
        .arg("update")
        .current_dir(&ws)
        .output()
        .unwrap();
    assert!(!output.status.success(), "update must require a manifest");
}

#[test]
fn cli_update_interrupted_swap_rolls_back() {
    let tmp = TempDir::new().unwrap();
    let ws = tmp.path().join("course");
    let output = Command::new(cmetal_bin())
        .arg("init")
        .arg(&ws)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    stamp_manifest(&ws, "0.0.1");

    let pristine_intro = ws.join("exercises/00_intro/intro1.c");
    let embedded_intro = std::fs::read_to_string(&pristine_intro).unwrap();
    let old_intro = format!("{embedded_intro}\n// old curriculum\n");
    std::fs::write(&pristine_intro, &old_intro).unwrap();
    std::fs::create_dir_all(ws.join("my_exercises/00_intro")).unwrap();
    std::fs::write(ws.join("my_exercises/00_intro/intro1.c"), &old_intro).unwrap();
    std::fs::create_dir_all(ws.join("my_exercises/01_pointers")).unwrap();
    std::fs::write(
        ws.join("my_exercises/01_pointers/pointers1.c"),
        "// my work\n",
    )
    .unwrap();

    // Inject a failure halfway through phase B of the swap.
    let output = Command::new(cmetal_bin())
        .arg("update")
        .env("CMETAL_TEST_FAIL_INSTALL", "include")
        .current_dir(&ws)
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "injected failure must fail update"
    );

    // The workspace must be exactly as before the attempt.
    assert_eq!(
        std::fs::read_to_string(&pristine_intro).unwrap(),
        old_intro,
        "old pristine curriculum must be rolled back into place"
    );
    assert_eq!(
        std::fs::read_to_string(ws.join("my_exercises/00_intro/intro1.c")).unwrap(),
        old_intro,
        "working copies must not be refreshed by a failed update"
    );
    assert_eq!(
        std::fs::read_to_string(ws.join("my_exercises/01_pointers/pointers1.c")).unwrap(),
        "// my work\n"
    );
    assert!(ws.join("info.toml").exists());
    assert!(ws.join("include/cmetal_test.h").exists());
    assert!(
        !ws.join(".cmetal/backup").exists(),
        "a fully rolled-back update must not leave a backup behind"
    );

    // A second update (no injection) must succeed and lose nothing.
    let output = Command::new(cmetal_bin())
        .arg("update")
        .current_dir(&ws)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "retry after rollback failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(&pristine_intro).unwrap(),
        embedded_intro
    );
    assert_eq!(
        std::fs::read_to_string(ws.join("my_exercises/00_intro/intro1.c")).unwrap(),
        embedded_intro,
        "untouched copy must be refreshed by the successful retry"
    );
    assert_eq!(
        std::fs::read_to_string(ws.join("my_exercises/01_pointers/pointers1.c")).unwrap(),
        "// my work\n"
    );
}

#[test]
fn cli_update_recovers_interrupted_backup() {
    let tmp = TempDir::new().unwrap();
    let ws = tmp.path().join("course");
    let output = Command::new(cmetal_bin())
        .arg("init")
        .arg(&ws)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    stamp_manifest(&ws, "0.0.1");

    // Simulate a crash mid-swap: info.toml moved to backup, never
    // replaced. resolve_base_dir would fail here — update must still
    // find the workspace via .cmetal/manifest.json and recover.
    std::fs::create_dir_all(ws.join(".cmetal/backup")).unwrap();
    std::fs::rename(ws.join("info.toml"), ws.join(".cmetal/backup/info.toml")).unwrap();

    let output = Command::new(cmetal_bin())
        .arg("update")
        .current_dir(&ws)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "update must recover an interrupted workspace: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(ws.join("info.toml").exists(), "info.toml must be restored");
    assert!(
        !ws.join(".cmetal/backup").exists(),
        "backup must be consumed"
    );
}

#[test]
fn cli_update_version_guard() {
    let tmp = TempDir::new().unwrap();
    let ws = tmp.path().join("course");
    let output = Command::new(cmetal_bin())
        .arg("init")
        .arg(&ws)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // Same version: a no-op that says so, without churning the tree.
    let output = Command::new(cmetal_bin())
        .arg("update")
        .current_dir(&ws)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("nothing to update"),
        "same-version update must be a no-op"
    );

    // Newer workspace than binary: refuse the downgrade.
    stamp_manifest(&ws, "99.0.0");
    let output = Command::new(cmetal_bin())
        .arg("update")
        .current_dir(&ws)
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "update must refuse to downgrade a newer workspace"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("NEWER"),
        "the refusal must explain the version conflict"
    );
}

#[test]
fn cli_migrates_prerename_clings_workspace() {
    let tmp = TempDir::new().unwrap();
    let ws = tmp.path().join("course");
    let output = Command::new(cmetal_bin())
        .arg("init")
        .arg(&ws)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // Turn it into a pre-rename workspace: .clings metadata (with an
    // older curriculum version) and a .clings-state.txt with progress.
    std::fs::rename(ws.join(".cmetal"), ws.join(".clings")).unwrap();
    std::fs::write(
        ws.join(".clings/manifest.json"),
        "{\n  \"curriculum_version\": \"0.0.1\",\n  \"format_version\": 1\n}\n",
    )
    .unwrap();
    std::fs::write(
        ws.join(".clings-state.txt"),
        "DON'T EDIT THIS FILE!\n\nintro1\n\nintro1\n",
    )
    .unwrap();

    // update must find the workspace via the legacy marker, migrate the
    // metadata directory, and complete normally.
    let output = Command::new(cmetal_bin())
        .arg("update")
        .current_dir(&ws)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "update on a pre-rename workspace failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(ws.join(".cmetal/manifest.json").exists());
    assert!(
        !ws.join(".clings").exists(),
        "legacy metadata dir must be adopted"
    );

    // The legacy state file is adopted on the first state load.
    if has_gcc() {
        let output = Command::new(cmetal_bin())
            .arg("list")
            .current_dir(&ws)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(
            ws.join(".cmetal-state.txt").exists(),
            "state must be migrated to the new name"
        );
        assert!(!ws.join(".clings-state.txt").exists());
        let state = std::fs::read_to_string(ws.join(".cmetal-state.txt")).unwrap();
        assert!(
            state.contains("intro1"),
            "progress must survive the migration"
        );
    }
}

#[test]
fn cli_diff_and_reset_single_exercise() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }

    let tmp = TempDir::new().unwrap();
    let ws = tmp.path().join("course");
    let output = Command::new(cmetal_bin())
        .arg("init")
        .arg(&ws)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // Learner edits their copy of intro1
    std::fs::create_dir_all(ws.join("my_exercises/00_intro")).unwrap();
    std::fs::write(
        ws.join("my_exercises/00_intro/intro1.c"),
        "// edited by me\nint main(void) { return 1; }\n",
    )
    .unwrap();

    let output = Command::new(cmetal_bin())
        .args(["diff", "intro1"])
        .current_dir(&ws)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("+// edited by me") && stdout.contains("yours"),
        "diff must show the learner's changes, got:\n{stdout}"
    );

    // Mark intro1 done first: reset <name> must un-done it, or watch
    // mode would never offer the restored exercise again.
    std::fs::write(
        ws.join(".cmetal-state.txt"),
        "DON'T EDIT THIS FILE!\n\nintro1\n\nintro1\n",
    )
    .unwrap();

    let output = Command::new(cmetal_bin())
        .args(["reset", "intro1"])
        .current_dir(&ws)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "reset <name> failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let state = std::fs::read_to_string(ws.join(".cmetal-state.txt")).unwrap();
    let done_entries: Vec<&str> = state
        .lines()
        .skip(3) // header, blank, current name
        .filter(|l| !l.trim().is_empty())
        .collect();
    assert!(
        !done_entries.contains(&"intro1"),
        "reset <name> must mark the exercise pending again, state:\n{state}"
    );
    assert_eq!(
        std::fs::read_to_string(ws.join("my_exercises/00_intro/intro1.c")).unwrap(),
        std::fs::read_to_string(ws.join("exercises/00_intro/intro1.c")).unwrap()
    );

    let output = Command::new(cmetal_bin())
        .args(["diff", "intro1"])
        .current_dir(&ws)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("identical"),
        "after reset the diff must be empty, got:\n{stdout}"
    );
}

#[test]
fn cli_hint_shows_hint_text() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }

    let tmp = TempDir::new().unwrap();
    let include_dir = tmp.path().join("include");
    std::fs::create_dir_all(&include_dir).unwrap();
    std::fs::copy(
        concat!(env!("CARGO_MANIFEST_DIR"), "/include/cmetal_test.h"),
        include_dir.join("cmetal_test.h"),
    )
    .unwrap();

    let ex_dir = tmp.path().join("exercises").join("00_intro");
    std::fs::create_dir_all(&ex_dir).unwrap();
    std::fs::write(
        ex_dir.join("ex1.c"),
        "#include <stdio.h>\nint main(void) { return 0; }\n",
    )
    .unwrap();

    let sol_dir = tmp.path().join("solutions").join("00_intro");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(
        sol_dir.join("ex1.c"),
        "#include <stdio.h>\nint main(void) { return 0; }\n",
    )
    .unwrap();

    let toml = r#"format_version = 1

[[exercises]]
name = "ex1"
dir = "00_intro"
test = false
sanitizers = false
hints = ["Try using printf", "Check the return value"]
"#;
    std::fs::write(tmp.path().join("info.toml"), toml).unwrap();

    let output = Command::new(cmetal_bin())
        .args(["hint", "ex1"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Try using printf"),
        "hint should show hint text, got: {stdout}"
    );
}

// ---- `run`: outcome -> exit code contract ----
//
// These pin the mapping the whole CLI composes on: a pass is 0, a
// wrong answer is 1, a missing file is 1, and an exercise this
// compiler cannot judge is *not* a failure.

#[test]
fn cli_run_exits_zero_on_pass() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }
    let tmp = TempDir::new().unwrap();
    setup_project(
        tmp.path(),
        &[(
            "ok",
            "00_intro",
            "#include <stdio.h>\nint main(void) { printf(\"hello\\n\"); return 0; }\n",
        )],
    );

    let output = Command::new(cmetal_bin())
        .args(["run", "ok"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "expected exit 0, stdout: {stdout}");
    // The stage label must not stutter: print_stage_output already
    // appends the word "output".
    assert!(
        stdout.contains("Program output"),
        "expected a 'Program output' section, got: {stdout}"
    );
    assert!(
        !stdout.contains("Output output"),
        "stage label stutters: {stdout}"
    );
}

#[test]
fn cli_run_exits_one_on_failing_exercise() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }
    let tmp = TempDir::new().unwrap();
    setup_project(
        tmp.path(),
        &[("bad", "00_intro", "int main(void) { return 1; }\n")],
    );

    let output = Command::new(cmetal_bin())
        .args(["run", "bad"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(1),
        "a failing exercise must exit 1, stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn cli_run_exits_one_when_the_file_is_gone() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }
    let tmp = TempDir::new().unwrap();
    setup_project(
        tmp.path(),
        &[("ghost", "00_intro", "int main(void) { return 0; }\n")],
    );
    // Both copies must go: prepare_workspace recreates the working
    // copy from the pristine file on every startup.
    std::fs::remove_file(tmp.path().join("exercises/00_intro/ghost.c")).unwrap();

    let output = Command::new(cmetal_bin())
        .args(["run", "ghost"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(output.status.code(), Some(1), "stdout: {stdout}");
    assert!(
        stdout.contains("not found"),
        "expected a missing-file message, got: {stdout}"
    );
}

#[test]
fn cli_run_skips_exercises_for_another_compiler() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }
    let tmp = TempDir::new().unwrap();
    setup_project_with(
        tmp.path(),
        &[(
            "clang_only",
            "00_intro",
            "int main(void) { return 0; }\n",
            Some("clang"),
        )],
    );

    let output = Command::new(cmetal_bin())
        .args(["--compiler", "gcc", "run", "clang_only"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Skipped is not failed: the learner asked for an exercise this
    // compiler cannot judge and was told so.
    assert!(
        output.status.success(),
        "an unsupported exercise must not fail the command, stdout: {stdout}"
    );
    assert!(
        stdout.contains("requires clang"),
        "expected the required compiler to be named, got: {stdout}"
    );
}

#[test]
fn cli_list_shows_status_glyphs() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }
    let tmp = TempDir::new().unwrap();
    setup_project_with(
        tmp.path(),
        &[
            ("solved", "00_intro", "int main(void) { return 0; }\n", None),
            (
                "skipped",
                "00_intro",
                "int main(void) { return 0; }\n",
                Some("clang"),
            ),
        ],
    );

    // Assert the setup step: if `run` regresses, this test must not
    // report it as a broken list renderer.
    let setup = Command::new(cmetal_bin())
        .args(["run", "solved"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        setup.status.success(),
        "setup: `run solved` should pass, stdout: {}",
        String::from_utf8_lossy(&setup.stdout)
    );

    let output = Command::new(cmetal_bin())
        .arg("list")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("✓ solved"),
        "a solved exercise should be ticked, got: {stdout}"
    );
    assert!(
        stdout.contains("− skipped") && stdout.contains("(requires clang)"),
        "an unsupported exercise should be marked and annotated, got: {stdout}"
    );
}

#[test]
fn cli_verify_fails_when_an_exercise_file_is_missing() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }
    let tmp = TempDir::new().unwrap();
    setup_project(
        tmp.path(),
        &[("ghost", "00_intro", "int main(void) { return 0; }\n")],
    );
    // An info.toml entry with no .c file behind it — what a packaging
    // slip looks like. The sweep must not call that a pass.
    std::fs::remove_file(tmp.path().join("exercises/00_intro/ghost.c")).unwrap();

    let output = Command::new(cmetal_bin())
        .arg("verify")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(
        output.status.code(),
        Some(1),
        "a missing exercise file must fail the sweep, stdout: {stdout}"
    );
    assert!(
        !stdout.contains("All exercises passed"),
        "the sweep must not claim success it did not verify: {stdout}"
    );
}

#[test]
fn cli_diff_defaults_to_the_current_exercise() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }
    let tmp = TempDir::new().unwrap();
    setup_project(
        tmp.path(),
        &[("first", "00_intro", "int main(void) { return 0; }\n")],
    );
    // Materialize the workspace so a working copy exists to diff.
    Command::new(cmetal_bin())
        .arg("list")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let output = Command::new(cmetal_bin())
        .arg("diff")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "`diff` with no argument should default to the current exercise, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("first"),
        "expected the current exercise to be named, got: {stdout}"
    );
}

/// No `has_gcc()` guard, deliberately: `diff <name>` compiles nothing,
/// so it must work on a machine with no toolchain at all. The empty
/// PATH proves it rather than assuming it.
#[test]
fn cli_diff_named_exercise_ignores_a_damaged_state_file() {
    let tmp = TempDir::new().unwrap();
    setup_project(
        tmp.path(),
        &[("foo", "00_intro", "int main(void) { return 0; }\n")],
    );

    // Not valid UTF-8: AppState::load would fail on this.
    let state_file = tmp.path().join(".cmetal-state.txt");
    std::fs::write(&state_file, [0xff, 0xfe, 0x00, b'x']).unwrap();

    let no_tools = no_toolchain();
    let output = Command::new(cmetal_bin())
        .args(["diff", "foo"])
        .current_dir(tmp.path())
        .env("PATH", no_tools.path())
        .output()
        .unwrap();

    // diff is a diagnostic command: asked about a named exercise it
    // must answer even when progress is unreadable.
    assert!(
        output.status.success(),
        "diff <name> must not depend on the progress file, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // ...and it must not have rewritten or migrated it behind our back.
    assert_eq!(
        std::fs::read(&state_file).unwrap(),
        vec![0xff, 0xfe, 0x00, b'x'],
        "diff <name> must leave the progress file untouched"
    );
}

// ---- Commands that compile nothing must not need a toolchain ----
//
// These run with an empty PATH, so a compiler cannot be found even if
// one is installed. That is the point: `list`, `hint` and `reset`
// never invoke one, and probing for a toolchain they will never use
// would lock out anyone who has not installed one yet.

#[test]
fn cli_compiler_free_commands_work_without_a_toolchain() {
    let tmp = TempDir::new().unwrap();
    setup_project(
        tmp.path(),
        &[("foo", "00_intro", "int main(void) { return 0; }\n")],
    );

    for args in [vec!["list"], vec!["hint", "foo"], vec!["reset"]] {
        let no_tools = no_toolchain();
        let output = Command::new(cmetal_bin())
            .args(&args)
            .current_dir(tmp.path())
            .env("PATH", no_tools.path())
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "`cmetal {}` must not require a compiler, stderr: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn cli_run_still_reports_a_missing_toolchain() {
    let tmp = TempDir::new().unwrap();
    setup_project(
        tmp.path(),
        &[("foo", "00_intro", "int main(void) { return 0; }\n")],
    );

    let no_tools = no_toolchain();
    let output = Command::new(cmetal_bin())
        .args(["run", "foo"])
        .current_dir(tmp.path())
        .env("PATH", no_tools.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Deferring construction must not turn "no compiler installed"
    // into a confusing failure further down.
    assert!(!output.status.success(), "run without gcc should fail");
    assert!(
        stderr.contains("gcc not found"),
        "expected a clear missing-toolchain error, got: {stderr}"
    );
}

/// No `has_gcc()` guard: recovering from a damaged progress file is
/// pure state handling, so it must hold on a machine with no toolchain.
#[test]
fn cli_read_only_commands_leave_a_damaged_state_file_alone() {
    let tmp = TempDir::new().unwrap();
    setup_project(
        tmp.path(),
        &[("foo", "00_intro", "int main(void) { return 0; }\n")],
    );
    let state_file = tmp.path().join(".cmetal-state.txt");
    let damaged = [0xff, 0xfe, 0x00, b'x'];

    // Unreadable progress must not lock the learner out of the tool,
    // and a command that only reads must leave the workspace exactly
    // as it found it — no rename, no rewrite.
    for args in [vec!["list"], vec!["hint", "foo"], vec!["diff"]] {
        std::fs::write(&state_file, damaged).unwrap();
        let output = Command::new(cmetal_bin())
            .args(&args)
            .current_dir(tmp.path())
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            output.status.success(),
            "`cmetal {}` should survive a damaged progress file, stderr: {stderr}",
            args.join(" ")
        );
        // The warning belongs on stderr: `cmetal list` gets piped.
        assert!(
            stderr.contains("empty progress"),
            "the learner must be told progress was lost, stderr: {stderr}"
        );
        assert!(
            !String::from_utf8_lossy(&output.stdout).contains("empty progress"),
            "the warning must not pollute stdout"
        );
        assert_eq!(
            std::fs::read(&state_file).unwrap(),
            damaged,
            "`cmetal {}` only reads, so it must not touch the file",
            args.join(" ")
        );
        assert!(
            !tmp.path().join(".cmetal-state.txt.damaged").exists(),
            "nothing should be moved aside before something destroys it"
        );
    }
}

#[test]
fn cli_preserves_a_damaged_state_file_before_overwriting_it() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }
    let tmp = TempDir::new().unwrap();
    setup_project(
        tmp.path(),
        &[("foo", "00_intro", "int main(void) { return 0; }\n")],
    );
    let state_file = tmp.path().join(".cmetal-state.txt");

    // `run` saves on a pass, so it is the first thing that would
    // clobber the unreadable file. A second corruption must not
    // overwrite the first backup: the older record is the richer one.
    let rich = b"DON'T EDIT THIS FILE!\n\nfoo\n\nfoo\n\xff bad\n";
    let poor = b"DON'T EDIT THIS FILE!\n\n\n\n\xff bad\n";
    for content in [rich.as_slice(), poor.as_slice()] {
        std::fs::write(&state_file, content).unwrap();
        let output = Command::new(cmetal_bin())
            .args(["run", "foo"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "run should pass, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    assert_eq!(
        std::fs::read(tmp.path().join(".cmetal-state.txt.damaged")).unwrap(),
        rich,
        "the first backup must survive a later one"
    );
    assert_eq!(
        std::fs::read(tmp.path().join(".cmetal-state.txt.damaged.1")).unwrap(),
        poor,
        "the second corruption gets its own name"
    );

    // reset clears progress but must not destroy the salvageable
    // copies — so it has to say what it left behind.
    std::fs::write(&state_file, [0xff, 0xfe, 0x00, b'x']).unwrap();
    let output = Command::new(cmetal_bin())
        .arg("reset")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "reset should survive it too");
    assert!(
        !state_file.exists(),
        "reset must clear the unreadable progress file"
    );
    assert!(
        tmp.path().join(".cmetal-state.txt.damaged.2").exists(),
        "reset must move the unreadable file aside, not delete it"
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("Left in place"),
        "reset must name the backups it kept, stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}
