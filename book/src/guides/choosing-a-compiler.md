# Choosing a compiler

cmetal compiles the exercises with your real system toolchain — gcc or clang —
not a bundled or emulated one. Which compiler you use is a genuine part of the
lesson: the two disagree, on purpose, about some of these bugs.

## Picking one

By default cmetal uses gcc. Pick the other at startup:

```bash
cmetal --compiler clang
```

The choice applies to every exercise for that session and is shown on the
welcome screen and in the watch header, so you always know which compiler's
diagnostics you're reading. The single-exercise subcommands take it too:

```bash
cmetal run bitwise2 --compiler clang
cmetal verify --compiler clang
```

## Why the compiler matters

Advanced C is partly a conversation with a specific compiler. gcc and clang
issue different warnings, phrase the same error differently, and — critically
for a learning tool — some bugs are only *diagnosed* by one of them. An
exercise whose whole point is a gcc-specific `-W...` warning would compile
cleanly under clang and pass without teaching anything.

## Compiler-restricted exercises

To avoid that hollow pass, an exercise can declare which compilers can actually
detect its bug, via `compilers = [...]` in `info.toml`. When you run cmetal with
a compiler that isn't in the list, that exercise is **skipped** rather than
shown as solved — it appears as "requires gcc" (or clang) in
[`cmetal list`](cli.md).

This keeps the invariant honest: an exercise only counts as passed on a
compiler that could have failed it. The mechanism is described from the
author's side in [Anatomy of an exercise](../reference/anatomy.md) and
[How verification works](../reference/how-it-works.md).

## Which should you use?

Either. If you're learning C for a codebase that standardises on one of them,
match it. Otherwise, work through once with gcc and once with clang — reading
how each describes the same class of bug is itself worth doing. Both must have
C11 support (any reasonably recent version does).
