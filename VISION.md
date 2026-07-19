# Vision

**clings wants to become the reference hands-on path from "I know C syntax"
to "I write C I can defend in a code review" — increasingly through the
problems found in language implementations and binary formats.**

C is taught everywhere, but almost always up to the point where programs
*compile*. The hard part of the language starts after that: undefined
behavior, aliasing, lifetime of memory, const discipline, error handling
that survives real call chains. Books explain these topics; almost nothing
lets you *practice* them with immediate feedback. That is the gap clings
exists to fill — the way rustlings did for Rust, but for the parts of C
that actually hurt.

## Where we are

Today clings ships 42 exercises across 14 topics (pointers, memory, UB,
strings, structs, function pointers, const, error handling, bitwise), a
watch-mode TUI with progressive hints, sanitizer-backed verification,
solutions that unlock only after you solve the exercise, and a
self-contained distribution: the binary embeds the curriculum, `clings
init` materializes a private workspace with no git clone, and `clings
update` delivers new exercises without ever overwriting your work.

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

### The direction — the C of language implementations

clings is developing a focused implementation track around the C used
in interpreters, compilers, and binary formats. That domain covers much
of hard C — tagged unions, hash tables, arenas, garbage collectors,
bytecode — in pure C11, entirely in userspace, where sanitizers give
direct feedback. It also addresses a documented gap: the standard book
on the subject advises readers who are not yet comfortable with C to
work through an introductory book first and come back.

The general advanced-C curriculum stays as it is: it is the foundations
tier the implementation track builds on.

One editorial rule keeps clings a C trainer rather than an interpreter
tutorial in exercise form: **every exercise must be useful to someone
who will never build an interpreter**. Endian-safe I/O belongs in any
protocol; a defensive varint decoder is any parser; an arena is a
lifetime strategy; a GC is a graph traversal over owned memory.
Exercises stand alone — they never require the artifacts of previous
chapters.

### Near term — the implementation track

- **Bytes on the wire** ✓ — endianness, defensive varints, bit packing
  (topic 12).
- **Tagged unions and value representation** ✓ — tag discipline,
  exhaustive dispatch, ownership across variants, and the header-first
  struct idiom (topic 13).
- **Hash tables from scratch** — FNV-1a, open addressing, tombstones,
  string interning.
- **Arena allocators** — ownership at scale, lifetime by construction.
- **A mark-sweep GC on a toy heap** — with AddressSanitizer as the
  judge, because a GC bug *is* a use-after-free.
- **NaN boxing** and **bytecode dispatch** (the portable switch, and
  computed goto as a gcc-only variant via per-exercise flags).
- **Capstone:** serialize, validate and reload a bytecode chunk —
  magic, version, constant pool, code stream.

Alongside the track, the foundations keep deepening: proper arcs for
the thin topics, more UB Lab scenarios (use-after-free across
functions, double-free, misaligned access), and "what the sanitizer is
telling you" notes attached to each exercise's hints.

### Mid term — the missing chapters of advanced C

- **Concurrency:** C11 `<threads.h>`, atomics, data races caught by
  ThreadSanitizer.
- **The machine under the language:** alignment, padding, `restrict`,
  volatile — what the optimizer assumes and why (endianness ✓ started
  with topic 12).
- **APIs that last:** opaque handles, ownership conventions, ABI
  stability basics.
- Difficulty ratings and optional topic tracks, so a learner can follow
  "systems track" or "embedded track" through the same exercise pool.

### Long term

- **C23 track** as compiler support matures (`nullptr`, `constexpr`,
  checked arithmetic).
- **Packaged distribution** so `clings` is one install command away —
  Homebrew and the self-contained binary exist. crates.io needs the
  name question settled first: the `clings` crate name is taken, and
  several projects in this niche share the name — a rename is under
  consideration as part of the repositioning.
- **Community exercise pipeline:** contributing a new exercise should be
  a 30-minute task with the invariant checker as the only gatekeeper.
- **Localized hints** — the code stays English, the teaching can speak
  your language.

## Non-goals

- Teaching C syntax from zero — start with any introductory course, then
  come here.
- Teaching compiler theory — parsing algorithms and type systems have
  their own excellent books; clings teaches the C those books assume
  you already have.
- C++ — a different language with different lessons.
- Becoming an IDE, a build system, or a general-purpose C tutor. clings
  is a sharp tool for one job: deliberate practice on hard C.

## How to help

Pick a topic above, write one broken program and its fix, and read
[CONTRIBUTING.md](CONTRIBUTING.md). If `scripts/check_exercises.py` is
green, you have taught the next person something real.
