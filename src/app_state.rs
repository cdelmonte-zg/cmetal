use crate::exercise::Exercise;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const STATE_FILE: &str = ".cmetal-state.txt";
/// Pre-rename workspaces (the project was called clings) used this
/// name; it is migrated transparently on first load.
const LEGACY_STATE_FILE: &str = ".clings-state.txt";

/// How many damaged progress files to keep before giving up on
/// preserving another. A learner who has hit this many corruptions has
/// a problem no backup will fix.
const MAX_DAMAGED_BACKUPS: u32 = 100;

pub struct AppState {
    state_path: PathBuf,
    pub exercises: Vec<Exercise>,
    done: HashSet<String>,
    pub current_index: usize,
}

impl AppState {
    /// Loads the learner's progress. Infallible by construction: a
    /// progress file that cannot be read is reported through `warn`
    /// and treated as no progress, never as a failure.
    ///
    /// `warn` is a parameter rather than a field the caller may read
    /// afterwards: a warning nobody prints is a learner who silently
    /// lost their progress, so constructing a state without deciding
    /// where those warnings go must not be possible. This module does
    /// no output of its own, which also keeps `load` testable without
    /// capturing a terminal.
    pub fn new(exercises: Vec<Exercise>, base_dir: &Path, mut warn: impl FnMut(&str)) -> Self {
        let state_path = base_dir.join(STATE_FILE);
        let legacy = base_dir.join(LEGACY_STATE_FILE);
        if !state_path.exists() && legacy.exists() {
            // Progress from a pre-rename workspace: adopt it.
            let _ = std::fs::rename(&legacy, &state_path);
        }
        let mut state = Self {
            state_path,
            exercises,
            done: HashSet::new(),
            current_index: 0,
        };
        state.load(&mut warn);
        state
    }

    /// Reads the progress file, tolerating a damaged one.
    ///
    /// An unreadable state file is lost progress, not a fatal error.
    /// Making it fatal locked the learner out of every command — including
    /// `reset`, whose whole job is to clear this file — leaving no way out
    /// but deleting it by hand. Warn, start from empty, and let the next
    /// save rewrite it.
    fn load(&mut self, warn: &mut impl FnMut(&str)) {
        if !self.state_path.exists() {
            return;
        }

        let content = match std::fs::read_to_string(&self.state_path) {
            Ok(content) => content,
            Err(e) => {
                self.preserve_damaged(e, warn);
                return;
            }
        };

        let mut lines = content.lines();

        // Skip header comment
        lines.next();
        // Skip blank line
        lines.next();

        // Current exercise name
        if let Some(current_name) = lines.next() {
            let current_name = current_name.trim();
            if !current_name.is_empty() {
                if let Some(idx) = self.exercises.iter().position(|e| e.name() == current_name) {
                    self.current_index = idx;
                }
            }
        }

        // Skip blank line
        lines.next();

        // Done exercises
        for line in lines {
            let name = line.trim();
            if !name.is_empty() {
                self.done.insert(name.to_string());
            }
        }
    }

    /// Moves a progress file we could not read aside before anything
    /// overwrites it.
    ///
    /// Corruption is usually partial — a few mangled bytes in a file
    /// listing twenty solved exercises still reads as invalid UTF-8,
    /// and the next save would replace all of it with one entry. The
    /// copy costs a rename and leaves the learner something to salvage.
    fn preserve_damaged(&self, error: std::io::Error, warn: &mut impl FnMut(&str)) {
        let kept = self
            .free_backup_path()
            .filter(|kept| std::fs::rename(&self.state_path, kept).is_ok());
        let fate = match &kept {
            Some(kept) => format!("kept it as {} and started", kept.display()),
            None => "started".to_string(),
        };
        warn(&format!(
            "Could not read {} ({error}) — {fate} from empty progress.",
            self.state_path.display()
        ));
    }

    /// The first unused `.damaged` name.
    ///
    /// Never overwrite an existing backup: a second corruption would
    /// otherwise destroy the record taken for the first, which is the
    /// older and usually richer one — the exact loss this whole
    /// mechanism exists to prevent.
    fn free_backup_path(&self) -> Option<PathBuf> {
        let first = self.state_path.with_extension("txt.damaged");
        if !first.exists() {
            return Some(first);
        }
        (1..MAX_DAMAGED_BACKUPS)
            .map(|n| self.state_path.with_extension(format!("txt.damaged.{n}")))
            .find(|candidate| !candidate.exists())
    }

    /// Damaged progress files left in the workspace, so commands that
    /// claim to have cleaned up can say what they did not remove.
    pub fn damaged_backups(&self) -> Vec<PathBuf> {
        let Some(dir) = self.state_path.parent() else {
            return Vec::new();
        };
        let prefix = format!("{STATE_FILE}.damaged");
        let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with(&prefix))
            })
            .collect();
        found.sort();
        found
    }

    pub fn save(&self) -> Result<()> {
        let current_name = self
            .exercises
            .get(self.current_index)
            .map(|e| e.name())
            .unwrap_or("");

        let mut content = String::new();
        content.push_str("DON'T EDIT THIS FILE!\n");
        content.push('\n');
        content.push_str(current_name);
        content.push('\n');
        content.push('\n');

        for name in &self.done {
            content.push_str(name);
            content.push('\n');
        }

        std::fs::write(&self.state_path, content).with_context(|| "Failed to write state file")?;
        Ok(())
    }

    pub fn current_exercise(&self) -> Option<&Exercise> {
        self.exercises.get(self.current_index)
    }

    pub fn is_done(&self, name: &str) -> bool {
        self.done.contains(name)
    }

    pub fn mark_done(&mut self, name: &str) {
        self.done.insert(name.to_string());
    }

    pub fn mark_pending(&mut self, name: &str) {
        self.done.remove(name);
    }

    /// Records a verified completion and persists it immediately —
    /// marking without saving is how completions get lost.
    pub fn complete(&mut self, name: &str) -> Result<()> {
        self.mark_done(name);
        self.save()
    }

    pub fn prev(&mut self) -> bool {
        if self.current_index > 0 {
            self.current_index -= 1;
            true
        } else {
            false
        }
    }

    pub fn next_pending(&mut self) -> bool {
        let start = self.current_index;
        let len = self.exercises.len();

        for i in 1..=len {
            let idx = (start + i) % len;
            let ex = &self.exercises[idx];
            if ex.supported && !self.done.contains(ex.name()) {
                self.current_index = idx;
                return true;
            }
        }
        false
    }

    /// Exercises that don't support the selected compiler don't count:
    /// they can't be completed in this run, so they must not block the
    /// final message nor inflate the progress total.
    pub fn all_done(&self) -> bool {
        self.exercises
            .iter()
            .filter(|e| e.supported)
            .all(|e| self.done.contains(e.name()))
    }

    pub fn progress(&self) -> (usize, usize) {
        let supported: Vec<_> = self.exercises.iter().filter(|e| e.supported).collect();
        let done = supported
            .iter()
            .filter(|e| self.done.contains(e.name()))
            .count();
        (done, supported.len())
    }

    fn find_exercise(&self, name: &str) -> Option<usize> {
        self.exercises.iter().position(|e| e.name() == name)
    }

    /// Resolves the exercise a command should act on: the one named, or
    /// the current one when the argument is omitted. Every name-taking
    /// command shares this so they agree on both the default and the
    /// error message.
    pub fn resolve(&self, name: Option<String>) -> Result<usize> {
        let name = name.unwrap_or_else(|| {
            self.current_exercise()
                .map(|e| e.name().to_string())
                .unwrap_or_default()
        });
        self.find_exercise(&name)
            .with_context(|| crate::errors::exercise_not_found(&name))
    }

    /// Builds a state directly from parts, bypassing the on-disk
    /// progress file. Test-only: lets other modules' tests exercise
    /// logic that reads a state without staging a workspace.
    #[cfg(test)]
    pub(crate) fn from_parts(exercises: Vec<Exercise>, base_dir: &Path) -> Self {
        Self {
            state_path: base_dir.join(STATE_FILE),
            exercises,
            done: HashSet::new(),
            current_index: 0,
        }
    }

    pub fn reset(&self) -> Result<()> {
        if self.state_path.exists() {
            std::fs::remove_file(&self.state_path)
                .with_context(|| "Failed to remove state file")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::info_file::ExerciseInfo;

    fn make_info(name: &str) -> ExerciseInfo {
        ExerciseInfo {
            name: name.to_string(),
            dir: "test".to_string(),
            test: false,
            sanitizers: false,
            flags: Vec::new(),
            compilers: None,
            hint: None,
            hints: None,
        }
    }

    fn make_exercise(name: &str) -> Exercise {
        let info = make_info(name);
        Exercise {
            path: PathBuf::from(format!("/tmp/{name}.c")),
            solution_path: PathBuf::from(format!("/tmp/{name}_sol.c")),
            reveal_path: PathBuf::from(format!("/tmp/{name}_revealed.c")),
            supported: true,
            info,
        }
    }

    fn make_unsupported_exercise(name: &str) -> Exercise {
        let mut ex = make_exercise(name);
        ex.supported = false;
        ex
    }

    fn make_state(names: &[&str], base_dir: &Path) -> AppState {
        let exercises: Vec<Exercise> = names.iter().map(|n| make_exercise(n)).collect();
        AppState::from_parts(exercises, base_dir)
    }

    // --- Compiler support ---

    #[test]
    fn unsupported_exercises_skipped_and_uncounted() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut state = make_state(&["a", "c"], tmp.path());
        state
            .exercises
            .insert(1, make_unsupported_exercise("b_gcc_only"));

        // progress total ignores the unsupported exercise
        assert_eq!(state.progress(), (0, 2));

        // navigation skips it
        state.current_index = 0;
        assert!(state.next_pending());
        assert_eq!(state.current_exercise().unwrap().name(), "c");

        // all_done ignores it
        state.mark_done("a");
        state.mark_done("c");
        assert!(state.all_done());
        assert_eq!(state.progress(), (2, 2));
    }

    // --- Navigation ---

    #[test]
    fn prev_at_zero_returns_false() {
        let tmp = tempfile::tempdir().unwrap();
        let mut state = make_state(&["a", "b", "c"], tmp.path());
        assert!(!state.prev());
        assert_eq!(state.current_index, 0);
    }

    #[test]
    fn prev_moves_back() {
        let tmp = tempfile::tempdir().unwrap();
        let mut state = make_state(&["a", "b", "c", "d"], tmp.path());
        state.current_index = 3;
        assert!(state.prev());
        assert_eq!(state.current_index, 2);
    }

    #[test]
    fn next_pending_skips_done() {
        let tmp = tempfile::tempdir().unwrap();
        let mut state = make_state(&["a", "b", "c"], tmp.path());
        state.mark_done("b");
        // at index 0, next should skip b (done) and go to c
        assert!(state.next_pending());
        assert_eq!(state.current_index, 2);
    }

    #[test]
    fn next_pending_wraps_around() {
        let tmp = tempfile::tempdir().unwrap();
        let mut state = make_state(&["a", "b", "c"], tmp.path());
        state.current_index = 2;
        state.mark_done("a");
        state.mark_done("c");
        // from index 2, only b is pending → wraps to index 1
        assert!(state.next_pending());
        assert_eq!(state.current_index, 1);
    }

    #[test]
    fn next_pending_all_done_returns_false() {
        let tmp = tempfile::tempdir().unwrap();
        let mut state = make_state(&["a", "b"], tmp.path());
        state.mark_done("a");
        state.mark_done("b");
        assert!(!state.next_pending());
    }

    // --- State tracking ---

    #[test]
    fn mark_done_and_is_done() {
        let tmp = tempfile::tempdir().unwrap();
        let mut state = make_state(&["a", "b"], tmp.path());
        assert!(!state.is_done("a"));
        state.mark_done("a");
        assert!(state.is_done("a"));
        assert!(!state.is_done("b"));
    }

    #[test]
    fn all_done_false_then_true() {
        let tmp = tempfile::tempdir().unwrap();
        let mut state = make_state(&["a", "b"], tmp.path());
        assert!(!state.all_done());
        state.mark_done("a");
        assert!(!state.all_done());
        state.mark_done("b");
        assert!(state.all_done());
    }

    #[test]
    fn progress_counts() {
        let tmp = tempfile::tempdir().unwrap();
        let mut state = make_state(&["a", "b", "c"], tmp.path());
        assert_eq!(state.progress(), (0, 3));
        state.mark_done("a");
        assert_eq!(state.progress(), (1, 3));
        state.mark_done("c");
        assert_eq!(state.progress(), (2, 3));
    }

    #[test]
    fn find_exercise_found_and_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let state = make_state(&["alpha", "beta"], tmp.path());
        assert_eq!(state.find_exercise("beta"), Some(1));
        assert_eq!(state.find_exercise("gamma"), None);
    }

    #[test]
    fn current_exercise_valid() {
        let tmp = tempfile::tempdir().unwrap();
        let state = make_state(&["a", "b"], tmp.path());
        assert_eq!(state.current_exercise().unwrap().name(), "a");
    }

    // --- Persistence ---

    #[test]
    fn save_and_reload_preserves_state() {
        let tmp = tempfile::tempdir().unwrap();
        let mut state = make_state(&["a", "b", "c"], tmp.path());
        state.current_index = 1;
        state.mark_done("a");
        state.save().unwrap();

        let mut warnings = Vec::new();
        let state2 = AppState::new(
            vec![make_exercise("a"), make_exercise("b"), make_exercise("c")],
            tmp.path(),
            |w| warnings.push(w.to_string()),
        );
        assert!(warnings.is_empty(), "a healthy file warns nothing");
        assert_eq!(state2.current_index, 1);
        assert!(state2.is_done("a"));
        assert!(!state2.is_done("b"));
    }

    #[test]
    fn reset_removes_state_file() {
        let tmp = tempfile::tempdir().unwrap();
        let mut state = make_state(&["a"], tmp.path());
        state.mark_done("a");
        state.save().unwrap();
        assert!(tmp.path().join(STATE_FILE).exists());

        state.reset().unwrap();
        assert!(!tmp.path().join(STATE_FILE).exists());
    }

    #[test]
    fn reset_no_file_is_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let state = make_state(&["a"], tmp.path());
        assert!(state.reset().is_ok());
    }

    // --- Name resolution shared by run/hint/solution ---

    #[test]
    fn resolve_defaults_to_the_current_exercise() {
        let tmp = tempfile::tempdir().unwrap();
        let mut state = make_state(&["a", "b", "c"], tmp.path());
        state.current_index = 2;
        assert_eq!(state.resolve(None).unwrap(), 2);
    }

    #[test]
    fn resolve_finds_by_name_regardless_of_current() {
        let tmp = tempfile::tempdir().unwrap();
        let mut state = make_state(&["a", "b", "c"], tmp.path());
        state.current_index = 0;
        assert_eq!(state.resolve(Some("b".to_string())).unwrap(), 1);
    }

    #[test]
    fn resolve_reports_the_missing_name() {
        let tmp = tempfile::tempdir().unwrap();
        let state = make_state(&["a"], tmp.path());
        let err = state.resolve(Some("nope".to_string())).unwrap_err();
        assert_eq!(err.to_string(), "Exercise 'nope' not found");
    }
}
