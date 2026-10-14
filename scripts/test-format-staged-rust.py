#!/usr/bin/env python3
"""Verify the Rust pre-commit formatter only changes staged Rust files."""

import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
HOOK = ROOT / ".githooks" / "format-staged-rust.sh"


class FormatStagedRustTests(unittest.TestCase):
    def test_formats_staged_file_without_touching_other_targets(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            (repo / "src").mkdir()
            (repo / "Cargo.toml").write_text(
                '[package]\nname = "hook-probe"\nversion = "0.1.0"\nedition = "2024"\n',
                encoding="utf-8",
            )
            staged = repo / "src" / "main.rs"
            spaced = repo / "src" / "file with space.rs"
            child = repo / "src" / "child.rs"
            unstaged = repo / "src" / "lib.rs"
            staged.write_text("mod child;\nfn main(){let value=1;println!(\"{value}\");}\n", encoding="utf-8")
            spaced.write_text("pub fn spaced(){let value=1;let _=value;}\n", encoding="utf-8")
            child.write_text("pub fn child(){let value=1;let _=value;}\n", encoding="utf-8")
            unstaged.write_text("pub fn probe(){let value=1;let _=value;}\n", encoding="utf-8")

            self.run_git(repo, "init", "--quiet")
            self.run_git(repo, "config", "user.name", "Hook Test")
            self.run_git(repo, "config", "user.email", "hook-test@example.invalid")
            self.run_git(repo, "add", "src/main.rs", "src/file with space.rs", "src/child.rs", "src/lib.rs", "Cargo.toml")
            self.run_git(repo, "commit", "--quiet", "-m", "test: add formatter fixture")

            staged.write_text("mod child;\nfn main(){let value=2;println!(\"{value}\");}\n", encoding="utf-8")
            spaced.write_text("pub fn spaced(){let value=2;let _=value;}\n", encoding="utf-8")
            self.run_git(repo, "add", "src/main.rs", "src/file with space.rs")
            spaced_before = spaced.read_bytes()
            child_before = child.read_bytes()
            unstaged_before = unstaged.read_bytes()

            subprocess.run(["bash", str(HOOK)], cwd=repo, check=True)

            self.assertEqual(
                staged.read_text(encoding="utf-8"),
                'mod child;\nfn main() {\n    let value = 2;\n    println!("{value}");\n}\n',
            )
            self.assertNotEqual(spaced.read_bytes(), spaced_before)
            self.assertEqual(
                spaced.read_text(encoding="utf-8"),
                "pub fn spaced() {\n    let value = 2;\n    let _ = value;\n}\n",
            )
            self.assertEqual(child.read_bytes(), child_before)
            self.assertEqual(unstaged.read_bytes(), unstaged_before)

    @staticmethod
    def run_git(repo, *arguments):
        subprocess.run(["git", *arguments], cwd=repo, check=True)


if __name__ == "__main__":
    unittest.main()
