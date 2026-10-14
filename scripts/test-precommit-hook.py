#!/usr/bin/env python3
"""Exercise the configured pre-commit formatter against a temporary repository."""

import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent


class PreCommitHookTests(unittest.TestCase):
    def test_formats_staged_blobs_and_preserves_unstaged_worktree_content(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            shutil.copytree(ROOT / ".githooks", repo / ".githooks")
            (repo / "src").mkdir()
            (repo / "notes").mkdir()
            (repo / "tests" / "fixtures").mkdir(parents=True)
            self.git(repo, "init", "--quiet")
            self.git(repo, "config", "user.name", "Hook Test")
            self.git(repo, "config", "user.email", "hook-test@example.invalid")

            (repo / "src" / "main.rs").write_text(
                'mod child;\nfn main() {\n    let value = 1;\n    println!("{value}");\n}\n',
                encoding="utf-8",
            )
            (repo / "src" / "child.rs").write_text(
                "pub fn child() {\n    let value = 1;\n    let _ = value;\n}\n",
                encoding="utf-8",
            )
            (repo / "src" / "lib.rs").write_text(
                "pub fn library() {\n    let value = 1;\n    let _ = value;\n}\n",
                encoding="utf-8",
            )
            (repo / "src" / "old.rs").write_text(
                "pub fn renamed() {\n    let value = 1;\n    let _ = value;\n}\n",
                encoding="utf-8",
            )
            (repo / "notes" / "staged.md").write_text("# notes\n", encoding="utf-8")
            (repo / "notes" / "config.yaml").write_text("status: baseline\n", encoding="utf-8")
            (repo / "tests" / "fixtures" / "golden.md").write_text(
                "# golden\n", encoding="utf-8"
            )
            (repo / "notes" / "image.bin").write_bytes(b"\x00\x01binary fixture\xff")
            self.git(repo, "add", ".")
            self.git(repo, "commit", "--quiet", "-m", "test: seed hook fixture")

            main = repo / "src" / "main.rs"
            main.write_text('mod child;\nfn main(){let value=2;println!("{value}");}\n', encoding="utf-8")
            self.git(repo, "add", "src/main.rs")
            main.write_text('mod child;\nfn main(){let value=3;println!("{value}");}\n', encoding="utf-8")

            library = repo / "src" / "lib.rs"
            library.write_text("pub fn library(){let value=2;let _=value;}\n", encoding="utf-8")
            library_before = library.read_bytes()

            spaced = repo / "src" / "file with space.rs"
            spaced.write_text("pub fn spaced(){let value=2;let _=value;}\n", encoding="utf-8")

            renamed = repo / "src" / "renamed.rs"
            (repo / "src" / "old.rs").rename(renamed)
            renamed.write_text("pub fn renamed(){let value=2;let _=value;}\n", encoding="utf-8")
            self.git(repo, "add", "-A", "--", "src/old.rs", "src/renamed.rs")

            markdown = repo / "notes" / "staged.md"
            markdown.write_text("# staged  \n", encoding="utf-8")
            config = repo / "notes" / "config.yaml"
            config.write_text("status: staged  \n", encoding="utf-8")
            self.git(
                repo,
                "add",
                "notes/staged.md",
                "notes/config.yaml",
                "tests/fixtures/golden.md",
                "notes/image.bin",
                "src/file with space.rs",
                "src/renamed.rs",
            )
            markdown.write_text("# staged  \n\nunstaged markdown\n", encoding="utf-8")
            markdown_before = markdown.read_bytes()

            fixture = repo / "tests" / "fixtures" / "golden.md"
            fixture.write_text("# golden  \n", encoding="utf-8")
            self.git(repo, "add", "tests/fixtures/golden.md")
            fixture_before = fixture.read_bytes()
            image = repo / "notes" / "image.bin"
            image.write_bytes(b"\x00\x01staged binary fixture\xff")
            self.git(repo, "add", "notes/image.bin")

            result = subprocess.run(
                ["git", "std", "hook", "run", "pre-commit", "--"],
                cwd=repo,
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

            self.assertEqual(
                self.index_blob(repo, "src/main.rs"),
                'mod child;\nfn main() {\n    let value = 2;\n    println!("{value}");\n}\n',
            )
            self.assertEqual(
                self.index_blob(repo, "src/file with space.rs"),
                "pub fn spaced() {\n    let value = 2;\n    let _ = value;\n}\n",
            )
            self.assertEqual(
                self.index_blob(repo, "src/renamed.rs"),
                "pub fn renamed() {\n    let value = 2;\n    let _ = value;\n}\n",
            )
            rename_status = self.git_output(repo, "diff", "--cached", "--name-status")
            self.assertIn("src/old.rs\tsrc/renamed.rs", rename_status)
            self.assertEqual(self.index_blob(repo, "notes/staged.md"), "# staged\n")
            self.assertEqual(self.index_blob(repo, "notes/config.yaml"), "status: staged\n")
            self.assertEqual(
                self.index_blob(repo, "tests/fixtures/golden.md"), "# golden  \n"
            )
            self.assertEqual(
                self.index_blob_bytes(repo, "notes/image.bin"),
                b"\x00\x01staged binary fixture\xff",
            )
            self.assertEqual(main.read_text(encoding="utf-8"), 'mod child;\nfn main(){let value=3;println!("{value}");}\n')
            self.assertEqual(markdown.read_bytes(), markdown_before)
            self.assertEqual(library.read_bytes(), library_before)
            self.assertIn("notes/staged.md", self.git_output(repo, "diff", "--name-only").splitlines())
            self.assertIn("src/main.rs", self.git_output(repo, "diff", "--name-only").splitlines())

    @classmethod
    def git(cls, repo, *arguments):
        subprocess.run(["git", *arguments], cwd=repo, check=True)

    @classmethod
    def git_output(cls, repo, *arguments):
        return subprocess.check_output(["git", *arguments], cwd=repo, text=True)

    @classmethod
    def index_blob(cls, repo, path):
        return subprocess.check_output(["git", "show", f":{path}"], cwd=repo, text=True)

    @classmethod
    def index_blob_bytes(cls, repo, path):
        return subprocess.check_output(["git", "show", f":{path}"], cwd=repo)


if __name__ == "__main__":
    unittest.main()
