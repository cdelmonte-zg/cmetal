# How verification works

Every time you save an exercise — or run `cmetal run`, `cmetal verify`, or the
CI check — the same pipeline decides pass or fail. Knowing the exact stages and
flags demystifies the results you see.

## The pipeline

For each exercise, in order:

1. **Compile and run the demo.** The `.c` file is compiled with the base flags
   and executed. A compile error stops here; a non-zero exit is a failure.
2. **Compile and run the tests** — only if the exercise sets `test = true`. The
   file is recompiled with `-DTEST`, which switches on the exercise's built-in
   test block, and run. A failed assertion is a failure.
3. **Compile and run under sanitizers** — only if the exercise sets
   `sanitizers = true`. The file is recompiled with AddressSanitizer and UBSan
   and run again. Any sanitizer diagnostic is a failure.

An exercise passes only when every stage that applies to it passes.

## The flags

**Base flags** (stage 1, and stage 2 with `-DTEST` added):

```text
-Iinclude -Wall -Wextra -Werror -pedantic -std=c11 -g
```

`-Werror` matters: for the exercises, a warning *is* an error. Much of what you
learn here is making the compiler stop complaining.

**Sanitizer flags** (stage 3):

```text
-Iinclude -fsanitize=address,undefined -fno-sanitize-recover=all -g -std=c11
```

`-fno-sanitize-recover=all` makes the program abort on the first sanitizer
finding rather than printing and continuing — so a caught UB is an unambiguous
failure. Note the sanitizer stage drops `-Werror`: its job is to catch runtime
behavior, not warnings.

Any per-exercise `flags` from `info.toml` (for example `-O2`) are appended to
every stage — some bugs only surface under optimisation.

## Per-exercise settings

Three fields in [`info.toml`](anatomy.md) tune the pipeline for each exercise:

- `test` — run the `-DTEST` stage.
- `sanitizers` — run the AddressSanitizer/UBSan stage.
- `flags` — extra compiler flags appended to every stage.
- `compilers` — restrict the exercise to compilers that can actually detect its
  bug (see below).

## The exercise invariant

The rule the whole project is built on: **every exercise must fail verification
as shipped, and every solution must pass it.** An exercise that starts green
teaches nothing. This is enforced by `scripts/check_exercises.py`, which
replicates this exact pipeline against both the broken `exercises/` and the
reference `solutions/`, and runs in CI on every push and pull request.

## Compiler restriction

Some bugs are only diagnosed by one compiler (a gcc-specific warning, say). Such
an exercise declares `compilers = ["gcc"]`. Under a different compiler it is
**skipped**, because its "must fail as shipped" half can't hold there — but note
the *solution* must still pass under every compiler, so that half of the
invariant is always checked. This is why a restricted exercise shows as
"requires gcc" in `cmetal list` rather than passing vacuously. See
[Choosing a compiler](../guides/choosing-a-compiler.md).
