//! Error text shared by more than one module.
//!
//! A message that two modules must agree on does not belong to either
//! of them: putting it in one makes the other depend on it for
//! presentation, which is the wrong direction. This is where those
//! messages live instead.

/// Reported when a name does not match any exercise, whether it was
/// resolved against `info.toml` or against the learner's progress.
pub fn exercise_not_found(name: &str) -> String {
    format!("Exercise '{name}' not found")
}
