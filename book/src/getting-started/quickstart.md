# Quickstart

Five minutes, one exercise, start to green.

## Start

Create a workspace (once) and run cmetal from inside it:

```bash
cmetal init my-cmetal-course
cd my-cmetal-course
cmetal
```

(A cloned repository works exactly the same way — just run `cmetal`
from inside the clone.)

The first run copies the pristine exercises from `exercises/` into
`my_exercises/` — that is where you work; your edits never touch the
pristine copies — and drops you on the first unsolved exercise in
watch mode.

## Fix your first exercise

cmetal shows you the current exercise, its status, and where its file lives:

```text
  Exercise: 00_intro/intro1
  File:     my_exercises/00_intro/intro1.c

  ✗ Compiled and ran, but the program signalled failure.

  Commands: [n]ext [p]rev [h]int [l]ist [r]e-run [q]uit
```

Open that `.c` file **under `my_exercises/`** in your editor — not the one in
`exercises/`, which stays untouched. Read the comment at the top: it tells you
what the exercise teaches. Find the bug or the `TODO`, fix it, and **save**.

cmetal notices the save, recompiles, and re-verifies automatically. When it
passes, the screen turns green and the official solution appears in
`my_solutions/` so you can compare approaches.

## Stuck?

Press `h`. Hints are progressive — the first is the gentlest nudge, and each
`h` reveals one more, up to the near-answer. Nothing is spoiled unless you ask
for it. See [Hints and solutions](../guides/hints-and-solutions.md).

## Move around

| Key | Action                |
|-----|-----------------------|
| `n` | Next exercise         |
| `p` | Previous exercise     |
| `h` | Show the next hint    |
| `l` | List all exercises    |
| `r` | Re-run the current one|
| `q` | Quit                  |

Progress is saved to `.cmetal-state.txt` and persists across sessions — quit
any time and `cmetal` picks up where you left off.

## Coming back later

```bash
cmetal                    # resume where you left off
cmetal list               # see every exercise and your progress
cmetal solution pointers2 # re-open a solution you've already earned
cmetal reset              # wipe progress, restore pristine exercises
```

The full command surface is in [The CLI](../guides/cli.md).

## Next

- [Watch mode: the core loop](../guides/watch-mode.md) — how save-driven
  verification actually works.
- [The curriculum](../reference/curriculum.md) — the 47 exercises and what each
  one teaches.
