#!/usr/bin/env python3
"""Drive the cmetal watch-mode TUI like a real user, solving every exercise.

For each exercise the bot:
  1. waits for cmetal to report the failure,
  2. presses 'h' to read the first hint (just like a stuck human would),
  3. "edits" the exercise by copying the reference solution over the
     working copy in my_exercises/ (created by cmetal on startup),
  4. waits for the file watcher to recompile and report success,
  5. presses 'n' to advance.

It runs against the real binary through a pty, so the whole pipeline is
exercised end-to-end: crossterm raw mode, the notify file watcher, the
compiler stages and the progress state file.

By default the my_exercises/ workspace and the progress state are restored
when the run finishes (pass --keep to leave everything solved). The
pristine exercises/ tree is never touched.

Usage:
    python3 scripts/autosolver.py [--keep] [--no-hints] [--timeout SECONDS]
"""

import argparse
import errno
import fcntl
import os
import pty
import re
import select
import shutil
import signal
import struct
import subprocess
import sys
import termios
import time
import tomllib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from solutions_codec import decode_bytes  # noqa: E402

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
WORKSPACE = os.path.join(REPO, "my_exercises")
SOLUTIONS = os.path.join(REPO, "solutions")
STATE_FILE = os.path.join(REPO, ".cmetal-state.txt")
BINARY = os.path.join(REPO, "target", "debug", "cmetal")

ANSI_RE = re.compile(r"\x1b\[[0-9;?]*[A-Za-z]|\x1b[()][AB0]|[\r\x07]")

WELCOME = "Press any key to start..."
SUCCESS = "Press 'n' to move to the next exercise."
FAILED = "failed at stage"
DONE = "All exercises completed!"


def write_solution(rel, dst):
    """Write the reference solution over the workspace copy, decoding the
    obfuscated .c.enc (or using a plaintext .c if one is unpacked)."""
    plain = os.path.join(SOLUTIONS, rel)
    if os.path.exists(plain):
        shutil.copy(plain, dst)
        return
    with open(plain + ".enc") as f:
        data = decode_bytes(f.read())
    with open(dst, "wb") as f:
        f.write(data)


def load_exercise_paths():
    with open(os.path.join(REPO, "info.toml"), "rb") as f:
        info = tomllib.load(f)
    return {
        ex["name"]: os.path.join(ex["dir"], f"{ex['name']}.c")
        for ex in info["exercises"]
    }


class CmetalBot:
    def __init__(self, timeout):
        self.timeout = timeout
        self.buf = ""
        env = dict(os.environ, TERM="xterm-256color")
        self.master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 100, 0, 0))
        self.proc = subprocess.Popen(
            [BINARY], cwd=REPO, env=env,
            stdin=slave, stdout=slave, stderr=slave, close_fds=True,
        )
        os.close(slave)

    def _read_chunk(self, deadline):
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            return False
        ready, _, _ = select.select([self.master], [], [], remaining)
        if not ready:
            return False
        try:
            data = os.read(self.master, 65536)
        except OSError as e:
            if e.errno == errno.EIO:  # pty closed: process exited
                return False
            raise
        self.buf += ANSI_RE.sub("", data.decode("utf-8", "replace"))
        return True

    def wait_for(self, *markers):
        """Read until one of the markers appears; return the marker found."""
        deadline = time.monotonic() + self.timeout
        while True:
            for m in markers:
                if m in self.buf:
                    return m
            if not self._read_chunk(deadline):
                if self.proc.poll() is not None and DONE in self.buf:
                    return DONE
                raise TimeoutError(
                    f"none of {markers!r} appeared; last screen:\n{self.buf[-2000:]}"
                )

    def current_exercise(self):
        names = re.findall(r"Exercise: (\w+)", self.buf)
        if not names:
            raise RuntimeError(f"no exercise name on screen:\n{self.buf[-2000:]}")
        return names[-1]

    def press(self, key):
        os.write(self.master, key.encode())

    def clear(self):
        self.buf = ""

    def close(self):
        if self.proc.poll() is None:
            self.proc.terminate()
            try:
                self.proc.wait(5)
            except subprocess.TimeoutExpired:
                self.proc.kill()
        os.close(self.master)


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--keep", action="store_true",
                    help="leave the exercises solved instead of restoring them")
    ap.add_argument("--no-hints", action="store_true",
                    help="skip pressing 'h' before solving each exercise")
    ap.add_argument("--timeout", type=float, default=60,
                    help="seconds to wait for each screen (default: 60)")
    args = ap.parse_args()

    subprocess.run(["cargo", "build", "--quiet"], cwd=REPO, check=True)
    paths = load_exercise_paths()

    # Move any in-progress workspace aside; cmetal recreates a fresh one
    # on startup from the pristine exercises.
    backup = None
    if not args.keep and os.path.isdir(WORKSPACE):
        backup = os.path.join(REPO, "target", "autosolver-workspace-backup")
        shutil.rmtree(backup, ignore_errors=True)
        shutil.move(WORKSPACE, backup)
    if os.path.exists(STATE_FILE):
        os.unlink(STATE_FILE)

    bot = CmetalBot(args.timeout)
    solved, attempts = [], {}
    try:
        bot.wait_for(WELCOME)
        bot.press(" ")
        bot.clear()

        while True:
            marker = bot.wait_for(SUCCESS, FAILED, DONE)
            if marker == DONE:
                break
            name = bot.current_exercise()

            if marker == SUCCESS:
                solved.append(name)
                print(f"  [{len(solved):2}/{len(paths)}] {name}: verde, premo 'n'")
                bot.clear()
                bot.press("n")
                continue

            attempts[name] = attempts.get(name, 0) + 1
            if attempts[name] > 2:
                raise RuntimeError(f"{name} still failing after applying the solution")
            if not args.no_hints:
                bot.press("h")
                time.sleep(0.3)
            print(f"  {name}: rosso, leggo l'hint e applico la soluzione...")
            bot.clear()
            write_solution(paths[name], os.path.join(WORKSPACE, paths[name]))

        print(f"\n  {DONE} ({len(solved)}/{len(paths)} esercizi)")
        ok = len(solved) == len(paths)
        if not ok:
            print("  ATTENZIONE: non tutti gli esercizi risultano risolti!")
        return 0 if ok else 1
    finally:
        bot.close()
        if not args.keep:
            shutil.rmtree(WORKSPACE, ignore_errors=True)
            shutil.rmtree(os.path.join(REPO, "my_solutions"), ignore_errors=True)
            if backup:
                shutil.move(backup, WORKSPACE)
            if os.path.exists(STATE_FILE):
                os.unlink(STATE_FILE)
            print("  Workspace e stato ripristinati.")


if __name__ == "__main__":
    signal.signal(signal.SIGINT, lambda *_: sys.exit(130))
    sys.exit(main())
