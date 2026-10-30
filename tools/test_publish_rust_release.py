# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("publisher", Path(__file__).with_name("publish-rust-release.py"))
publisher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(publisher)


class PresentationTests(unittest.TestCase):
    def assets(self, directory):
        for name in publisher.PACKAGES + publisher.DOCUMENTS[:2]:
            (directory / name).write_bytes(b"fixture")
        (directory / "SHA256SUMS.txt").write_text("".join(
            f"{hashlib.sha256(b'fixture').hexdigest()}  {name}\n" for name in publisher.PACKAGES), encoding="utf-8")

    def test_only_all_platforms_licenses_and_sums_are_accepted(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            self.assets(directory)
            publisher.validate_assets(directory)
            for extra in ("speed-1.png", "BUILD-PROVENANCE.json", "README-iOS.md", "RELEASE-NOTES.md", "benchmark.zip", "Meltype-flash-Windows.zip"):
                with self.subTest(extra=extra):
                    (directory / extra).write_bytes(b"extra")
                    with self.assertRaises(ValueError):
                        publisher.validate_assets(directory)
                    (directory / extra).unlink()
            (directory / publisher.PACKAGES[6]).unlink()
            with self.assertRaises(ValueError):
                publisher.validate_assets(directory)

    def test_tampered_package_fails_before_publication(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            self.assets(directory)
            (directory / publisher.PACKAGES[0]).write_bytes(b"changed")
            with self.assertRaisesRegex(ValueError, "SHA256"):
                publisher.validate_assets(directory)

    def test_windows_newlines_do_not_split_table_rows(self):
        text = "# Title\r\r\n\r\n|OS|File|\r\r\n|---|---|\r\r\n|Windows|app.zip|\r\r\n"
        fixed = publisher.normalize_markdown(text)
        self.assertIn("|OS|File|\n|---|---|\n|Windows|app.zip|", fixed)
        self.assertNotIn("\r", fixed)

    def test_body_has_all_os_and_inline_images_without_benchmark_download_links(self):
        benchmark = "# 測定\n\n## 全入力の中央値\n\n|入力|値|\n|---|---|\n|sample|1|\n"
        title, body = publisher.compose_body("flash-v0.1.0-preview.4", "a" * 40, benchmark, "b" * 40, "fix: table\nfix: `quoted` <tag>")
        self.assertEqual(title, "Meltype-flash v0.1.0 Pre4")
        for os_name in ("Windows", "Linux", "macOS", "Android", "ChromeOS", "iOS"):
            self.assertIn(os_name, body)
        self.assertEqual(body.count("https://raw.githubusercontent.com/"), 10)
        self.assertIn("/" + "b" * 40 + "/flash-v0.1.0-preview.4/", body)
        self.assertNotIn("releases/download/flash-v0.1.0-preview.4/cpu.png", body)
        self.assertNotIn("[全測定", body)
        self.assertIn("&lt;tag&gt;", body)

    def test_github_rendering_requires_tables_and_ten_images(self):
        with patch.object(publisher, "gh", return_value='<table role="table"></table>' * 3 + '<img src="a">' * 10):
            publisher.validate_rendered("body")
        for html in ("<p>broken table</p>", "<table>" * 3, '<table role="table"></table>' * 3 + '<img src="a">' * 5):
            with patch.object(publisher, "gh", return_value=html), self.assertRaises(ValueError):
                publisher.validate_rendered("body")

    def test_benchmark_must_match_source_report_and_charts(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            (directory / "charts").mkdir()
            report = {"comparison_kind": "rust-versions", "new_commit": "a" * 40, "old_commit": "b" * 40,
                      "binaries_unchanged": True, "sources_unchanged": True, "accepted_performance_baseline": True,
                      "results": [{"case": "sample"}], "process_resources": [{}],
                      "versions": ["csharp", "old", "new"]}
            report_file = directory / "report.json"
            report_file.write_text(json.dumps(report), encoding="utf-8")
            summary_file = directory / "charts/summary.json"
            summary_file.write_text(json.dumps({"full_report_sha256": hashlib.sha256(report_file.read_bytes()).hexdigest()}), encoding="utf-8")
            (directory / "charts/README.md").write_text("## 全入力の中央値\n\n## CPU・RAMのプロセス中央値\n\n## 表示範囲外の測定値\n\n|項目|値|\n|---|---|\n|sample|999|\n\n## 全画像\n\n[raw](data.csv)", encoding="utf-8")
            for name in publisher.IMAGES:
                (directory / "charts" / name).write_bytes(b"\x89PNG\r\n\x1a\nfixture")
            _, body = publisher.validate_benchmark(directory, "a" * 40)
            self.assertNotIn("data.csv", body)
            self.assertIn("表示範囲外の測定値", body)
            self.assertIn("|sample|999|", body)
            with self.assertRaises(ValueError):
                publisher.validate_benchmark(directory, "c" * 40)
            summary_file.write_text('{"full_report_sha256":"stale"}', encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "一致"):
                publisher.validate_benchmark(directory, "a" * 40)

    def test_media_commit_uses_only_image_branch_and_preserves_parent(self):
        calls = []

        def fake_api(path, payload=None, method=None):
            calls.append((path, payload, method))
            if "matching-refs" in path:
                return [{"ref": "refs/heads/release-media", "object": {"sha": "parent"}}]
            if path == "git/commits/parent":
                return {"tree": {"sha": "old-tree"}}
            return {"sha": "media-commit"}

        with tempfile.TemporaryDirectory() as temporary, patch.object(publisher, "api", side_effect=fake_api):
            charts = Path(temporary)
            for name in publisher.IMAGES:
                (charts / name).write_bytes(b"png")
            self.assertEqual(publisher.publish_images("flash-v0.1.0-preview.4", charts), "media-commit")
        tree = next(payload for path, payload, _ in calls if path == "git/trees")
        self.assertEqual(tree["base_tree"], "old-tree")
        self.assertEqual(len(tree["tree"]), 10)
        ref, payload, method = calls[-1]
        self.assertEqual((ref, method, payload["force"]), ("git/refs/heads/release-media", "PATCH", False))


if __name__ == "__main__":
    unittest.main()
