# clings

**Small exercises to learn advanced C concepts — by fixing broken code.**

C is taught everywhere, but almost always up to the point where a program
*compiles*. The hard part of the language starts after that: undefined
behavior, aliasing, the lifetime of memory, `const` discipline, error handling
that survives a real call chain. Books explain these; almost nothing lets you
*practice* them with immediate feedback. That is the gap clings fills — the way
[rustlings](https://github.com/rust-lang/rustlings) did for Rust, but for the
parts of C that actually hurt.

Each exercise is a `.c` file with a real bug or a `TODO`. You open it, fix it,
and save. clings recompiles, runs the binary, runs the tests, runs the
sanitizers, and tells you — in seconds — whether you got it right.

```text
  Exercise: 02_memory/memory2

  ✗ AddressSanitizer: heap-use-after-free
      free(buf);
      return buf[0];   // <- read after free

  Press [h] for a hint.
```

Fix it, save, and the same screen turns green — and the official solution
unlocks in `my_solutions/` so you can compare it with yours.

## The loop

1. Run `clings` in the repo. Your working copies live in `my_exercises/`
   (gitignored); the originals in `exercises/` stay pristine.
2. Open the exercise's `.c` file in your editor. Fix the bug.
3. Save. clings recompiles and re-verifies automatically — [watch
   mode](guides/watch-mode.md) is the core experience.
4. Stuck? Press `h` for [progressive hints](guides/hints-and-solutions.md).
5. Green? The solution unlocks. On to the next.

## What it is for

- **Deliberate practice on hard C.** 35 exercises across 12 topics, from
  pointer decay to bit-packing, each built around a bug you will actually meet
  in production code. See [the curriculum](reference/curriculum.md).
- **Real toolchains, not a sandbox.** gcc and clang, AddressSanitizer and
  UBSan — the tools you use at work. [Choosing a
  compiler](guides/choosing-a-compiler.md).
- **A curriculum you can extend.** Adding an exercise is writing a broken
  program, its fix, and a few hints; one script gatekeeps quality. See
  [Contributing an exercise](guides/contributing.md).

## What it is not

clings is not a C course from zero, not a C++ tutor, and not an IDE or build
system. It assumes you know C syntax and want to get good at the parts that
bite. It is a sharp tool for one job: deliberate practice on advanced C. Where
that line moves next is tracked in the [roadmap](project/roadmap.md).

The CLI is written in Rust for a fast, cross-platform experience; the exercises
are pure C11.
