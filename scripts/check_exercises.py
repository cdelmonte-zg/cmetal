#!/usr/bin/env python3
"""Enforce the cmetal exercise invariant:

  - every file in exercises/  must FAIL verification as shipped,
  - every file in solutions/  must PASS verification.

An exercise that passes as-is gives the learner a green check without
teaching anything; a solution that fails is unsolvable. Both are bugs.

Verification mirrors `Exercise::verify()` in src/exercise.rs exactly,
including the per-exercise `test`, `sanitizers` and `flags` settings
from info.toml:

  1. compile with the base flags (+ per-exercise flags), run the demo
  2. if test:        recompile with -DTEST, run the tests
  3. if sanitizers:  recompile with ASan/UBSan (no -Werror), run again

Exercises restricted to specific compilers via `compilers = [...]` in
info.toml are skipped on other compilers (their "must fail as shipped"
half doesn't hold there), but their solutions are still verified.

Usage:
    python3 scripts/check_exercises.py [--compiler gcc|clang]
"""

import argparse
import os
import subprocess
import sys
import tempfile
import tomllib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from solutions_codec import decode_bytes  # noqa: E402

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

BASE_FLAGS = ["-Iinclude", "-Wall", "-Wextra", "-Werror", "-pedantic",
              "-std=c11", "-g"]
SAN_FLAGS = ["-Iinclude", "-fsanitize=address,undefined",
             "-fno-sanitize-recover=all", "-g", "-std=c11"]
RUN_TIMEOUT = 10


def compile_ok(cc, args, out, src):
    res = subprocess.run([cc, *args, "-o", out, src],
                         cwd=REPO, capture_output=True)
    return res.returncode == 0


def run_ok(out):
    try:
        return subprocess.run([out], capture_output=True,
                              timeout=RUN_TIMEOUT).returncode == 0
    except subprocess.TimeoutExpired:
        return False


def verify(cc, src, ex, build_dir):
    """Return None if the file passes all stages, else the failing stage."""
    flags = ex.get("flags", [])
    out = os.path.join(build_dir, ex["name"])

    if not compile_ok(cc, BASE_FLAGS + flags, out, src):
        return "compilation"
    if not run_ok(out):
        return "execution"
    if ex.get("test", True):
        if not compile_ok(cc, BASE_FLAGS + flags + ["-DTEST"], out, src):
            return "test compilation"
        if not run_ok(out):
            return "tests"
    if ex.get("sanitizers", False):
        if not compile_ok(cc, SAN_FLAGS + flags, out, src):
            return "sanitizer compilation"
        if not run_ok(out):
            return "sanitizer check"
    return None


def solution_source(rel, build_dir):
    """Return a compilable path for a solution: the plaintext .c if present
    (contributor working copy), else the .c.enc decoded into build_dir."""
    plain = os.path.join(REPO, "solutions", rel)
    if os.path.exists(plain):
        return plain
    enc = plain + ".enc"
    if not os.path.exists(enc):
        return None
    out = os.path.join(build_dir, "sol_" + rel.replace(os.sep, "_"))
    with open(enc) as f:
        data = decode_bytes(f.read())
    with open(out, "wb") as f:
        f.write(data)
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--compiler", default="gcc",
                    help="compiler to use (default: gcc)")
    args = ap.parse_args()

    with open(os.path.join(REPO, "info.toml"), "rb") as f:
        info = tomllib.load(f)

    problems = []
    skipped = 0
    compiler_name = os.path.basename(args.compiler)
    with tempfile.TemporaryDirectory() as build_dir:
        for ex in info["exercises"]:
            rel = os.path.join(ex["dir"], f"{ex['name']}.c")

            # The "must fail as shipped" half only holds for compilers the
            # exercise supports (e.g. gcc-only diagnostics). The solution
            # must still pass with every compiler, so that half always runs.
            supported = ex.get("compilers") is None or any(
                c.lower() == compiler_name.lower() for c in ex["compilers"])
            if not supported:
                skipped += 1
                req = ", ".join(ex["compilers"])
                print(f"  skip exercise {ex['name']:24s} requires: {req}")
            else:
                stage = verify(args.compiler, os.path.join("exercises", rel),
                               ex, build_dir)
                if stage is None:
                    problems.append(
                        f"exercises/{rel}: passes as shipped -- the learner "
                        f"gets a green check without fixing anything")
                    print(f"  FAIL exercise {ex['name']:24s} passes as-is")
                else:
                    print(f"  ok   exercise {ex['name']:24s} fails at: {stage}")

            src = solution_source(rel, build_dir)
            if src is None:
                problems.append(f"solutions/{rel}(.enc): missing")
                print(f"  FAIL solution {ex['name']:24s} missing")
                continue
            stage = verify(args.compiler, src, ex, build_dir)
            if stage is not None:
                problems.append(f"solutions/{rel}: fails at stage: {stage}")
                print(f"  FAIL solution {ex['name']:24s} fails at: {stage}")
            else:
                print(f"  ok   solution {ex['name']:24s} passes")

    n = len(info["exercises"])
    if problems:
        print(f"\n{len(problems)} problem(s) across {n} exercises:")
        for p in problems:
            print(f"  - {p}")
        return 1
    note = f" ({skipped} exercise check(s) skipped: other compiler)" if skipped else ""
    print(f"\nOK: all {n - skipped} exercises fail as shipped, "
          f"all {n} solutions pass.{note}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
