#!/usr/bin/env python3
"""Simulate a complete learner journey, installation to final green.

This is the end-to-end gate the unit/integration tests can't provide:
it drives the REAL binary through the exact path a learner takes on the
embedded-workspace flow, with no git clone involved:

  1. `cmetal init` materializes a workspace from the embedded curriculum
  2. the first `cmetal list` populates my_exercises/
  3. smoke check: the first exercise FAILS as shipped via the workspace
  4. for every exercise: decode the obfuscated solution, write it as the
     learner's working copy, then `cmetal run <name>` must pass and
     reveal the official solution in my_solutions/
  5. `cmetal verify` exits 0 and the state file records every solved
     exercise

Usage:
    python3 scripts/simulate_learner.py [--binary PATH] [--compiler CC]

Exercises restricted to another compiler (info.toml `compilers`) are
skipped, mirroring what the learner would see.
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
RUN_TIMEOUT = 120


def run(cmd, cwd, check=True):
    r = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True,
                       timeout=RUN_TIMEOUT)
    if check and r.returncode != 0:
        print(r.stdout)
        print(r.stderr, file=sys.stderr)
        sys.exit(f"FAILED ({r.returncode}): {' '.join(map(str, cmd))}")
    return r


def default_binary():
    for profile in ("release", "debug"):
        p = os.path.join(REPO, "target", profile, "cmetal")
        if os.path.exists(p):
            return p
    sys.exit("No cmetal binary found — build one or pass --cmetal PATH")


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--binary", default=None, help="path to the cmetal binary")
    ap.add_argument("--compiler", default="gcc", help="compiler (default: gcc)")
    args = ap.parse_args()
    cmetal = os.path.abspath(args.binary) if args.binary else default_binary()
    cc = args.compiler

    with tempfile.TemporaryDirectory() as tmp:
        ws = os.path.join(tmp, "course")

        # 1. Installation experience: init, no repo anywhere in sight.
        run([cmetal, "init", ws], cwd=tmp)
        assert os.path.exists(os.path.join(ws, "info.toml")), "init: no info.toml"
        assert os.path.exists(os.path.join(ws, ".cmetal", "manifest.json")), \
            "init: no manifest"

        with open(os.path.join(ws, "info.toml"), "rb") as f:
            exercises = tomllib.load(f)["exercises"]
        print(f"workspace ready: {len(exercises)} exercises, compiler {cc}")

        def supported(ex):
            allowed = ex.get("compilers")
            return not allowed or cc.lower() in [c.lower() for c in allowed]

        # 2. First contact populates the working copies.
        run([cmetal, "--compiler", cc, "list"], cwd=ws)
        # The smoke check needs an exercise this compiler can actually
        # run; the curriculum invariant guarantees every exercise starts
        # failing, so the first supported one must fail as shipped.
        first = next(ex for ex in exercises if supported(ex))
        assert os.path.exists(os.path.join(
            ws, "my_exercises", first["dir"], f"{first['name']}.c")), \
            "list did not populate my_exercises/"

        # 3. ...and it must fail through the workspace copy, not just in
        #    the pristine tree.
        r = run([cmetal, "--compiler", cc, "run", first["name"]],
                cwd=ws, check=False)
        assert r.returncode != 0, \
            f"{first['name']} passed as shipped — the journey teaches nothing"

        # 4. Solve everything the way a (very fast) learner would.
        solved, skipped = [], []
        for ex in exercises:
            name, d = ex["name"], ex["dir"]
            if not supported(ex):
                skipped.append(name)
                continue
            with open(os.path.join(ws, "solutions", d, f"{name}.c.enc")) as f:
                solution = decode_bytes(f.read())
            my = os.path.join(ws, "my_exercises", d, f"{name}.c")
            os.makedirs(os.path.dirname(my), exist_ok=True)
            with open(my, "wb") as f:
                f.write(solution)
            run([cmetal, "--compiler", cc, "run", name], cwd=ws)
            reveal = os.path.join(ws, "my_solutions", d, f"{name}.c")
            assert os.path.exists(reveal), f"{name}: solution not revealed"
            solved.append(name)
            print(f"  solved {len(solved):2d}/{len(exercises)}  {name}")

        # 5. The finish line: verify agrees, and the progress file
        #    remembers every step of the journey.
        run([cmetal, "--compiler", cc, "verify"], cwd=ws)
        with open(os.path.join(ws, ".cmetal-state.txt")) as f:
            lines = f.read().splitlines()
        # .cmetal-state.txt layout (see AppState::load/save): header
        # line, blank, current exercise, blank, then one completed
        # exercise per line.
        done = {line.strip() for line in lines[4:] if line.strip()}
        missing = [n for n in solved if n not in done]
        assert not missing, f"not recorded as done: {missing}"

        print(f"\nJourney complete: {len(solved)} solved, "
              f"{len(skipped)} skipped ({', '.join(skipped) or 'none'})")


if __name__ == "__main__":
    main()
