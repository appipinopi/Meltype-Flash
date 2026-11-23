# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""The CI result gate must not mistake adb's zero exit code for passing tests."""
import unittest
from instrument import verify_report


class ReportTests(unittest.TestCase):
    def test_required_completed_tests(self):
        for count in (4,5,6,7,8,9,10,11,12,13,14):
            verify_report("INSTRUMENTATION_STATUS_CODE: 1\nINSTRUMENTATION_STATUS_CODE: 0\n" * count
                          + f"OK ({count} tests)\nINSTRUMENTATION_CODE: -1\n", count)

    def test_empty_success_is_not_a_test_run(self):
        with self.assertRaises(RuntimeError):
            verify_report("OK (0 tests)\nINSTRUMENTATION_CODE: -1\n", 4)

    def test_skips_do_not_satisfy_required_tests(self):
        with self.assertRaises(RuntimeError):
            verify_report("INSTRUMENTATION_STATUS_CODE: 0\n" * 3
                          + "INSTRUMENTATION_STATUS_CODE: -3\nOK (4 tests)\nINSTRUMENTATION_CODE: -1\n", 4)

    def test_failure_or_crash_is_rejected(self):
        for report in ("FAILURES!!!\nINSTRUMENTATION_CODE: -1\n",
                       "INSTRUMENTATION_FAILED: Process crashed\n", ""):
            with self.assertRaises(RuntimeError):
                verify_report(report, 4)

    def test_misleading_summary_does_not_hide_failed_status(self):
        with self.assertRaises(RuntimeError):
            verify_report("INSTRUMENTATION_STATUS_CODE: 0\n" * 4
                          + "INSTRUMENTATION_STATUS_CODE: -2\nOK (4 tests)\nINSTRUMENTATION_CODE: -1\n", 4)


if __name__ == "__main__":
    unittest.main(verbosity=2)
