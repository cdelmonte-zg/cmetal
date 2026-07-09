# The CLI, subcommand by subcommand

Running `clings` with no arguments starts [watch mode](watch-mode.md). The
subcommands below do one thing and exit — handy for scripting, for jumping to a
specific exercise, or for checking progress without entering the loop.

## `clings run <name>`

Compile, run, and verify a single exercise, print the result, and exit.

```bash
clings run pointers1
clings run bitwise2 --compiler clang
```

Useful when you want to check one exercise without the watch loop taking over.

## `clings hint <name> [--level N]`

Print the exercise's hints. With no `--level`, prints the first; `--level N`
prints the first `N`. Hints go from gentlest to near-answer — see
[Hints and solutions](hints-and-solutions.md).

```bash
clings hint pointers1
clings hint pointers1 --level 3
```

## `clings solution <name>`

Reveal the official solution for an exercise you've already solved. Solutions
are stored obfuscated and unlock only once earned, so this works only after the
exercise passes.

```bash
clings solution pointers2
```

## `clings list`

List every exercise with its status — solved, pending, or skipped (e.g.
"requires gcc" for a [compiler-restricted](choosing-a-compiler.md) exercise
under the wrong compiler). This is the map of where you are.

```bash
clings list
```

## `clings verify`

Run the full verification pipeline across **all** exercises and report. This is
the same check CI runs; use it to confirm a clean toolchain or after changing
compilers.

```bash
clings verify
clings verify --compiler clang
```

## `clings reset`

Clear your progress and restore the pristine exercises into `my_exercises/`.
Your solved-state in `.clings-state.txt` is wiped and revealed solutions are
cleared. Nothing outside the clings workspace is touched.

```bash
clings reset
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
