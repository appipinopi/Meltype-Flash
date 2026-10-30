# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).parent / path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


coverage = module("coverage", "benchmark-platforms.py")
resources = module("posix_resources", "performance/process_resources.py")


class CoverageTests(unittest.TestCase):
    def test_empty_results_do_not_claim_mobile_or_desktop_measurements(self):
        with tempfile.TemporaryDirectory() as directory:
            result = coverage.collect(Path(directory))
        self.assertEqual(len(result["coverage"]), 6)
        for row in result["coverage"]:
            self.assertEqual(set(list(row.values())[1:]), {"未測定"})

    def test_native_result_does_not_claim_ime_or_mobile_measurement(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report.json"
            path.write_text(json.dumps(dict(comparison_kind="rust-versions",
                accepted_performance_baseline=True, host_os="Darwin", backend="native",
                machine="Mac CI", measured_at="2026-10-23", old_ref="pre3", new_ref="abc",
                old_version="0.1.0-pre.3", current_version="0.1.0-pre.4",
                fixtures_sha256="123", results=[], scope="C ABI")), encoding="utf-8")
            result = coverage.collect(Path(directory))
            mac = next(r for r in result["coverage"] if r["os"] == "macOS")
            self.assertEqual(mac["native"], "測定済み")
            self.assertEqual(mac["core"], "未測定")
            self.assertEqual(mac["ime_application"], "未測定")
            self.assertTrue(all(r["native"] == "未測定" for r in result["coverage"] if r["os"] != "macOS"))
            data = json.loads(path.read_text())
            data["accepted_performance_baseline"] = False
            path.write_text(json.dumps(data))
            with self.assertRaises(ValueError):
                coverage.collect(Path(directory))


@unittest.skipIf(os.name == "nt", "wait4 is POSIX-only; Windows uses existing Win32 accounting")
class ProcessTests(unittest.TestCase):
    def test_prebuilt_comparison_preserves_posix_execute_permission(self):
        comparison = module("comparison_posix", "compare-rust-benchmarks.py")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            host = root / "host"
            host.write_text(f"#!{sys.executable}\n" +
                "import json,sys\n"
                "case=json.load(open(sys.argv[1]))[0]\n"
                "for i in range(int(sys.argv[2])):\n"
                " print(json.dumps(dict(case=case['name'],sample=i,total_ms=1,"
                "responses=[json.dumps(dict(consumed=True,commits=[],view=None))])))\n")
            host.chmod(0o755)
            output = root / "result"
            args = ["compare", "--old", str(host), "--new", str(host), "--old-ref", "pre3",
                    "--new-ref", "pre4", "--output", str(output), "--samples", "3"]
            with patch.object(sys, "argv", args), patch.object(comparison.benchmark, "fixtures",
                    return_value=[dict(name="short_test", keys=[[13, 0, 0]])]), \
                 patch.object(comparison.benchmark, "command", return_value="rustc test"), patch("builtins.print"):
                comparison.main()
            report = json.loads((output / "report.json").read_text())
            self.assertTrue(os.access(output / "old", os.X_OK))
            self.assertEqual(len(report["rows"]), 12)
            self.assertTrue(report["results"][0]["eligible_for_speed_claim"])

    def test_separate_children_have_separate_peak_rss_and_failure_propagates(self):
        import subprocess
        with tempfile.TemporaryFile(mode="w+") as output:
            common = dict(cwd=Path.cwd(), env=os.environ.copy(), stdout=output)
            big = resources.measure_process([sys.executable, "-c",
                "a=bytearray(96*1024*1024); [a.__setitem__(i,1) for i in range(0,len(a),4096)]; print(len(a))"], **common)
            small = resources.measure_process([sys.executable, "-c", "print('ok')"], **common)
            self.assertGreater(big["peak_working_set_bytes"], small["peak_working_set_bytes"] + 50*1024*1024)
            self.assertGreater(small["cpu_seconds"], 0)
            self.assertIsNone(small["sampled_peak_private_bytes"])
            with self.assertRaises(subprocess.CalledProcessError):
                resources.measure_process([sys.executable, "-c", "raise SystemExit(7)"], **common)


if __name__ == "__main__":
    unittest.main()
