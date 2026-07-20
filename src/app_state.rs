use crate::exercise::Exercise;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

const STATE_FILE: &str = ".cmetal-state.txt";
/// Pre-rename workspaces (the project was called clings) used this
/// name; it is migrated transparently on first load.
const LEGACY_STATE_FILE: &str = ".clings-state.txt";

/// Distinct `.damaged` names to try: the unsuffixed one plus `.1`
/// through `.99`.
const MAX_DAMAGED_BACKUPS: u32 = 100;

/// Base name for preserved copies of an unreadable progress file.
/// Both the writer and the reader derive from here, so they cannot
/// disagree if STATE_FILE changes.
static DAMAGED_PREFIX: LazyLock<String> = LazyLock::new(|| format!("{STATE_FILE}.damaged"));

/// What happened to the progress file on disk.
#[derive(PartialEq, Eq)]
enum ProgressFile {
    /// Read successfully, or not there at all.
    Loaded,
    /// Exists but could not be read. The original is still on disk and
    /// must be preserved before anything writes over it.
    Unreadable,
    /// Was unreadable and has been moved aside. Behaves like `Loaded`;
    /// it is a distinct state so that leaving `Unreadable` is a
    /// transition the type records rather than a flag to clear.
    Preserved,
}

pub struct AppState {
    state_path: PathBuf,
    pub exercises: Vec<Exercise>,
    done: HashSet<String>,
    pub current_index: usize,
    progress_file: ProgressFile,
}

impl AppState {
    /// Loads the learner's progress. Infallible: a progress file that
    /// cannot be read is reported through `warn` and treated as no
    /// progress, never as a failure.
    ///
    /// `warn` is a parameter so that a state cannot be constructed
    /// without deciding where its warnings go, and so this module does
    /// no output of its own.
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
            progress_file: ProgressFile::Loaded,
        };
        state.load(&mut warn);
        state
    }

    /// Reads the progress file, tolerating a damaged one: unreadable
    /// progress is lost progress, not a fatal error, or a broken file
    /// would lock the learner out of every command including `reset`.
    fn load(&mut self, warn: &mut impl FnMut(&str)) {
        if !self.state_path.exists() {
            return;
        }

        let content = match std::fs::read_to_string(&self.state_path) {
            Ok(content) => content,
            Err(e) => {
                // Left in place: read-only commands must leave a
                // broken workspace as they found it. `save` and
                // `reset` preserve it before they overwrite it.
                self.progress_file = ProgressFile::Unreadable;
                warn(&format!(
                    "Could not read {} ({e}) — starting from empty progress. \
                     The file is kept: delete it yourself, or it is moved \
                     aside as a .damaged copy the first time progress is \
                     saved.",
                    self.state_path.display()
                ));
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

    /// Moves an unreadable progress file aside so the caller can write
    /// over `state_path` without destroying the only copy of it.
    /// Corruption is usually partial, so the rename leaves the learner
    /// something to salvage. A no-op unless the file is `Unreadable`.
    fn preserve_if_unreadable(&mut self) -> Result<()> {
        if self.progress_file != ProgressFile::Unreadable {
            return Ok(());
        }
        let kept = self.free_backup_path().with_context(|| {
            format!(
                "Refusing to destroy {}: it could not be read and there is no \
                 free .damaged name left to move it to. Remove some of them, \
                 or delete the file yourself.",
                self.state_path.display()
            )
        })?;
        std::fs::rename(&self.state_path, &kept).with_context(|| {
            format!(
                "Refusing to destroy {}: it could not be read and could not be \
                 moved to {}.",
                self.state_path.display(),
                kept.display()
            )
        })?;
        // Only on success: a failed rename must be retried.
        self.progress_file = ProgressFile::Preserved;
        Ok(())
    }

    /// The first unused `.damaged` name. Never overwrites an existing
    /// backup: the older record is the one worth keeping.
    fn free_backup_path(&self) -> Option<PathBuf> {
        let dir = self.state_path.parent()?;
        let prefix = &*DAMAGED_PREFIX;
        std::iter::once(dir.join(prefix))
            .chain((1..MAX_DAMAGED_BACKUPS).map(|n| dir.join(format!("{prefix}.{n}"))))
            .find(|candidate| !candidate.exists())
    }

    /// Damaged progress files left in the workspace, so commands that
    /// claim to have cleaned up can say what they did not remove.
    pub fn damaged_backups(&self) -> Vec<PathBuf> {
        let Some(dir) = self.state_path.parent() else {
            return Vec::new();
        };
        let prefix = &*DAMAGED_PREFIX;
        // A workspace we cannot list simply has no backups to report.
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut found: Vec<(u32, PathBuf)> = entries
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                let name = path.file_name()?.to_str()?.to_owned();
                let suffix = name.strip_prefix(prefix.as_str())?;
                // Order by the number the names promise, not by their
                // bytes: lexicographically `.10` sorts before `.2`.
                let n = match suffix {
                    "" => 0,
                    rest => rest.strip_prefix('.')?.parse().ok()?,
                };
                Some((n, path))
            })
            .collect();
        found.sort();
        found.into_iter().map(|(_, path)| path).collect()
    }

    pub fn save(&mut self) -> Result<()> {
        self.preserve_if_unreadable()?;

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
            progress_file: ProgressFile::Loaded,
        }
    }

    pub fn reset(&mut self) -> Result<()> {
        if self.state_path.exists() {
            // An unreadable file is moved aside rather than deleted:
            // the learner asked to clear their progress, not to destroy
            // the only copy of a record they may still want to read.
            if self.progress_file == ProgressFile::Unreadable {
                self.preserve_if_unreadable()?;
            } else {
                std::fs::remove_file(&self.state_path)
                    .with_context(|| "Failed to remove state file")?;
            }
        }
        // The file is gone, so the object must not still hold it, or a
        // later save writes the cleared progress back.
        self.done.clear();
        self.current_index = 0;
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
        let mut state = make_state(&["a"], tmp.path());
        assert!(state.reset().is_ok());
    }

    // --- Name resolution shared by run/hint/solution ---

    // --- Damaged progress files ---

    fn write_damaged(dir: &Path) {
        std::fs::write(dir.join(STATE_FILE), [0xff, 0xfe, 0x00, b'x']).unwrap();
    }

    /// Backup names as the production code reports them, so the tests
    /// pin what a learner is actually told rather than a second
    /// implementation of the same scan.
    fn damaged_copies(state: &AppState) -> Vec<String> {
        state
            .damaged_backups()
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect()
    }

    /// The warning must recommend no cmetal command at all: the tool
    /// cannot know which are safe for the state the learner is in, so
    /// the only remedies it offers here are inert ones.
    #[test]
    fn the_damaged_file_warning_recommends_nothing_destructive() {
        let tmp = tempfile::tempdir().unwrap();
        write_damaged(tmp.path());
        let mut warnings = Vec::new();
        let _ = AppState::new(vec![make_exercise("a")], tmp.path(), |w| {
            warnings.push(w.to_string())
        });

        assert_eq!(warnings.len(), 1);
        assert!(
            !warnings[0].contains("cmetal "),
            "the warning must not send the learner to run anything: {}",
            warnings[0]
        );
    }

    /// Preserved once, not once per save — watch mode saves on every
    /// navigation keypress.
    #[test]
    fn repeated_saves_preserve_only_once() {
        let tmp = tempfile::tempdir().unwrap();
        write_damaged(tmp.path());
        let mut state = AppState::new(vec![make_exercise("a")], tmp.path(), |_| {});

        state.save().unwrap();
        state.save().unwrap();
        state.save().unwrap();

        assert_eq!(
            damaged_copies(&state),
            vec![".cmetal-state.txt.damaged".to_string()],
            "only the unreadable file belongs in a .damaged copy"
        );
    }

    /// Loading must not move anything: `list`, `hint` and `diff` are
    /// read-only and have to leave a broken workspace as they found it.
    #[test]
    fn loading_leaves_the_damaged_file_in_place() {
        let tmp = tempfile::tempdir().unwrap();
        write_damaged(tmp.path());
        let mut warnings = Vec::new();
        let state = AppState::new(vec![make_exercise("a")], tmp.path(), |w| {
            warnings.push(w.to_string())
        });

        assert_eq!(warnings.len(), 1, "the learner must be told");
        assert!(
            warnings[0].contains("delete it"),
            "the warning must say how to clear it: {}",
            warnings[0]
        );
        assert!(damaged_copies(&state).is_empty());
        assert!(tmp.path().join(STATE_FILE).exists());
    }

    /// A second corruption must not overwrite the first record, which
    /// is the older and usually richer one.
    #[test]
    fn each_corruption_gets_its_own_backup() {
        let tmp = tempfile::tempdir().unwrap();
        let mut state = None;
        for _ in 0..2 {
            write_damaged(tmp.path());
            let mut fresh = AppState::new(vec![make_exercise("a")], tmp.path(), |_| {});
            fresh.save().unwrap();
            state = Some(fresh);
        }
        assert_eq!(
            damaged_copies(state.as_ref().unwrap()),
            vec![
                ".cmetal-state.txt.damaged".to_string(),
                ".cmetal-state.txt.damaged.1".to_string()
            ]
        );
    }

    /// Reset clears progress; it must not destroy the one copy of a
    /// record the learner may still want to read.
    #[test]
    fn reset_moves_an_unreadable_file_aside() {
        let tmp = tempfile::tempdir().unwrap();
        write_damaged(tmp.path());
        let mut state = AppState::new(vec![make_exercise("a")], tmp.path(), |_| {});

        state.reset().unwrap();

        assert!(!tmp.path().join(STATE_FILE).exists());
        assert_eq!(
            damaged_copies(&state),
            vec![".cmetal-state.txt.damaged".to_string()]
        );
    }

    /// Reset clears the object too, or the next save writes back the
    /// completions it just deleted.
    #[test]
    fn reset_clears_progress_in_memory() {
        let tmp = tempfile::tempdir().unwrap();
        let mut state = make_state(&["a", "b"], tmp.path());
        state.mark_done("a");
        state.current_index = 1;
        state.save().unwrap();

        state.reset().unwrap();

        assert!(!state.is_done("a"), "progress must not survive a reset");
        assert_eq!(state.current_index, 0);
        assert_eq!(state.progress(), (0, 2));

        // The proof that it matters: saving now must not resurrect it.
        // The file is header, blank, current exercise, blank, then one
        // line per completion — so the completions are what follows.
        state.save().unwrap();
        let written = std::fs::read_to_string(tmp.path().join(STATE_FILE)).unwrap();
        let completions: Vec<&str> = written
            .lines()
            .skip(4)
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        assert!(
            completions.is_empty(),
            "a save after reset wrote back the cleared progress: {completions:?}"
        );
    }

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
