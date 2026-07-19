# The CLI, subcommand by subcommand

Running `cmetal` with no arguments starts [watch mode](watch-mode.md). The
subcommands below do one thing and exit — handy for scripting, for jumping to a
specific exercise, or for checking progress without entering the loop.

## `cmetal init [dir]`

Create a self-contained workspace from the curriculum embedded in the
binary — no git clone needed. With no argument it creates
`./cmetal-workspace`. The target must be a new or empty directory:
`init` refuses to touch a directory that already has content.

```bash
cmetal init my-cmetal-course
cd my-cmetal-course
cmetal
```

## `cmetal run <name>`

Compile, run, and verify a single exercise, print the result, and exit.

```bash
cmetal run pointers1
cmetal run bitwise2 --compiler clang
```

Useful when you want to check one exercise without the watch loop taking over.

## `cmetal hint <name> [--level N]`

Print the exercise's hints. With no `--level`, prints the first; `--level N`
prints the first `N`. Hints go from gentlest to near-answer — see
[Hints and solutions](hints-and-solutions.md).

```bash
cmetal hint pointers1
cmetal hint pointers1 --level 3
```

## `cmetal solution <name>`

Reveal the official solution for an exercise you've already solved. Solutions
are stored obfuscated and unlock only once earned, so this works only after the
exercise passes.

```bash
cmetal solution pointers2
```

## `cmetal diff <name>`

Show how your working copy differs from the pristine exercise, as a
unified diff. Useful after `cmetal update` reports that an exercise
changed upstream while you had edits.

```bash
cmetal diff pointers1
```

## `cmetal list`

List every exercise with its status — solved, pending, or skipped (e.g.
"requires gcc" for a [compiler-restricted](choosing-a-compiler.md) exercise
under the wrong compiler). This is the map of where you are.

```bash
cmetal list
```

## `cmetal verify`

Run the full verification pipeline across **all** exercises and report. This is
the same check CI runs; use it to confirm a clean toolchain or after changing
compilers.

```bash
cmetal verify
cmetal verify --compiler clang
```

## `cmetal update`

Bring an [init-created](#cmetal-init-dir) workspace up to date with the
curriculum embedded in the binary (upgrade the binary first: that is how
new exercises arrive). Your work is safe: working copies you edited are
never overwritten — if their exercise changed upstream, update says so
and points you at `cmetal diff`/`cmetal reset` — while copies you never
touched are refreshed automatically. Interrupted updates are recovered
on the next run. In a git checkout, use `git pull` instead.

```bash
cmetal update
```

## `cmetal reset [name]`

With a name: restore that one exercise's working copy to the pristine
version and mark it pending again — other progress is kept. Without:
clear ALL progress and restore every pristine exercise into
`my_exercises/`. Nothing outside the cmetal workspace is touched.

```bash
cmetal reset pointers1   # redo one exercise
cmetal reset             # start over completely
```

## `--compiler <gcc|clang>`

A global flag, accepted by watch mode and the single-exercise subcommands, that
selects the toolchain for the session. Defaults to gcc. See
[Choosing a compiler](choosing-a-compiler.md).

## Interactive keys (watch mode)

Inside watch mode, these keys drive the loop:

| Key | Action                        |
|-----|-------------------------------|
| `n` | Next exercise                 |
| `p` | Previous exercise             |
| `h` | Show hint (additive)          |
| `l` | List all exercises            |
| `r` | Re-run current exercise       |
| `q` | Quit                          |
