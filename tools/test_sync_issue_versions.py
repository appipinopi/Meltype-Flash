# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("versions", Path(__file__).with_name("sync-issue-versions.py"))
versions = importlib.util.module_from_spec(spec)
spec.loader.exec_module(versions)


class VersionFormTests(unittest.TestCase):
    def test_both_forms_keep_report_fields_and_refresh_only_version(self):
        for name in versions.FORMS:
            text = (versions.ROOT / ".github/ISSUE_TEMPLATE" / name).read_text(encoding="utf-8")
            before, rest = text.split("    id: version\n", 1)
            after = rest[rest.index("  - type:"):]
            updated = versions.update(text, ["flash-v1.0.1", "flash-v1.0.0"])
            self.assertTrue(updated.startswith(before))
            self.assertTrue(updated.endswith(after))
            self.assertIn('        - "flash-v1.0.1"\n        - "flash-v1.0.0"', updated)
            self.assertIn('        - "そのほか / 開発版"', updated)
            self.assertEqual(updated, versions.update(updated, ["flash-v1.0.1", "flash-v1.0.0"]))

    def test_missing_version_does_not_silently_modify_a_different_field(self):
        with self.assertRaises(ValueError):
            versions.update("body:\n  - type: input\n    id: os\n", ["flash-v1.0.1"])


if __name__ == "__main__":
    unittest.main()
