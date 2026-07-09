# The curriculum

clings ships **32 exercises across 12 topics**, ordered roughly from warm-up to
the parts of C that bite in code review. Each is a real bug — the kind found in
production C — never a fill-in-the-blanks template.

| #  | Topic                 | Exercises | What you will learn                                  |
|----|-----------------------|-----------|------------------------------------------------------|
| 00 | Intro                 | 1         | Getting started, basic program structure             |
| 01 | Pointers              | 2         | Decay, arithmetic, pointer-size pitfalls             |
| 02 | Memory                | 3         | `malloc`/`free`, `realloc`, leaks, double-free       |
| 03 | Undefined Behavior    | 1         | Signed overflow detection                            |
| 04 | Preprocessor          | 1         | Stringify, token pasting, macro pitfalls             |
| 05 | UB Lab                | 6         | Hands-on UB experiments with sanitizer feedback      |
| 06 | Strings               | 3         | Safe concatenation, tokenizing, parsing              |
| 07 | Structs               | 3         | Layout/padding, opaque types, linked lists           |
| 08 | Function Pointers     | 3         | Callbacks, generic sort, dispatch tables             |
| 09 | Const Correctness     | 3         | `const` parameters, pointer-to-const, immutable API  |
| 10 | Error Handling        | 3         | Return codes, error propagation, error context       |
| 11 | Bitwise               | 3         | Bit counting, packing/unpacking, bit tricks          |

## How the topics build

The early topics (**Pointers**, **Memory**) establish the mental model that
everything else depends on: what a pointer *is*, when an array decays into one,
who owns a heap allocation and when it dies. Get these wrong and the rest of C
is guesswork.

The middle topics turn that model against you. **Undefined Behavior** and the
six-exercise **UB Lab** are where you deliberately trigger real UB — signed
overflow, use-after-free, dangling pointers — and watch AddressSanitizer and
UBSan catch it. The sanitizer report *is* the teaching material; the point isn't
to avoid UB abstractly but to recognise what it looks like when a tool flags it.

The later topics are about writing C other people can trust: **Strings** (the
functions everyone gets wrong), **Structs** (layout, padding, opaque types),
**Function Pointers** (callbacks and dispatch), **Const Correctness** (APIs that
document their own immutability), **Error Handling** (codes that survive a real
call chain), and **Bitwise** (the low-level idioms).

## The UB Lab

The UB Lab (topic 05) is worth calling out. Most exercises ask you to *fix* a
bug. The UB Lab asks you to *observe* one: you write the code that triggers a
specific undefined behavior, run it under sanitizers, and read exactly how the
tool reports it. Use-after-free across function boundaries, dangling stack
pointers, integer promotion traps — the goal is fluency in what a sanitizer is
telling you, which is a skill no book can hand you.

## Seeing the list live

`clings list` shows every exercise with its status — solved, pending, or skipped
because it [requires a specific compiler](../guides/choosing-a-compiler.md). It
is the authoritative, up-to-date view; this table is the map.

The curriculum is growing — where it's headed (concurrency, alignment and the
machine model, long-lasting APIs, a C23 track) is in the
[roadmap](../project/roadmap.md).
