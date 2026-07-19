# Contributing to clings

Thanks for your interest in contributing!

> **Note:** learners never edit `exercises/` — clings copies it into the
> gitignored `my_exercises/` workspace on first run. `exercises/` must
> only ever contain the broken, unsolved versions.

## Adding an exercise

1. Create the exercise file in `exercises/<topic_dir>/<name>.c`
2. Create the matching solution in `solutions/<topic_dir>/<name>.c`,
   then run `python3 scripts/solutions_codec.py pack` — solutions are
   stored obfuscated as `.c.enc` so learners aren't spoiled by accident
   (clings reveals them in `my_solutions/` once an exercise passes).
   To edit existing solutions, run `... unpack` first, edit, re-`pack`.
3. Add an entry in `info.toml` with progressive hints

### Naming

- Topic directories: `NN_topic` (e.g. `01_pointers`, `06_strings`)
- Exercise files: `<topic>N.c` (e.g. `pointers1.c`, `pointers2.c`)

### Exercise structure

```c
// Short description of what this exercise teaches.

#include <stdio.h>

// TODO: Fix/implement something
void function_with_bug(void) { ... }

#ifndef TEST
int main(void) {
    // Interactive demo that shows the bug
}
#else
#include "clings_test.h"

TEST(test_name) { ASSERT_EQ(...); }

int main(void) {
    RUN_TEST(test_name);
    TEST_REPORT();
}
#endif
```

### info.toml entry

```toml
[[exercises]]
name = "pointers3"
dir = "01_pointers"
test = true          # compile with -DTEST and run tests
sanitizers = false   # compile with ASan/UBSan
# flags = ["-O2"]    # optional extra compiler flags for this exercise
# compilers = ["gcc"]  # optional: restrict to compilers where the bug is
                       # detectable (default: all). Restricted exercises are
                       # skipped when clings runs with another --compiler.
hints = [
    "First hint: the gentlest nudge",
    "Second hint: more specific",
    "Third hint: almost the answer",
]
```

### Requirements for solutions

- Must compile with `-Wall -Wextra -Werror -pedantic -std=c11`
- Must pass with both `gcc` and `clang`
- Must pass with `-fsanitize=address,undefined` when sanitizers are enabled
- Stick to C11 standard -- no POSIX-specific features

### Language vs platform

Exercises may rely on mainstream ABI facts — 8-bit bytes, 32-bit `int`,
8-byte `double` alignment — but must not present them as C guarantees.
When an exercise depends on such an assumption, make it explicit in the
code with a `_Static_assert` (see `ub3`, `structs1`, `bitwise1`,
`strings3`) and phrase comments so that what the C standard guarantees
and what the target ABI provides stay distinguishable.

### Fallible allocations

If an exercise's contract includes allocation failure ("returns -1 and
leaves the object untouched"), route its allocations through
`CLINGS_MALLOC` / `CLINGS_REALLOC` from `include/clings_alloc.h`
instead of calling malloc/realloc directly. In normal builds they are
plain malloc/realloc; in TEST builds a test can arm the next allocation
to fail with `clings_fail_next_alloc()` and assert the failure branch
deterministically (see `memory2`, `structs2`, `function_pointers2`).

### The exercise invariant

Every exercise must **fail** verification as shipped, and every solution
must **pass** it. An exercise that is already green teaches nothing;
CI enforces this on every push and PR:

```bash
python3 scripts/check_exercises.py
```

This replicates the exact `clings` verification pipeline (base flags plus
the per-exercise `test`, `sanitizers` and `flags` settings from
`info.toml`). Run it whenever you add or change an exercise — and before
pushing, so you don't accidentally publish exercises in their solved
state. You can have git run it automatically on every push:

```bash
git config core.hooksPath scripts/hooks
```

## Running tests locally

```bash
cargo test                # Rust unit + integration tests
cargo clippy -- -D warnings
python3 scripts/check_exercises.py   # C exercise/solution invariant
```

## Code style

- **C:** follow the existing style, 4-space indent
- **Rust:** `cargo fmt` + `cargo clippy`
