use crate::app_state::AppState;
use crate::compiler::Compiler;
use crate::term;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, DebouncedEventKind};
use std::io;
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, SystemTime};

enum WatchEvent {
    FileChanged,
    Key(KeyCode),
    Quit,
}

/// Restores the terminal no matter how run_watch exits: every `?`
/// between raw-mode entry and the end of the loop would otherwise
/// leave the user's shell in raw mode inside the alternate screen.
struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        crossterm::execute!(io::stdout(), EnterAlternateScreen)?;
        crossterm::terminal::enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(io::stdout(), LeaveAlternateScreen);
    }
}

/// Get the mtime of the current exercise file, if available.
fn current_exercise_mtime(state: &AppState) -> Option<SystemTime> {
    state
        .current_exercise()
        .and_then(|e| e.path.metadata().ok())
        .and_then(|m| m.modified().ok())
}

pub fn run_watch(
    state: &mut AppState,
    compiler: &Compiler,
    exercises_dir: &Path,
    build_dir: &Path,
    welcome: Option<&str>,
) -> Result<()> {
    let (tx, rx) = mpsc::channel();

    // Track which hint level we're at for the current exercise
    let mut hint_level: usize = 0;

    // Track whether the last run of the current exercise succeeded
    let mut last_run_success;

    // Track exercise mtime to avoid spurious refreshes
    let mut last_mtime = current_exercise_mtime(state);

    // File watcher
    let tx_fs = tx.clone();
    let mut debouncer = new_debouncer(
        Duration::from_millis(500),
        move |res: Result<Vec<DebouncedEvent>, notify::Error>| {
            if let Ok(events) = res {
                for event in events {
                    if event.kind == DebouncedEventKind::Any {
                        let path_str = event.path.to_string_lossy().to_string();
                        if path_str.ends_with(".c") {
                            let _ = tx_fs.send(WatchEvent::FileChanged);
                        }
                    }
                }
            }
        },
    )?;

    debouncer
        .watcher()
        .watch(exercises_dir, notify::RecursiveMode::Recursive)?;

    // Show welcome message before entering alternate screen
    if let Some(msg) = welcome {
        println!();
        for line in msg.lines() {
            println!("  {line}");
        }
        println!();
        println!(
            "  cmetal v{} — compiler: {}",
            env!("CARGO_PKG_VERSION"),
            compiler.kind()
        );
        println!();
        println!("  Press any key to start...");
        crossterm::terminal::enable_raw_mode()?;
        let _ = event::read();
        crossterm::terminal::disable_raw_mode()?;
    }

    // Enter alternate screen and enable raw mode; the guard restores
    // both on every exit path, early error returns included.
    let _guard = TerminalGuard::enter()?;

    // Initial run
    print_watch_header(state, compiler);
    last_run_success = run_current_exercise(state, compiler, build_dir);
    print_watch_commands();

    // Drain any events that arrived during initial compilation
    while rx.try_recv().is_ok() {}

    // Keyboard reader thread
    let tx_key = tx.clone();
    std::thread::spawn(move || loop {
        if let Ok(Event::Key(key_event)) = event::read() {
            if key_event.modifiers.contains(KeyModifiers::CONTROL)
                && key_event.code == KeyCode::Char('c')
            {
                let _ = tx_key.send(WatchEvent::Quit);
                break;
            }
            let _ = tx_key.send(WatchEvent::Key(key_event.code));
        }
    });

    loop {
        match rx.recv() {
            Ok(WatchEvent::FileChanged) => {
                // Only re-render if the exercise file actually changed
                let new_mtime = current_exercise_mtime(state);
                if new_mtime == last_mtime {
                    continue;
                }
                last_mtime = new_mtime;

                term::clear_screen();
                print_watch_header(state, compiler);
                last_run_success = run_current_exercise(state, compiler, build_dir);
                print_watch_commands();

                // Drain events that arrived during compilation
                while rx.try_recv().is_ok() {}
            }
            Ok(WatchEvent::Key(KeyCode::Char('n'))) => {
                // Mark current as done only if it passed, then advance
                if last_run_success {
                    if let Some(name) = state.current_exercise().map(|e| e.name().to_string()) {
                        state.mark_done(&name);
                    }
                }
                if state.all_done() {
                    term::clear_screen();
                    println!("\r");
                    term::print_success("All exercises completed! Congratulations!");
                    println!("\r");
                    break;
                }
                state.next_pending();
                state.save()?;
                hint_level = 0; // Reset hints for new exercise
                last_mtime = current_exercise_mtime(state);
                term::clear_screen();
                print_watch_header(state, compiler);
                last_run_success = run_current_exercise(state, compiler, build_dir);
                print_watch_commands();
                while rx.try_recv().is_ok() {}
            }
            Ok(WatchEvent::Key(KeyCode::Char('p'))) => {
                // Go back to previous exercise
                if state.prev() {
                    state.save()?;
                    hint_level = 0;
                    last_mtime = current_exercise_mtime(state);
                    term::clear_screen();
                    print_watch_header(state, compiler);
                    last_run_success = run_current_exercise(state, compiler, build_dir);
                    print_watch_commands();
                    while rx.try_recv().is_ok() {}
                }
            }
            Ok(WatchEvent::Key(KeyCode::Char('h'))) => {
                // Show progressive hint
                if let Some(exercise) = state.current_exercise() {
                    let hints = exercise.hints();
                    term::clear_screen();
                    print_watch_header(state, compiler);
                    println!("\r");

                    if hints.is_empty() {
                        term::print_warning("No hints available for this exercise.");
                    } else {
                        let current = hint_level.min(hints.len() - 1);
                        for i in 0..=current {
                            term::print_header(&format!("Hint {} of {}:", i + 1, hints.len()));
                            println!("\r");
                            for line in hints[i].lines() {
                                println!("  {line}\r");
                            }
                            println!("\r");
                        }

                        if current + 1 < hints.len() {
                            hint_level = current + 1;
                            term::print_info(&format!(
                                "Press 'h' again for the next hint ({} more).",
                                hints.len() - current - 1
                            ));
                        } else {
                            term::print_info("No more hints. You've seen them all!");
                        }
                    }
                    println!("\r");
                    print_watch_commands();
                }
            }
            Ok(WatchEvent::Key(KeyCode::Char('l'))) => {
                // List exercises
                term::clear_screen();
                println!("\r");
                term::print_header("Exercises:");
                println!("\r");
                for (i, ex) in state.exercises.iter().enumerate() {
                    let status = if state.is_done(ex.name()) {
                        "✓"
                    } else if !ex.supported {
                        "−"
                    } else if i == state.current_index {
                        "→"
                    } else {
                        " "
                    };
                    let note = if ex.supported {
                        String::new()
                    } else {
                        format!("  (requires {})", ex.required_compilers())
                    };
                    println!("  {status} {}{note}\r", ex.name());
                }
                println!("\r");
                print_watch_commands();
            }
            Ok(WatchEvent::Key(KeyCode::Char('r'))) => {
                // Re-run current exercise
                term::clear_screen();
                print_watch_header(state, compiler);
                last_run_success = run_current_exercise(state, compiler, build_dir);
                print_watch_commands();
                while rx.try_recv().is_ok() {}
            }
            Ok(WatchEvent::Key(KeyCode::Char('q'))) | Ok(WatchEvent::Quit) => {
                break;
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }

    drop(_guard); // restore the terminal before anything else prints
    state.save()?;
    Ok(())
}

fn print_watch_header(state: &AppState, compiler: &Compiler) {
    let (done, total) = state.progress();
    println!("\r");
    term::print_header(&format!(
        "cmetal v{} [{}]  Exercise {} of {}",
        env!("CARGO_PKG_VERSION"),
        compiler.kind(),
        state.current_index + 1,
        total
    ));
    term::print_progress(done, total);
    println!("\r");
}

fn print_watch_commands() {
    println!("\r");
    let mut stdout = io::stdout();
    let _ = crossterm::execute!(
        stdout,
        crossterm::style::SetForegroundColor(crossterm::style::Color::DarkGrey)
    );
    println!("  [n] next  [p] prev  [h] hint  [l] list  [r] re-run  [q] quit\r");
    let _ = crossterm::execute!(
        stdout,
        crossterm::style::SetAttribute(crossterm::style::Attribute::Reset)
    );
}

fn run_current_exercise(state: &AppState, compiler: &Compiler, build_dir: &Path) -> bool {
    let exercise = match state.current_exercise() {
        Some(e) => e,
        None => {
            term::print_warning("No exercises found.");
            return false;
        }
    };

    println!("  Exercise: {}\r", exercise.name());
    println!("  File: {}\r", exercise.path.display());
    println!("\r");

    if !exercise.supported {
        term::print_warning(&format!(
            "This exercise requires {} (current compiler: {}).",
            exercise.required_compilers(),
            compiler.kind()
        ));
        term::print_info("Press 'n' to skip it, or restart cmetal with --compiler.");
        return false;
    }

    if !exercise.exists() {
        term::print_error(&format!(
            "Exercise file not found: {}",
            exercise.path.display()
        ));
        return false;
    }

    match exercise.verify(compiler, build_dir) {
        Ok(result) => {
            if result.success {
                term::print_success(&format!(
                    "{} compiled and ran successfully!",
                    exercise.name()
                ));
                if !result.output.is_empty() {
                    term::print_stage_output("Program", &result.output);
                }
                println!("\r");
                if let Ok(path) = exercise.reveal_solution() {
                    term::print_info(&format!(
                        "Official solution revealed: {} — compare it with yours!",
                        path.display()
                    ));
                }
                term::print_info("Press 'n' to move to the next exercise.");
                true
            } else {
                term::print_error(&format!(
                    "{} failed at stage: {}",
                    exercise.name(),
                    result.stage
                ));
                term::print_stage_output(result.stage, &result.output);
                false
            }
        }
        Err(e) => {
            term::print_error(&format!("Error verifying {}: {e}", exercise.name()));
            false
        }
    }
}
