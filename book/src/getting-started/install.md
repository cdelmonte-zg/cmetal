# Install

clings is two things: the `clings` binary (the runner) and this repository (the
exercises). You always need the repository — that is where the exercises live
and where your progress is kept. Pick whichever route gets you the binary.

## Prerequisites

- **gcc and/or clang** with C11 support — the exercises are compiled with your
  system toolchain.
- **git** — to clone the exercises.
- **A Rust toolchain** — only if you build the binary from source.

## Option 1 — Homebrew (macOS, Linux)

No Rust required.

```bash
brew install cdelmonte-zg/tap/clings
```

## Option 2 — prebuilt binary

No Rust required. Download the archive for your platform from the
[latest release](https://github.com/cdelmonte-zg/clings/releases/latest), then:

```bash
tar -xzf clings-<version>-<target>.tar.gz
sudo mv clings-<version>-<target>/clings /usr/local/bin/
```

> **macOS:** the binaries are not code-signed yet. If Gatekeeper blocks the
> first run, clear the quarantine flag:
> `xattr -d com.apple.quarantine $(which clings)`.

## Option 3 — build from source

```bash
git clone https://github.com/cdelmonte-zg/clings.git
cd clings
cargo install --path .
```

## Get the exercises

The binary embeds the full curriculum: however you installed it, the
shortest path is a self-contained workspace —

```bash
clings init my-clings-course   # or just `clings init`
cd my-clings-course
clings
```

Cloning the repository works exactly the same way and remains the route
for contributing exercises or following unreleased changes:

```bash
git clone https://github.com/cdelmonte-zg/clings.git
cd clings
clings
```

Either way, on first run clings copies the exercises into `my_exercises/` —
that is where you work. Head to the [Quickstart](quickstart.md).

## Upgrade

New exercises ship with the binary. Upgrade it, matching how you
installed it:

```bash
brew upgrade clings              # Homebrew
cargo install --path . --force   # built from source
```

If you installed a prebuilt binary, download the new archive from the
[latest release](https://github.com/cdelmonte-zg/clings/releases/latest) and
replace `/usr/local/bin/clings` the same way you installed it.

Then bring your workspace up to date with the curriculum embedded in
the new binary:

```bash
cd my-clings-course
clings update
```

`update` never overwrites work you have edited: untouched working
copies are refreshed, edited ones are kept and reported — compare with
`clings diff <name>` or take the new version with `clings reset <name>`.
(In a git checkout, update with `git pull` instead.)

## Uninstall

Remove the binary, matching how you installed it:

```bash
brew uninstall clings            # Homebrew
sudo rm /usr/local/bin/clings    # prebuilt binary
cargo uninstall clings           # built from source
```

Everything else — your progress, `my_exercises/`, revealed solutions — lives
inside the cloned repository. Delete the `clings` directory and no trace is
left.
