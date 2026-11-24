"""Check that missing-notice exceptions and dependency coverage fail closed."""

import base64
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from assemble import check_android_coverage, clean_licenses
from prepare import apply_exception


class NoticeTests(unittest.TestCase):
    def exception_fixture(self, root):
        crate = root / "example-1.0.0"
        crate.mkdir()
        (crate / "Cargo.toml").write_text('[package]\nname="example"\nversion="1.0.0"\n')
        (crate / ".cargo_vcs_info.json").write_text(json.dumps({"git": {"sha1": "a" * 40}}))
        source = root / "original.txt"
        source.write_text("Synthetic copyright attribution.\n")
        return {"name": "example", "version": "1.0.0", "revision": "a" * 40,
                "files": [{"path": "LICENSE-MIT", "source": str(source),
                           "hash": "sha256-" + base64.b64encode(hashlib.sha256(source.read_bytes()).digest()).decode()}]}

    def test_preserves_original_attribution(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            exception = self.exception_fixture(root)
            apply_exception(root, exception)
            self.assertEqual((root / "example-1.0.0/LICENSE-MIT").read_bytes(),
                             (root / "original.txt").read_bytes())

    def test_stale_revision_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            exception = self.exception_fixture(root)
            exception["revision"] = "b" * 40
            with self.assertRaisesRegex(ValueError, "revision changed"):
                apply_exception(root, exception)

    def test_changed_text_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            exception = self.exception_fixture(root)
            (root / "original.txt").write_text("Different attribution.\n")
            with self.assertRaisesRegex(ValueError, "hash mismatch"):
                apply_exception(root, exception)

    def test_upstream_notice_requires_exception_removal(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            exception = self.exception_fixture(root)
            apply_exception(root, exception)
            with self.assertRaisesRegex(ValueError, "remove the exception"):
                apply_exception(root, exception)

    def test_android_declaration_change_requires_review(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            config = root / "packaging/notices"
            config.mkdir(parents=True)
            declaration = root / "build.gradle.kts"
            declaration.write_text("original dependency declarations")
            receipt = {"scope": "synthetic Android scope", "files": {
                "build.gradle.kts": hashlib.sha256(declaration.read_bytes()).hexdigest()}}
            (config / "android-coverage.json").write_text(json.dumps(receipt))
            self.assertEqual(check_android_coverage(root), receipt["scope"])
            declaration.write_text("new runtime dependency")
            with self.assertRaisesRegex(ValueError, "coverage"):
                check_android_coverage(root)

    def test_uncovered_dependency_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "coverage mismatch"):
            clean_licenses({"licenses": []}, [{"name": "example", "version": "1.0.0", "source": "registry"}])


if __name__ == "__main__":
    unittest.main()
