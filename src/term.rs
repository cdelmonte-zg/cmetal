//! Two warning channels, with a rule for choosing:
//!
//! - [`print_warning`] for warnings that are part of a command's own
//!   report, where the reader is looking at that command's output;
//! - [`warn_stderr`] for diagnostics emitted while starting up, which
//!   any command can produce and which must not end up inside the
//!   output someone is piping from `cmetal list`.

use crossterm::style::{Attribute, Color, SetAttribute, SetForegroundColor};
use std::io::{self, IsTerminal};

pub fn print_success(msg: &str) {
    let mut stdout = io::stdout();
    let _ = crossterm::execute!(
        stdout,
        SetForegroundColor(Color::Green),
        SetAttribute(Attribute::Bold)
    );
    print!("  ✓ ");
    let _ = crossterm::execute!(stdout, SetAttribute(Attribute::Reset));
    println!("{msg}\r");
}

pub fn print_error(msg: &str) {
    let mut stdout = io::stdout();
    let _ = crossterm::execute!(
        stdout,
        SetForegroundColor(Color::Red),
        SetAttribute(Attribute::Bold)
    );
    print!("  ✗ ");
    let _ = crossterm::execute!(stdout, SetAttribute(Attribute::Reset));
    println!("{msg}\r");
}

pub fn print_warning(msg: &str) {
    let mut stdout = io::stdout();
    let _ = crossterm::execute!(
        stdout,
        SetForegroundColor(Color::Yellow),
        SetAttribute(Attribute::Bold)
    );
    print!("  ⚠ ");
    let _ = crossterm::execute!(stdout, SetAttribute(Attribute::Reset));
    println!("{msg}\r");
}

/// A warning that must not land in stdout: diagnostics about a broken
/// workspace would otherwise show up in the output of `cmetal list`
/// and anything piped from it.
/// A startup diagnostic, kept out of stdout.
///
/// Unlike the helpers above it emits no carriage return and no colour
/// unless stderr is a terminal: this never runs inside watch mode's
/// raw screen, and stderr is the stream people redirect into logs.
pub fn warn_stderr(msg: &str) {
    let mut stderr = io::stderr();
    if !stderr.is_terminal() {
        eprintln!("warning: {msg}");
        return;
    }
    let _ = crossterm::execute!(
        stderr,
        SetForegroundColor(Color::Yellow),
        SetAttribute(Attribute::Bold)
    );
    eprint!("  ⚠ ");
    let _ = crossterm::execute!(stderr, SetAttribute(Attribute::Reset));
    eprintln!("{msg}");
}

pub fn print_info(msg: &str) {
    let mut stdout = io::stdout();
    let _ = crossterm::execute!(stdout, SetForegroundColor(Color::Cyan));
    print!("  ℹ ");
    let _ = crossterm::execute!(stdout, SetAttribute(Attribute::Reset));
    println!("{msg}\r");
}

pub fn print_header(msg: &str) {
    let mut stdout = io::stdout();
    let _ = crossterm::execute!(
        stdout,
        SetForegroundColor(Color::Magenta),
        SetAttribute(Attribute::Bold)
    );
    println!("\r\n  {msg}\r");
    let _ = crossterm::execute!(stdout, SetAttribute(Attribute::Reset));
}

pub fn print_progress(done: usize, total: usize) {
    let mut stdout = io::stdout();
    let bar_width = 30;
    let filled = (done * bar_width).checked_div(total).unwrap_or(0);
    let empty = bar_width - filled;

    let _ = crossterm::execute!(stdout, SetForegroundColor(Color::Cyan));
    print!("  Completed: [");
    let _ = crossterm::execute!(stdout, SetForegroundColor(Color::Green));
    print!("{}", "█".repeat(filled));
    let _ = crossterm::execute!(stdout, SetForegroundColor(Color::DarkGrey));
    print!("{}", "░".repeat(empty));
    let _ = crossterm::execute!(stdout, SetForegroundColor(Color::Cyan));
    println!("] {done}/{total}\r");
    let _ = crossterm::execute!(stdout, SetAttribute(Attribute::Reset));
}

pub fn print_stage_output(stage: &str, output: &str) {
    if !output.is_empty() {
        let mut stdout = io::stdout();
        let _ = crossterm::execute!(stdout, SetForegroundColor(Color::DarkGrey));
        println!("\r\n  ── {stage} output ──\r");
        let _ = crossterm::execute!(stdout, SetAttribute(Attribute::Reset));
        for line in output.lines() {
            println!("  {line}\r");
        }
    }
}

/// Asks a yes/no question, defaulting to no.
///
/// Returns true without asking when stdin is not a terminal: a script
/// or CI run has no one to answer, and the command it typed is the
/// answer. Interactive callers get a real choice.
///
/// The question goes to stderr, not stdout: `cmetal reset | tee log`
/// still has a terminal on stdin, so a prompt written to stdout would
/// disappear into the pipe and leave the learner staring at a program
/// that looks hung.
pub fn confirm(question: &str) -> std::io::Result<bool> {
    use std::io::{BufRead, Write};
    if !io::stdin().is_terminal() {
        return Ok(true);
    }
    eprint!("  {question} [y/N] ");
    io::stderr().flush()?;
    let mut answer = String::new();
    io::stdin().lock().read_line(&mut answer)?;
    Ok(matches!(answer.trim().to_lowercase().as_str(), "y" | "yes"))
}

pub fn clear_screen() {
    let _ = crossterm::execute!(
        io::stdout(),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        crossterm::cursor::MoveTo(0, 0)
    );
}
