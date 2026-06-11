use crate::exercise::Exercise;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const STATE_FILE: &str = ".clings-state.txt";

pub struct AppState {
    state_path: PathBuf,
    pub exercises: Vec<Exercise>,
    done: HashSet<String>,
    pub current_index: usize,
}

impl AppState {
    pub fn new(exercises: Vec<Exercise>, base_dir: &Path) -> Result<Self> {
        let state_path = base_dir.join(STATE_FILE);
        let mut state = Self {
            state_path,
            exercises,
            done: HashSet::new(),
            current_index: 0,
        };
        state.load()?;
        Ok(state)
    }

    fn load(&mut self) -> Result<()> {
        if !self.state_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.state_path)
            .with_context(|| "Failed to read state file")?;

        let mut lines = content.lines();

        // Skip header comment
        lines.next();
        // Skip blank line
        lines.next();

        // Current exercise name
        if let Some(current_name) = lines.next() {
            let current_name = current_name.trim();
            if !current_name.is_empty() {
                if let Some(idx) = self
                    .exercises
                    .iter()
                    .position(|e| e.name() == current_name)
                {
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

        Ok(())
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
            if !self.done.contains(self.exercises[idx].name()) {
                self.current_index = idx;
                return true;
            }
        }
        false
    }

    pub fn all_done(&self) -> bool {
        self.exercises.iter().all(|e| self.done.contains(e.name()))
    }

    pub fn progress(&self) -> (usize, usize) {
        (self.done.len(), self.exercises.len())
    }

    pub fn find_exercise(&self, name: &str) -> Option<usize> {
        self.exercises.iter().position(|e| e.name() == name)
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
            info,
        }
    }

    fn make_state(names: &[&str], base_dir: &Path) -> AppState {
        let exercises: Vec<Exercise> = names.iter().map(|n| make_exercise(n)).collect();
        AppState {
            state_path: base_dir.join(STATE_FILE),
            exercises,
            done: HashSet::new(),
            current_index: 0,
        }
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

        let state2 = AppState::new(
            vec![make_exercise("a"), make_exercise("b"), make_exercise("c")],
            tmp.path(),
        )
        .unwrap();
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
}
