//! Two warning channels, with a rule for choosing:
//!
//! - [`print_warning`] for warnings that are part of a command's own
//!   report, where the reader is looking at that command's output;
//! - [`warn_stderr`] for diagnostics emitted while starting up, which
//!   any command can produce and which must not end up inside the
//!   output someone is piping from `cmetal list`.

use anyhow::Context;
use crossterm::style::{Attribute, Color, SetAttribute, SetForegroundColor};
use std::io::{self, BufRead, IsTerminal, Write};

pub fn print_success(msg: &str) -> anyhow::Result<()> {
    let mut stdout = io::stdout();
    let _ = crossterm::execute!(
        stdout,
        SetForegroundColor(Color::Green),
        SetAttribute(Attribute::Bold)
    );
    write_stdout("  ✓ ")?;
    let _ = crossterm::execute!(stdout, SetAttribute(Attribute::Reset));
    write_stdout(&format!("{msg}\r\n"))?;

    Ok(())
}

pub fn print_error(msg: &str) -> anyhow::Result<()> {
    let mut stdout = io::stdout();
    let _ = crossterm::execute!(
        stdout,
        SetForegroundColor(Color::Red),
        SetAttribute(Attribute::Bold)
    );
    write_stdout("  ✗ ")?;
    let _ = crossterm::execute!(stdout, SetAttribute(Attribute::Reset));
    write_stdout(&format!("{msg}\r\n"))?;

    Ok(())
}

pub fn print_warning(msg: &str) -> anyhow::Result<()> {
    let mut stdout = io::stdout();
    let _ = crossterm::execute!(
        stdout,
        SetForegroundColor(Color::Yellow),
        SetAttribute(Attribute::Bold)
    );
    write_stdout("  ⚠ ")?;
    let _ = crossterm::execute!(stdout, SetAttribute(Attribute::Reset));
    write_stdout(&format!("{msg}\r\n"))?;

    Ok(())
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

pub fn print_info(msg: &str) -> anyhow::Result<()> {
    let mut stdout = io::stdout();
    let _ = crossterm::execute!(stdout, SetForegroundColor(Color::Cyan));
    write_stdout("  ℹ ")?;
    let _ = crossterm::execute!(stdout, SetAttribute(Attribute::Reset));
    write_stdout(&format!("{msg}\r\n"))?;

    Ok(())
}

pub fn print_header(msg: &str) -> anyhow::Result<()> {
    let mut stdout = io::stdout();
    let _ = crossterm::execute!(
        stdout,
        SetForegroundColor(Color::Magenta),
        SetAttribute(Attribute::Bold)
    );
    write_stdout(&format!("\r\n  {msg}\r\n"))?;
    let _ = crossterm::execute!(stdout, SetAttribute(Attribute::Reset));

    Ok(())
}

pub fn print_progress(done: usize, total: usize) -> anyhow::Result<()> {
    let mut stdout = io::stdout();
    let bar_width = 30;
    let filled = (done * bar_width).checked_div(total).unwrap_or(0);
    let empty = bar_width - filled;

    let _ = crossterm::execute!(stdout, SetForegroundColor(Color::Cyan));
    write_stdout("  Completed: [")?;

    let _ = crossterm::execute!(stdout, SetForegroundColor(Color::Green));
    write_stdout("█".repeat(filled).as_str())?;

    let _ = crossterm::execute!(stdout, SetForegroundColor(Color::DarkGrey));
    write_stdout("░".repeat(empty).as_str())?;

    let _ = crossterm::execute!(stdout, SetForegroundColor(Color::Cyan));
    write_stdout(&format!("] {done}/{total}\r\n"))?;

    let _ = crossterm::execute!(stdout, SetAttribute(Attribute::Reset));

    Ok(())
}

pub fn print_stage_output(stage: &str, output: &str) -> anyhow::Result<()> {
    if !output.is_empty() {
        let mut stdout = io::stdout();

        let _ = crossterm::execute!(stdout, SetForegroundColor(Color::DarkGrey));
        write_stdout(&format!("\r\n  ── {stage} output ──\r\n"))?;

        let _ = crossterm::execute!(stdout, SetAttribute(Attribute::Reset));
        for line in output.lines() {
            write_stdout(&format!("  {line}\r\n"))?;
        }
    }

    Ok(())
}

pub fn write_stdout(text: &str) -> anyhow::Result<()> {
    match io::stdout().write_all(text.as_bytes()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(e).context("Failed to write to stdout"),
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
pub fn confirm(question: &str) -> anyhow::Result<bool> {
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
