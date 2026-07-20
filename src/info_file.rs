use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct InfoFile {
    pub format_version: u32,
    #[serde(default)]
    #[allow(dead_code)]
    pub default_compiler: Option<String>,
    #[serde(default)]
    pub welcome_message: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub final_message: Option<String>,
    pub exercises: Vec<ExerciseInfo>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExerciseInfo {
    pub name: String,
    pub dir: String,
    #[serde(default = "default_true")]
    pub test: bool,
    #[serde(default)]
    pub sanitizers: bool,
    /// Extra compiler flags for this exercise (e.g. ["-O2", "-Wstrict-aliasing=2"])
    #[serde(default)]
    pub flags: Vec<String>,
    /// Compilers this exercise works with (e.g. ["gcc"]).
    /// Absent = works with every compiler.
    #[serde(default)]
    pub compilers: Option<Vec<String>>,
    /// Single hint (legacy format, still supported)
    #[serde(default)]
    pub hint: Option<String>,
    /// Progressive hints array (new format)
    #[serde(default)]
    pub hints: Option<Vec<String>>,
}

impl InfoFile {
    /// Looks up an exercise by name, without consulting any progress
    /// state. The compiler-free diagnostic commands use this so a
    /// damaged `.cmetal-state.txt` cannot stop them from answering a
    /// question about a named exercise.
    pub fn find(&self, name: &str) -> Result<&ExerciseInfo> {
        self.exercises
            .iter()
            .find(|ei| ei.name == name)
            .with_context(|| crate::errors::exercise_not_found(name))
    }
}

impl ExerciseInfo {
    /// Path of this exercise's source file relative to the exercises
    /// root — the single place the `<dir>/<name>.c` layout convention
    /// lives (my_exercises/ and solutions/ mirror it).
    pub fn rel_path(&self) -> std::path::PathBuf {
        Path::new(&self.dir).join(format!("{}.c", self.name))
    }
}

impl ExerciseInfo {
    /// Whether this exercise is meant to run with the given compiler.
    pub fn supports_compiler(&self, compiler: &str) -> bool {
        match &self.compilers {
            None => true,
            Some(list) => list.iter().any(|c| c.eq_ignore_ascii_case(compiler)),
        }
    }

    /// Get all hints as a slice, merging both formats.
    /// If `hints` array is set, use that. Otherwise wrap `hint` string.
    pub fn get_hints(&self) -> Vec<&str> {
        if let Some(hints) = &self.hints {
            hints.iter().map(|s| s.as_str()).collect()
        } else if let Some(hint) = &self.hint {
            if hint.is_empty() {
                vec![]
            } else {
                vec![hint.as_str()]
            }
        } else {
            vec![]
        }
    }
}

impl InfoFile {
    pub fn parse_str(content: &str) -> Result<Self> {
        let info: InfoFile = toml::from_str(content).with_context(|| "Failed to parse TOML")?;
        if info.format_version != 1 {
            anyhow::bail!(
                "Unsupported info.toml format_version: {}",
                info.format_version
            );
        }
        Ok(info)
    }

    pub fn parse(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        Self::parse_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_toml() -> &'static str {
        r#"
format_version = 1
[[exercises]]
name = "ex1"
dir = "00_intro"
"#
    }

    fn full_toml() -> &'static str {
        r#"
format_version = 1
default_compiler = "gcc"
welcome_message = "Welcome!"
final_message = "Done!"

[[exercises]]
name = "ex1"
dir = "00_intro"
test = true
sanitizers = true
flags = ["-O2", "-Wstrict-aliasing=2"]
hints = ["hint1", "hint2"]

[[exercises]]
name = "ex2"
dir = "01_pointers"
test = false
sanitizers = false
hint = "legacy hint"
"#
    }

    #[test]
    fn parse_minimal_toml() {
        let info = InfoFile::parse_str(minimal_toml()).unwrap();
        assert_eq!(info.format_version, 1);
        assert_eq!(info.exercises.len(), 1);
        assert_eq!(info.exercises[0].name, "ex1");
        assert_eq!(info.exercises[0].dir, "00_intro");
    }

    #[test]
    fn parse_full_toml() {
        let info = InfoFile::parse_str(full_toml()).unwrap();
        assert_eq!(info.exercises.len(), 2);
        assert_eq!(info.welcome_message.as_deref(), Some("Welcome!"));
        assert_eq!(info.final_message.as_deref(), Some("Done!"));
        assert!(info.exercises[0].test);
        assert!(info.exercises[0].sanitizers);
        assert!(!info.exercises[1].test);
        assert!(!info.exercises[1].sanitizers);
    }

    #[test]
    fn defaults_test_true_sanitizers_false() {
        let info = InfoFile::parse_str(minimal_toml()).unwrap();
        let ex = &info.exercises[0];
        assert!(ex.test);
        assert!(!ex.sanitizers);
        assert!(ex.flags.is_empty());
    }

    #[test]
    fn parse_flags() {
        let info = InfoFile::parse_str(full_toml()).unwrap();
        assert_eq!(info.exercises[0].flags, vec!["-O2", "-Wstrict-aliasing=2"]);
        assert!(info.exercises[1].flags.is_empty());
    }

    #[test]
    fn supports_compiler_default_all() {
        let info = InfoFile::parse_str(minimal_toml()).unwrap();
        assert!(info.exercises[0].supports_compiler("gcc"));
        assert!(info.exercises[0].supports_compiler("clang"));
    }

    #[test]
    fn supports_compiler_restricted() {
        let toml = r#"
format_version = 1
[[exercises]]
name = "ex1"
dir = "00_intro"
compilers = ["gcc"]
"#;
        let info = InfoFile::parse_str(toml).unwrap();
        assert!(info.exercises[0].supports_compiler("gcc"));
        assert!(info.exercises[0].supports_compiler("GCC"));
        assert!(!info.exercises[0].supports_compiler("clang"));
    }

    #[test]
    fn reject_wrong_format_version() {
        let toml = r#"
format_version = 99
[[exercises]]
name = "ex1"
dir = "00_intro"
"#;
        let result = InfoFile::parse_str(toml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("format_version"));
    }

    #[test]
    fn get_hints_with_hints_array() {
        let info = InfoFile::parse_str(full_toml()).unwrap();
        let hints = info.exercises[0].get_hints();
        assert_eq!(hints, vec!["hint1", "hint2"]);
    }

    #[test]
    fn get_hints_with_legacy_hint() {
        let info = InfoFile::parse_str(full_toml()).unwrap();
        let hints = info.exercises[1].get_hints();
        assert_eq!(hints, vec!["legacy hint"]);
    }

    #[test]
    fn get_hints_no_hints() {
        let info = InfoFile::parse_str(minimal_toml()).unwrap();
        let hints = info.exercises[0].get_hints();
        assert!(hints.is_empty());
    }

    #[test]
    fn get_hints_empty_hint_string() {
        let toml = r#"
format_version = 1
[[exercises]]
name = "ex1"
dir = "00_intro"
hint = ""
"#;
        let info = InfoFile::parse_str(toml).unwrap();
        assert!(info.exercises[0].get_hints().is_empty());
    }
}
