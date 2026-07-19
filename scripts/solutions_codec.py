#!/usr/bin/env python3
"""Pack/unpack the reference solutions as obfuscated .c.enc files.

Solutions live in the repo XOR-ed and base64-encoded so a learner browsing
the tree doesn't get spoiled by accident. This is NOT security — the key
ships with the repo — it's a spoiler shield. cmetal decodes a solution and
reveals it in my_solutions/ once the exercise passes verification.

The encoding must stay in sync with `decode()` in src/solutions.rs.

Usage:
    python3 scripts/solutions_codec.py pack
        Encode every solutions/**/*.c into <name>.c.enc and remove the
        plaintext file. Run this after writing or editing a solution.

    python3 scripts/solutions_codec.py unpack [--to DIR]
        Decode every solutions/**/*.c.enc back to plaintext .c files,
        into solutions/ itself (default, for editing) or into DIR
        (e.g. a temp dir for CI verification).
"""

import argparse
import base64
import os
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SOLUTIONS = os.path.join(REPO, "solutions")

# Must match XOR_KEY in src/solutions.rs
KEY = b"clings-spoiler-shield"


def xor(data):
    return bytes(b ^ KEY[i % len(KEY)] for i, b in enumerate(data))


def encode_bytes(plain):
    return base64.encodebytes(xor(plain)).decode("ascii")


def decode_bytes(text):
    return xor(base64.b64decode("".join(text.split())))


def iter_files(suffix):
    for root, _, files in os.walk(SOLUTIONS):
        for f in sorted(files):
            if f.endswith(suffix):
                yield os.path.join(root, f)


def pack():
    n = 0
    for src in iter_files(".c"):
        with open(src, "rb") as f:
            plain = f.read()
        with open(src + ".enc", "w") as f:
            f.write(encode_bytes(plain))
        os.unlink(src)
        n += 1
    print(f"packed {n} solution(s)")
    if n == 0:
        print("nothing to pack — did you mean 'unpack' first?", file=sys.stderr)
        return 1
    return 0


def unpack(dest):
    n = 0
    for src in iter_files(".c.enc"):
        rel = os.path.relpath(src[: -len(".enc")], SOLUTIONS)
        out = os.path.join(dest, rel)
        os.makedirs(os.path.dirname(out), exist_ok=True)
        with open(src) as f:
            plain = decode_bytes(f.read())
        with open(out, "wb") as f:
            f.write(plain)
        n += 1
    print(f"unpacked {n} solution(s) into {dest}")
    return 0 if n else 1


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("pack")
    p_unpack = sub.add_parser("unpack")
    p_unpack.add_argument("--to", default=SOLUTIONS, metavar="DIR")
    args = ap.parse_args()
    return pack() if args.cmd == "pack" else unpack(args.to)


if __name__ == "__main__":
    sys.exit(main())
