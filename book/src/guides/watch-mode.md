# Watch mode: the core loop

Running `clings` with no arguments starts **watch mode** — the experience the
tool is built around. It picks up at your first unsolved exercise and stays
there, watching the file, until you solve it.

## What it does

1. It shows the current exercise: its name, the path to its `.c` file under
   `my_exercises/`, and its current status.
2. It watches `my_exercises/` for changes.
3. Every time you **save** the exercise file, clings recompiles it, runs it,
   and re-verifies — no keypress needed. The screen updates in place with the
   new result (a compile error, a failed assertion, a sanitizer report, or a
   green pass).
4. When the exercise passes, its solution unlocks in `my_solutions/` and you
   move on with `n`.

The unit of feedback is a file save. You never leave your editor to "run"
anything; saving *is* running.

## Reading a result

A result is one of a few shapes:

- **Compile error** — the compiler's own output, verbatim. The most common
  early result; the message is the lesson.
- **Runtime failure** — the program compiled and ran, but signalled failure
  (a non-zero exit, or a failed test assertion when the exercise has tests).
- **Sanitizer report** — for exercises with sanitizers enabled, AddressSanitizer
  or UBSan caught something at runtime (a use-after-free, an overflow, a leak).
  This is the whole point of the [UB Lab](../reference/curriculum.md) exercises.
- **Pass** — green. The solution unlocks.

What exactly is compiled and run — and with which flags — is spelled out in
[How verification works](../reference/how-it-works.md).

## Keys

| Key | Action                                             |
|-----|----------------------------------------------------|
| `n` | Move to the next exercise                          |
| `p` | Move to the previous exercise                      |
| `h` | Reveal the next hint (they accumulate)             |
| `l` | List every exercise with its status               |
| `r` | Re-run the current exercise now                    |
| `q` | Quit (progress is saved)                           |

`r` is useful when a result depends on something outside the file — you changed
a compiler with `--compiler`, or you just want to force a fresh run.

## Progress persists

Your position and which exercises you've solved are written to
`.clings-state.txt` in the repo. Quit whenever; the next `clings` resumes
exactly where you were. To wipe it and start clean, use
[`clings reset`](cli.md).

## When you don't want the loop

Watch mode is for working through the curriculum. For one-off actions — running
a single exercise, listing progress, re-opening a solution — the
[subcommands](cli.md) do the job without entering the loop.
