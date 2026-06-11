# Vision

**clings wants to become the reference hands-on path from "I know C syntax"
to "I write C I can defend in a code review".**

C is taught everywhere, but almost always up to the point where programs
*compile*. The hard part of the language starts after that: undefined
behavior, aliasing, lifetime of memory, const discipline, error handling
that survives real call chains. Books explain these topics; almost nothing
lets you *practice* them with immediate feedback. That is the gap clings
exists to fill — the way rustlings did for Rust, but for the parts of C
that actually hurt.

## Where we are

Today clings ships 32 exercises across 12 topics (pointers, memory, UB,
strings, structs, function pointers, const, error handling, bitwise), a
watch-mode TUI with progressive hints, sanitizer-backed verification, a
gitignored workspace so your progress never pollutes the repo, and
solutions that unlock only after you solve the exercise.

## Design principles

These are the non-negotiables that every future change must respect:

1. **Learn by fixing, not by reading.** Every exercise is broken code with
   a real bug — the same bugs found in production C — never a fill-in-the-
   blanks template.
2. **Every exercise fails as shipped; every solution passes.** This
   invariant is enforced by CI (`scripts/check_exercises.py`). An exercise
   that starts green teaches nothing.
3. **Feedback in seconds.** Save the file, see the result. Compile errors,
   test failures and sanitizer reports *are* the teaching material.
4. **No accidental spoilers.** Solutions stay opaque until earned, then
   appear next to your own attempt for comparison.
5. **Pure C11, real toolchains.** gcc and clang, ASan and UBSan — the
   tools you will use at work. No custom runtime, no framework lock-in.
6. **The learner's clone stays clean.** Work happens in gitignored
   directories; the repository itself is never dirtied by learning.

## Where we want to go

### Near term — depth of the current curriculum

- Grow each topic to a proper arc (3–6 exercises from warm-up to tricky).
- More UB Lab scenarios: use-after-free across functions, dangling stack
  pointers, integer promotion traps, sequence-point violations.
- Explain failures, not just show them: short "what the sanitizer is
  telling you" notes attached to each exercise's hints.

### Mid term — the missing chapters of advanced C

- **Concurrency:** C11 `<threads.h>`, atomics, data races caught by
  ThreadSanitizer.
- **The machine under the language:** alignment, padding, endianness,
  `restrict`, volatile — what the optimizer assumes and why.
- **APIs that last:** opaque handles, ownership conventions, ABI
  stability basics.
- Difficulty ratings and optional topic tracks, so a learner can follow
  "systems track" or "embedded track" through the same exercise pool.

### Long term

- **C23 track** as compiler support matures (`nullptr`, `constexpr`,
  checked arithmetic).
- **Packaged distribution** (crates.io, Homebrew, distro packages) so
  `clings` is one install command away.
- **Community exercise pipeline:** contributing a new exercise should be
  a 30-minute task with the invariant checker as the only gatekeeper.
- **Localized hints** — the code stays English, the teaching can speak
  your language.

## Non-goals

- Teaching C syntax from zero — start with any introductory course, then
  come here.
- C++ — a different language with different lessons.
- Becoming an IDE, a build system, or a general-purpose C tutor. clings
  is a sharp tool for one job: deliberate practice on hard C.

## How to help

Pick a topic above, write one broken program and its fix, and read
[CONTRIBUTING.md](CONTRIBUTING.md). If `scripts/check_exercises.py` is
green, you have taught the next person something real.
