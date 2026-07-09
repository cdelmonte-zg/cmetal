# Anatomy of an exercise

An exercise is three things that live together: a broken C file, its reference
solution, and an entry in `info.toml` that ties them to the
[verification pipeline](how-it-works.md). This chapter is the reference for that
shape; the step-by-step contributor walkthrough is in
[Contributing an exercise](../guides/contributing.md).

## The three pieces

| Piece      | Location                              | Role                                   |
|------------|---------------------------------------|----------------------------------------|
| Exercise   | `exercises/<NN_topic>/<name>.c`       | The broken code the learner fixes      |
| Solution   | `solutions/<NN_topic>/<name>.c.enc`   | The reference fix, stored obfuscated   |
| Metadata   | `info.toml`                           | Hints and per-exercise pipeline settings |

Learners never touch `exercises/` — clings copies it into the gitignored
`my_exercises/` workspace on first run. `exercises/` therefore only ever holds
the *broken, unsolved* version.

## The exercise file

A typical exercise carries an interactive demo and an optional test block behind
`#ifdef TEST`, so the same file serves both the run stage and the `-DTEST` test
stage:

```c
// Short description of what this exercise teaches.

#include <stdio.h>

// TODO: fix or implement something
void function_with_bug(void) { /* ... */ }

#ifndef TEST
int main(void) {
    // Interactive demo that shows the bug
}
#else
#include "clings_test.h"

TEST(test_name) { ASSERT_EQ(/* ... */); }

int main(void) {
    RUN_TEST(test_name);
    TEST_REPORT();
}
#endif
```

The first comment line matters — it's what the learner reads to know what the
exercise is about.

## The `info.toml` entry

```toml
[[exercises]]
name = "pointers3"
dir = "01_pointers"
test = true            # run the -DTEST stage
sanitizers = false     # run the AddressSanitizer/UBSan stage
# flags = ["-O2"]      # extra compiler flags, appended to every stage
# compilers = ["gcc"]  # restrict to compilers that can detect the bug
hints = [
    "First hint: the gentlest nudge",
    "Second hint: more specific",
    "Third hint: almost the answer",
]
```

- **`name` / `dir`** — locate the `.c` file at `<dir>/<name>.c`. Naming is
  `NN_topic` for directories and `<topic>N.c` for files
  (`01_pointers/pointers2.c`).
- **`test`, `sanitizers`, `flags`, `compilers`** — feed straight into the
  [verification pipeline](how-it-works.md).
- **`hints`** — the progressive ladder, gentlest first. An exercise may use a
  single `hint = "..."` instead of a `hints = [...]` list.

## Solution storage

Solutions are authored in plaintext under `solutions/` but committed as `.c.enc`
so browsing the repo never spoils an answer. Two scripts manage the round-trip:

```bash
python3 scripts/solutions_codec.py unpack   # decode .c.enc -> .c to edit
python3 scripts/solutions_codec.py pack     # re-encode .c -> .c.enc to commit
```

The plaintext `.c` files under `solutions/` are gitignored; only the `.c.enc`
form is tracked.

## The invariant it all serves

Whatever the pieces say, one rule governs them: the exercise must **fail**
verification as shipped, and the solution must **pass** it. `clings verify` and
`scripts/check_exercises.py` both enforce it — the latter runs in CI. See
[How verification works](how-it-works.md#the-exercise-invariant).
