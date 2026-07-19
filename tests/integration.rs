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

/// Build the clings binary path (debug target).
fn clings_bin() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    // test binary is in target/debug/deps/, clings binary is in target/debug/
    path.pop(); // remove test binary name
    if path.ends_with("deps") {
        path.pop();
    }
    path.push("clings");
    path
}

/// Create a minimal clings project in a temp directory.
fn setup_project(tmp: &Path, exercises: &[(&str, &str, &str)]) {
    // exercises: [(name, dir, c_code)]
    let include_dir = tmp.join("include");
    std::fs::create_dir_all(&include_dir).unwrap();
    std::fs::copy(
        concat!(env!("CARGO_MANIFEST_DIR"), "/include/clings_test.h"),
        include_dir.join("clings_test.h"),
    )
    .unwrap();

    let mut toml = String::from("format_version = 1\n\n");
    for (name, dir, code) in exercises {
        toml.push_str(&format!(
            "[[exercises]]\nname = \"{name}\"\ndir = \"{dir}\"\ntest = false\nsanitizers = false\n\n"
        ));

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

    let output = Command::new(clings_bin())
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

    let output = Command::new(clings_bin())
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

    let output = Command::new(clings_bin())
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
        tmp.path().join(".clings-state.txt"),
        "DON'T EDIT THIS FILE!\n\nex1\n\nex1\n",
    )
    .unwrap();

    let output = Command::new(clings_bin())
        .arg("reset")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "reset should succeed");
    assert!(
        !tmp.path().join(".clings-state.txt").exists(),
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
    let output = Command::new(clings_bin())
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
    let output = Command::new(clings_bin())
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

    let output = Command::new(clings_bin())
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
    let output = Command::new(clings_bin())
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

    let output = Command::new(clings_bin())
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

    let output = Command::new(clings_bin())
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

    // A passing `clings run` must mark the exercise done...
    let output = Command::new(clings_bin())
        .args(["run", "ok1"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let state = std::fs::read_to_string(tmp.path().join(".clings-state.txt"))
        .expect("run must write the state file");
    assert!(state.contains("ok1"), "run must mark the exercise done");

    // ...a `clings solution` unlocked by an on-the-spot verify pass must
    // persist that completion too (before `verify` runs, so the unlock
    // really goes through the verified-now path)...
    let output = Command::new(clings_bin())
        .args(["solution", "ok3"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let state = std::fs::read_to_string(tmp.path().join(".clings-state.txt")).unwrap();
    assert!(
        state.contains("ok3"),
        "solution must mark a verified-now exercise done, state:\n{state}"
    );
    assert!(
        !state.contains("ok2"),
        "ok2 has not been verified yet, state:\n{state}"
    );

    // ...and `clings verify` must persist every exercise that passed.
    let output = Command::new(clings_bin())
        .arg("verify")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let state = std::fs::read_to_string(tmp.path().join(".clings-state.txt")).unwrap();
    assert!(
        state.contains("ok1") && state.contains("ok2") && state.contains("ok3"),
        "verify must mark passing exercises done, state:\n{state}"
    );
}

#[test]
fn cli_init_creates_selfcontained_workspace() {
    let tmp = TempDir::new().unwrap();
    let ws = tmp.path().join("course");

    let output = Command::new(clings_bin())
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
    assert!(ws.join("include/clings_test.h").exists());
    assert!(ws.join("include/clings_alloc.h").exists());
    assert!(ws.join("solutions/00_intro/intro1.c.enc").exists());
    assert!(ws.join(".clings/manifest.json").exists());
    // The archive must never ship plaintext solutions
    assert!(!ws.join("solutions/00_intro/intro1.c").exists());

    // init must refuse to clobber an existing workspace
    let output = Command::new(clings_bin())
        .arg("init")
        .arg(&ws)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "init must not overwrite a workspace"
    );

    // The engine treats the workspace exactly like a repo checkout
    if has_gcc() {
        let output = Command::new(clings_bin())
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
fn cli_hint_shows_hint_text() {
    if !has_gcc() {
        eprintln!("skipping: gcc not available");
        return;
    }

    let tmp = TempDir::new().unwrap();
    let include_dir = tmp.path().join("include");
    std::fs::create_dir_all(&include_dir).unwrap();
    std::fs::copy(
        concat!(env!("CARGO_MANIFEST_DIR"), "/include/clings_test.h"),
        include_dir.join("clings_test.h"),
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

    let output = Command::new(clings_bin())
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
