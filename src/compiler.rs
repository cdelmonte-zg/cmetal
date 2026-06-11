use anyhow::{Context, Result};
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

        Ok(Self {
            kind,
            include_dir: base_dir.join("include"),
        })
    }

    pub fn kind(&self) -> CompilerKind {
        self.kind
    }

    fn include_flag(&self) -> String {
        format!("-I{}", self.include_dir.display())
    }

    fn base_args(&self) -> Vec<String> {
        let mut args = vec![
            self.include_flag(),
            "-Wall".into(),
            "-Wextra".into(),
            "-Werror".into(),
            "-pedantic".into(),
            "-std=c11".into(),
            "-g".into(),
        ];
        if self.supports_flag("-fno-diagnostics-show-fix-it-hints") {
            args.push("-fno-diagnostics-show-fix-it-hints".into());
        }
        args
    }

    fn supports_flag(&self, flag: &str) -> bool {
        Command::new(self.kind.command_name())
            .args([flag, "-x", "c", "-E", "-"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn compile(
        &self,
        source: &Path,
        output: &Path,
        extra_flags: &[String],
    ) -> Result<CompileResult> {
        let mut args = self.base_args();
        args.extend_from_slice(extra_flags);
        args.push("-o".into());
        args.push(output.to_str().unwrap().into());
        args.push(source.to_str().unwrap().into());

        self.run_compiler(&args)
    }

    pub fn compile_with_tests(
        &self,
        source: &Path,
        output: &Path,
        extra_flags: &[String],
    ) -> Result<CompileResult> {
        let mut args = self.base_args();
        args.extend_from_slice(extra_flags);
        args.push("-DTEST".into());
        args.push("-o".into());
        args.push(output.to_str().unwrap().into());
        args.push(source.to_str().unwrap().into());

        self.run_compiler(&args)
    }

    pub fn compile_with_sanitizers(
        &self,
        source: &Path,
        output: &Path,
        extra_flags: &[String],
    ) -> Result<CompileResult> {
        let mut args = vec![
            self.include_flag(),
            "-fsanitize=address,undefined".into(),
            "-fno-sanitize-recover=all".into(),
            "-g".into(),
            "-std=c11".into(),
        ];
        args.extend_from_slice(extra_flags);
        args.push("-o".into());
        args.push(output.to_str().unwrap().into());
        args.push(source.to_str().unwrap().into());

        self.run_compiler(&args)
    }

    fn run_compiler(&self, args: &[String]) -> Result<CompileResult> {
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
}
