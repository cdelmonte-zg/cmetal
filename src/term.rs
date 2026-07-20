use crossterm::style::{Attribute, Color, SetAttribute, SetForegroundColor};
use std::io;

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
pub fn warn_stderr(msg: &str) {
    let mut stderr = io::stderr();
    let _ = crossterm::execute!(
        stderr,
        SetForegroundColor(Color::Yellow),
        SetAttribute(Attribute::Bold)
    );
    eprint!("  ⚠ ");
    let _ = crossterm::execute!(stderr, SetAttribute(Attribute::Reset));
    eprintln!("{msg}\r");
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

pub fn clear_screen() {
    let _ = crossterm::execute!(
        io::stdout(),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        crossterm::cursor::MoveTo(0, 0)
    );
}
