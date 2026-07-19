# Install

The `cmetal` binary carries the whole course: install it, run `cmetal
init`, and you have a private workspace with every exercise — no git,
no clone. (Contributors still work from the repository; see
[Contributing](../guides/contributing.md).) Pick whichever route gets
you the binary.

## Prerequisites

- **gcc and/or clang** with C11 support — the exercises are compiled with your
  system toolchain.
- **git and a Rust toolchain** — only to contribute or build the binary
  from source; learning needs neither.

## Option 1 — Homebrew (macOS, Linux)

No Rust required.

```bash
brew install cdelmonte-zg/tap/cmetal
```

## Option 2 — prebuilt binary

No Rust required. Download the archive for your platform from the
[latest release](https://github.com/cdelmonte-zg/cmetal/releases/latest), then:

```bash
tar -xzf cmetal-<version>-<target>.tar.gz
sudo mv cmetal-<version>-<target>/cmetal /usr/local/bin/
```

> **macOS:** the binaries are not code-signed yet. If Gatekeeper blocks the
> first run, clear the quarantine flag:
> `xattr -d com.apple.quarantine $(which cmetal)`.

## Option 3 — build from source

```bash
git clone https://github.com/cdelmonte-zg/cmetal.git
cd cmetal
cargo install --path .
```

## Get the exercises

The binary embeds the full curriculum: however you installed it, the
shortest path is a self-contained workspace —

```bash
cmetal init my-cmetal-course   # or just `cmetal init`
cd my-cmetal-course
cmetal
```

Cloning the repository works exactly the same way and remains the route
for contributing exercises or following unreleased changes:

```bash
git clone https://github.com/cdelmonte-zg/cmetal.git
cd cmetal
cmetal
```

Either way, on first run cmetal copies the exercises into `my_exercises/` —
that is where you work. Head to the [Quickstart](quickstart.md).

## Upgrade

New exercises ship with the binary. Upgrade it, matching how you
installed it:

```bash
brew upgrade cmetal              # Homebrew
cargo install --path . --force   # built from source
```

If you installed a prebuilt binary, download the new archive from the
[latest release](https://github.com/cdelmonte-zg/cmetal/releases/latest) and
replace `/usr/local/bin/cmetal` the same way you installed it.

Then bring your workspace up to date with the curriculum embedded in
the new binary:

```bash
cd my-cmetal-course
cmetal update
```

`update` never overwrites work you have edited: untouched working
copies are refreshed, edited ones are kept and reported — compare with
`cmetal diff <name>` or take the new version with `cmetal reset <name>`.
(In a git checkout, update with `git pull` instead.)

## Uninstall

Remove the binary, matching how you installed it:

```bash
brew uninstall cmetal            # Homebrew
sudo rm /usr/local/bin/cmetal    # prebuilt binary
cargo uninstall cmetal           # built from source
```

Everything else — your progress, `my_exercises/`, revealed solutions — lives
inside your workspace (or clone). Delete that directory and no trace is
left.
