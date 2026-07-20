use anyhow::{Context, Result};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerKind {
    Gcc,
    Clang,
}

impl CompilerKind {
    pub fn command_name(self) -> &'static str {
        match self {
            CompilerKind::Gcc => "gcc",
            CompilerKind::Clang => "clang",
        }
    }
}

impl std::fmt::Display for CompilerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.command_name())
    }
}

pub struct CompileResult {
    pub success: bool,
    pub output: String,
}

pub struct Compiler {
    kind: CompilerKind,
    include_dir: PathBuf,
    /// Probed once at construction: spawning a compiler per
    /// base_args() call would cost a process per verify stage.
    supports_no_fixit_hints: bool,
}

impl Compiler {
    pub fn new(kind: CompilerKind, base_dir: &Path) -> Result<Self> {
        let name = kind.command_name();
        let status = Command::new(name)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .with_context(|| format!("{name} not found. Please install it."))?;

        if !status.success() {
            anyhow::bail!("{name} --version failed");
        }

        let supports_no_fixit_hints = probe_flag(kind, "-fno-diagnostics-show-fix-it-hints");

        Ok(Self {
            kind,
            include_dir: base_dir.join("include"),
            supports_no_fixit_hints,
        })
    }

    pub fn kind(&self) -> CompilerKind {
        self.kind
    }

    fn include_flag(&self) -> OsString {
        let mut flag = OsString::from("-I");
        flag.push(self.include_dir.as_os_str());
        flag
    }

    fn base_args(&self) -> Vec<OsString> {
        let mut args = vec![
            self.include_flag(),
            "-Wall".into(),
            "-Wextra".into(),
            "-Werror".into(),
            "-pedantic".into(),
            "-std=c11".into(),
            "-g".into(),
        ];
        if self.supports_no_fixit_hints {
            args.push("-fno-diagnostics-show-fix-it-hints".into());
        }
        args
    }

    pub fn compile(
        &self,
        source: &Path,
        output: &Path,
        extra_flags: &[String],
    ) -> Result<CompileResult> {
        let mut args = self.base_args();
        args.extend(extra_flags.iter().map(OsString::from));
        args.push("-o".into());
        args.push(output.into());
        args.push(source.into());

        self.run_compiler(&args)
    }

    pub fn compile_with_tests(
        &self,
        source: &Path,
        output: &Path,
        extra_flags: &[String],
    ) -> Result<CompileResult> {
        let mut args = self.base_args();
        args.extend(extra_flags.iter().map(OsString::from));
        args.push("-DTEST".into());
        args.push("-o".into());
        args.push(output.into());
        args.push(source.into());

        self.run_compiler(&args)
    }

    /// The sanitizer-stage flags; the drift-alarm test compares this
    /// exact construction against the CI checker's SAN_FLAGS.
    fn sanitizer_args(&self) -> Vec<OsString> {
        vec![
            self.include_flag(),
            "-fsanitize=address,undefined".into(),
            "-fno-sanitize-recover=all".into(),
            "-g".into(),
            "-std=c11".into(),
        ]
    }

    pub fn compile_with_sanitizers(
        &self,
        source: &Path,
        output: &Path,
        extra_flags: &[String],
    ) -> Result<CompileResult> {
        let mut args = self.sanitizer_args();
        args.extend(extra_flags.iter().map(OsString::from));
        args.push("-o".into());
        args.push(output.into());
        args.push(source.into());

        self.run_compiler(&args)
    }

    fn run_compiler(&self, args: &[OsString]) -> Result<CompileResult> {
        let output = Command::new(self.kind.command_name())
            .args(args)
            .output()
            .with_context(|| format!("Failed to run {}", self.kind))?;

        let mut combined = String::new();
        if !output.stdout.is_empty() {
            combined.push_str(&String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            combined.push_str(&String::from_utf8_lossy(&output.stderr));
        }

        Ok(CompileResult {
            success: output.status.success(),
            output: combined,
        })
    }
}

fn probe_flag(kind: CompilerKind, flag: &str) -> bool {
    Command::new(kind.command_name())
        .args([flag, "-x", "c", "-E", "-"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_name_gcc() {
        assert_eq!(CompilerKind::Gcc.command_name(), "gcc");
    }

    #[test]
    fn command_name_clang() {
        assert_eq!(CompilerKind::Clang.command_name(), "clang");
    }

    #[test]
    fn display_trait() {
        assert_eq!(format!("{}", CompilerKind::Gcc), "gcc");
        assert_eq!(format!("{}", CompilerKind::Clang), "clang");
    }

    /// The CI checker (scripts/check_exercises.py) replicates this
    /// module's verification flags. There is no shared spec yet, so
    /// this test is the drift alarm: it parses the Python constants
    /// and compares them with what base_args()/sanitizer args build.
    #[test]
    fn python_checker_flags_match() {
        let script = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/check_exercises.py"),
        )
        .expect("scripts/check_exercises.py must exist");

        let extract = |name: &str| -> Vec<String> {
            let start = script
                .find(&format!("{name} = ["))
                .unwrap_or_else(|| panic!("{name} not found in check_exercises.py"));
            let open = script[start..].find('[').unwrap() + start;
            let close = script[open..].find(']').unwrap() + open;
            // take the quoted substrings: split on '"' yields the
            // contents at every odd index (commas inside a flag,
            // like -fsanitize=address,undefined, stay intact)
            script[open + 1..close]
                .split('"')
                .skip(1)
                .step_by(2)
                .map(str::to_string)
                .collect()
        };

        // Base flags: everything after the include flag must match,
        // minus the probed -fno-diagnostics flag (a cosmetic, Rust-
        // side-only addition the checker deliberately omits).
        let compiler = Compiler {
            kind: CompilerKind::Gcc,
            include_dir: PathBuf::from("include"),
            supports_no_fixit_hints: false,
        };
        let rust_base: Vec<String> = compiler
            .base_args()
            .iter()
            .skip(1)
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let py_base: Vec<String> = extract("BASE_FLAGS").into_iter().skip(1).collect();
        assert_eq!(rust_base, py_base, "base flags drifted from the CI checker");

        // Same shape for the sanitizer stage: compare what the Rust
        // side actually builds, not a copy of it in the test.
        let rust_san: Vec<String> = compiler
            .sanitizer_args()
            .iter()
            .skip(1)
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let py_san: Vec<String> = extract("SAN_FLAGS").into_iter().skip(1).collect();
        assert_eq!(
            rust_san, py_san,
            "sanitizer flags drifted from the CI checker"
        );

        let timeout: u64 = script
            .lines()
            .find_map(|l| l.strip_prefix("RUN_TIMEOUT = "))
            .expect("RUN_TIMEOUT not found")
            .trim()
            .parse()
            .unwrap();
        assert_eq!(
            timeout,
            crate::exercise::RUN_TIMEOUT.as_secs(),
            "run timeout drifted from the CI checker"
        );
    }
}
