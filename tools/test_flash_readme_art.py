# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("readme_art", Path(__file__).with_name("make-flash-readme-art.py"))
readme_art = importlib.util.module_from_spec(spec)
spec.loader.exec_module(readme_art)


class ReleaseHeaderTests(unittest.TestCase):
    def test_stable_and_preview_headers_link_to_matching_release(self):
        for version, tag, label, status in (
            ("1.0.0", "flash-v1.0.0", "v1.0.0", "正式版"),
            ("0.1.0-pre.7", "flash-v0.1.0-preview.7", "v0.1.0 Pre7", "開発プレビュー"),
        ):
            with self.subTest(version=version), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                (root / "Cargo.toml").write_text(f'[workspace.package]\nversion="{version}"', encoding="utf-8")
                with patch.object(readme_art, "ROOT", root), patch.object(readme_art, "OUT", root / "assets/branding/readme"):
                    header = readme_art.header(root / "chromeos/README.md", "chromeos", "ChromeOS")
                self.assertIn(f"/releases/tag/{tag}\"", header)
                self.assertIn(f'alt="Flash {label}"', header)
                self.assertIn(f'alt="{status}"', header)


if __name__ == "__main__":
    unittest.main()
