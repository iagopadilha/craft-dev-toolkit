#!/usr/bin/env python3
"""Reports differences between the crates this repository shares with Rolo Loop (the local clone of iagopadilha/rolo-loop) (`egress-policy`, `pycompat`).

Each repository carries its own copy; this script compares the files of the two copies byte by byte (line endings aside) so a change made in one
is not forgotten in the other.

Usage: python tools/crate_sync_check.py [--rolo-loop ../claude-maestro]  (the local folder of the rolo-loop clone)
Exit 0 when the copies are identical, 1 when they differ, 2 when the other repository is not there.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(HERE)
SHARED = ("egress-policy", "pycompat")


def files_of(root):
    found = {}
    for current, dirs, names in os.walk(root):
        dirs[:] = [d for d in dirs if d not in ("target", ".git")]
        for name in names:
            path = os.path.join(current, name)
            with open(path, "rb") as handle:
                found[os.path.relpath(path, root).replace("\\", "/")] = handle.read().replace(b"\r\n", b"\n")
    return found


def main():
    other = os.path.abspath(sys.argv[sys.argv.index("--rolo-loop") + 1]) if "--rolo-loop" in sys.argv else os.path.join(os.path.dirname(REPO), "claude-maestro")
    if not os.path.isdir(os.path.join(other, "crates")):
        print("Rolo Loop clone not found at", other)
        return 2
    different = 0
    for crate in SHARED:
        ours, theirs = files_of(os.path.join(REPO, "crates", crate)), files_of(os.path.join(other, "crates", crate))
        for name in sorted(set(ours) | set(theirs)):
            if ours.get(name) != theirs.get(name):
                different += 1
                state = "only here" if name not in theirs else ("only in rolo-loop" if name not in ours else "differs")
                print("%s/%s: %s" % (crate, name, state))
    print("shared crates identical" if not different else "%d file(s) differ" % different)
    return 1 if different else 0


if __name__ == "__main__":
    sys.exit(main())
