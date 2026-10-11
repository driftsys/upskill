#!/usr/bin/env python3
"""Regression checks for documentation failures that mdBook can overlook."""

import hashlib
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True

spec = importlib.util.spec_from_file_location(
    "check_docs", Path(__file__).with_name("check-docs.py")
)
check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check)


class DocumentationChecks(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="upskill-docs-test-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        (self.root / "docs/book").mkdir(parents=True)
        (self.root / "book.toml").write_text('[book]\ntitle = "Test"\nsrc = "docs/book"\n')
        (self.root / "docs/book/SUMMARY.md").write_text('# Summary\n\n- [Intro](intro.md)\n')
        (self.root / "docs/book/intro.md").write_text('# Intro\n\n[Local](#intro)\n')
        for name in ("README.md", "AGENTS.md", "CONTRIBUTING.md"):
            (self.root / name).write_text('# Test\n')

    def test_good_book_and_links(self):
        self.assertEqual(check.check_markdown(self.root), [])
        self.assertEqual(check.check_book(self.root), [])

    def test_source_link_failure_and_fenced_example_exclusion(self):
        folder = self.root / "docs/specification"
        folder.mkdir()
        (folder / "contract.md").write_text(
            '# Test\n\n```md\n[Example](example.md)\n```\n'
            '[Broken](missing.md)\n[Reference][target]\n\n[target]: lost.md\n'
        )
        errors = check.check_markdown(self.root)
        self.assertEqual(len(errors), 2)
        self.assertIn('docs/specification/contract.md: missing missing.md', errors)
        self.assertIn('docs/specification/contract.md: missing lost.md', errors)

    def test_missing_chapter_fails_without_modifying_sources(self):
        chapter = self.root / "docs/book/intro.md"
        chapter.unlink()
        self.assertTrue(check.check_book(self.root))
        self.assertFalse(chapter.exists())

    def test_missing_include_fails(self):
        (self.root / "docs/book/intro.md").write_text('# Intro\n\n{{#include missing.md}}\n')
        self.assertTrue(check.check_book(self.root))

    def test_rendered_anchor_failure(self):
        (self.root / "docs/book/intro.md").write_text('# Intro\n\n[Broken](#missing)\n')
        self.assertTrue(any('missing anchor' in error for error in check.check_book(self.root)))

    def test_changed_schema_snapshot_fails(self):
        folder = self.root / "docs/technotes/research/schemas"
        folder.mkdir(parents=True)
        schema = folder / "example.schema.json"
        raw = b'{"a":1}'
        schema.write_bytes(raw)
        (folder / "manifest.json").write_text(json.dumps([{
            "file": schema.name,
            "bytes": len(raw),
            "sha256": hashlib.sha256(raw).hexdigest(),
        }]))
        self.assertEqual(check.check_snapshots(self.root), [])
        schema.write_bytes(b'{"a":2}')
        self.assertTrue(any('changed' in error for error in check.check_snapshots(self.root)))

    def test_research_include_leak_fails(self):
        folder = self.root / "docs/technotes"
        folder.mkdir()
        (folder / "private.md").write_text("# Research\n")
        # Follow indirect includes too, not only directives in book chapters.
        canonical = self.root / "docs/specification"
        canonical.mkdir()
        (canonical / "intermediate.md").write_text("{{#include ../technotes/private.md}}\n")
        (self.root / "docs/book/intro.md").write_text("# Intro\n\n{{#include ../specification/intermediate.md}}\n")
        self.assertTrue(any('Unpublished source' in error for error in check.check_book(self.root)))

    def test_research_rust_directives_fail(self):
        folder = self.root / "docs/technotes"
        folder.mkdir()
        (folder / "private.rs").write_text("fn main() {}\n")
        for directive in ("rustdoc_include", "playground"):
            with self.subTest(directive=directive):
                (self.root / "docs/book/intro.md").write_text(
                    "# Intro\n\n{{#" + directive + " ../technotes/private.rs}}\n"
                )
                self.assertTrue(any('Unpublished source' in error for error in check.check_book(self.root)))

    def test_research_summary_leak_fails(self):
        folder = self.root / "docs/technotes"
        folder.mkdir()
        (folder / "private.md").write_text("# Research\n")
        (self.root / "docs/book/SUMMARY.md").write_text("# Summary\n\n- [Research](../technotes/private.md)\n")
        self.assertTrue(any('Unpublished source' in error for error in check.check_book(self.root)))

    def test_research_asset_leak_fails(self):
        leaked = self.root / "docs/book/technotes"
        leaked.mkdir()
        (leaked / "evidence.json").write_text('{}')
        self.assertTrue(any('Unpublished material' in error for error in check.check_book(self.root)))


if __name__ == "__main__":
    unittest.main()
