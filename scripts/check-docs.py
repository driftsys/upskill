#!/usr/bin/env python3
"""Check repository documentation links and an isolated mdBook build."""

import hashlib
import json
import re
import shutil
import subprocess
import sys
import tempfile
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit


class HtmlLinks(HTMLParser):
    def __init__(self):
        super().__init__()
        self.links = []
        self.ids = set()

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if "id" in attrs:
            self.ids.add(attrs["id"])
        for key in ("href", "src"):
            if key in attrs:
                self.links.append(attrs[key])


def markdown_links(text):
    """Read inline and reference destinations, excluding fenced examples."""
    lines = []
    fence = None
    for line in text.splitlines():
        marker = re.match(r"^\s*(`{3,}|~{3,})", line)
        if marker:
            value = marker[1]
            if fence is None:
                fence = value
            elif value[0] == fence[0] and len(value) >= len(fence):
                fence = None
            continue
        if fence is None:
            lines.append(line)
    prose = "\n".join(lines)
    return re.findall(r"\]\(<?([^\s)>]+)>?(?:\s+[^)]*)?\)", prose) + re.findall(
        r"^\s*\[[^]]+\]:\s*<?([^\s>]+)", prose, re.MULTILINE
    )


def local_target(source, link):
    url = urlsplit(link)
    if url.scheme or url.netloc or url.path.startswith("/"):
        return None
    return source.parent / unquote(url.path) if url.path else source


def check_markdown(root):
    errors = []
    paths = list((root / "docs").rglob("*.md"))
    paths += [root / name for name in ("README.md", "AGENTS.md", "CONTRIBUTING.md")]
    for path in paths:
        relative = path.relative_to(root)
        if any(part in ("wip", "archive") for part in relative.parts) and path.name != "README.md":
            continue  # Historical task bodies retain their original references.
        for link in markdown_links(path.read_text()):
            target = local_target(path, link)
            if target is not None and not target.exists():
                errors.append(f"{relative}: missing {link}")
    return errors


def check_html(output):
    output = output.resolve()
    pages = {}
    errors = []
    for path in output.rglob("*.html"):
        page = HtmlLinks()
        page.feed(path.read_text())
        pages[path.resolve()] = page
    for path, page in pages.items():
        for link in page.links:
            target = local_target(path, link)
            if target is None:
                continue
            target = target.resolve()
            if target.is_dir():
                target /= "index.html"
            if not target.exists():
                errors.append(f"{path.relative_to(output)}: missing {link}")
                continue
            fragment = unquote(urlsplit(link).fragment)
            if fragment and target in pages and fragment not in pages[target].ids:
                errors.append(f"{path.relative_to(output)}: missing anchor {link}")
    return errors


def check_publication_sources(root):
    excluded = [root / "docs" / name for name in
                ("technotes", "wip", "archive", "superpowers", "research")]
    excluded = [path.resolve() for path in excluded]
    pending = list((root / "docs/book").rglob("*.md"))
    visited = set()
    errors = []
    while pending:
        source = pending.pop().resolve()
        if source in visited:
            continue
        visited.add(source)
        if any(source == path or path in source.parents for path in excluded):
            errors.append(f"Unpublished source included in book: {source}")
            continue
        if not source.is_file():
            continue  # Missing sources are diagnosed by the build.
        text = source.read_text()
        for target in re.findall(r"\{\{#include\s+([^}\s]+)\s*\}\}", text):
            pending.append(source.parent / target.split(":", 1)[0])
        if source.name == "SUMMARY.md":
            for link in markdown_links(text):
                target = local_target(source, link)
                if target is not None:
                    pending.append(target)
    return errors


def check_book(root):
    source_errors = check_publication_sources(root)
    if source_errors:
        return source_errors
    with tempfile.TemporaryDirectory(prefix="upskill-docs-") as temporary:
        staging = Path(temporary)
        shutil.copy(root / "book.toml", staging / "book.toml")
        shutil.copytree(root / "docs", staging / "docs")
        before = {p.relative_to(staging) for p in (staging / "docs").rglob("*")}
        result = subprocess.run(
            ["mdbook", "build", str(staging)], capture_output=True, text=True
        )
        log = result.stdout + result.stderr
        errors = []
        if result.returncode or re.search(r"\b(?:WARN|ERROR)\b", log):
            errors.append(f"mdBook build failed or warned:\n{log}")
        after = {p.relative_to(staging) for p in (staging / "docs").rglob("*")}
        if after != before:
            errors.append("mdBook created missing source files: " + str(sorted(after - before)))
        output = staging / "book"
        if not (output / "index.html").exists():
            errors.append("mdBook did not produce index.html")
        else:
            errors.extend(check_html(output))
            for path in output.rglob("*"):
                relative = path.relative_to(output)
                if any(part in {"technotes", "wip", "archive", "superpowers", "research"} for part in relative.parts):
                    errors.append(f"Unpublished material in book output: {relative}")
            for path in output.rglob("*.html"):
                if "{{#include" in path.read_text():
                    errors.append(f"Unresolved include in {path.name}")
        return errors


def check_snapshots(root):
    errors = []
    for manifest in (root / "docs/technotes").glob("*/schemas/manifest.json"):
        entries = json.loads(manifest.read_text())
        files = {str(p.relative_to(manifest.parent)) for p in manifest.parent.rglob("*.schema.json")}
        names = [entry["file"] for entry in entries]
        if len(set(names)) != len(names) or set(names) != files:
            errors.append(f"{manifest.relative_to(root)}: schema inventory mismatch")
        for entry in entries:
            path = manifest.parent / entry["file"]
            if not path.is_file():
                errors.append(f"Missing schema snapshot: {path}")
                continue
            raw = path.read_bytes()
            json.loads(raw)
            if len(raw) != entry["bytes"] or hashlib.sha256(raw).hexdigest() != entry["sha256"]:
                errors.append(f"Schema snapshot changed: {path.relative_to(root)}")
    return errors


def main():
    root = Path(__file__).resolve().parents[1]
    version = subprocess.run(["mdbook", "--version"], capture_output=True, text=True, check=True)
    if version.stdout.strip() != "mdbook v0.5.4":
        print("Documentation checks require mdBook 0.5.4 (see CONTRIBUTING.md).", file=sys.stderr)
        return 1
    errors = check_markdown(root) + check_book(root) + check_snapshots(root)
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print("Documentation links, rendered book, publication isolation and schema snapshots passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
