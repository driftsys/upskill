#!/usr/bin/env python3
"""Format staged text blobs without changing the working tree."""

import os
import subprocess
import sys


FIXTURE_PREFIX = b"tests/fixtures/"
CAPTURED_SCHEMA_PREFIX = b"docs/technotes/agent-customization/schemas/"


def run(arguments, *, input_bytes=None):
    result = subprocess.run(
        arguments,
        input=input_bytes,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        message = result.stderr.decode("utf-8", errors="replace").strip()
        raise RuntimeError(message or f"{arguments[0]} exited with status {result.returncode}")
    return result.stdout


def staged_paths():
    output = run(
        ["git", "diff", "--cached", "--name-only", "-z", "--diff-filter=ACMRT"]
    )
    return [path for path in output.split(b"\0") if path]


def staged_index_entries():
    output = run(["git", "ls-files", "--stage", "-z"])
    entries = {}
    for record in output.split(b"\0"):
        if not record:
            continue
        header, path = record.split(b"\t", 1)
        mode, object_id, stage = header.split(b" ", 2)
        if stage == b"0":
            entries[path] = (mode.decode("ascii"), object_id.decode("ascii"))
    return entries


def format_blob(path, contents):
    if path.endswith(b".rs"):
        return run(
            [
                "rustfmt",
                "--edition",
                "2024",
                "--config",
                "skip_children=true",
                "--emit",
                "stdout",
            ],
            input_bytes=contents,
        )

    if b"\0" in contents:
        return contents
    try:
        contents.decode("utf-8")
    except UnicodeDecodeError:
        return contents

    return run(
        ["prim", "fmt", "--stdin-filepath", os.fsdecode(path)],
        input_bytes=contents,
    )


def main():
    index_entries = staged_index_entries()
    for path in staged_paths():
        # Prim stdin formatting does not apply .primignore exclusions.
        if path.startswith((FIXTURE_PREFIX, CAPTURED_SCHEMA_PREFIX)):
            continue
        entry = index_entries.get(path)
        if entry is None:
            continue
        mode, object_id = entry
        if mode not in {"100644", "100755"}:
            continue

        original = run(["git", "cat-file", "blob", object_id])
        formatted = format_blob(path, original)
        if formatted == original:
            continue

        formatted_object = run(["git", "hash-object", "-w", "--stdin"], input_bytes=formatted)
        run(
            [
                "git",
                "update-index",
                "--add",
                "--cacheinfo",
                f"{mode},{formatted_object.decode('ascii').strip()},{os.fsdecode(path)}",
            ]
        )


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"staged formatting failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
