#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""第2弾コア比較ベンチ (core_bench_plan / core_bench_runner / core_targets.json) のユニットテスト。

ネットワーク・dotnet・cargo は使わない (ソース解析と JSON の組み立てだけを検証する)。
"""
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

HERE = Path(__file__).parent
BENCH_DIR = HERE / "ime-benchmark"
sys.path.insert(0, str(BENCH_DIR))


def _load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


core_bench_plan = _load("core_bench_plan", BENCH_DIR / "core_bench_plan.py")
core_bench_runner = _load("core_bench_runner", BENCH_DIR / "core_bench_runner.py")

TARGETS = json.loads((BENCH_DIR / "core_targets.json").read_text(encoding="utf-8"))


class TargetsTest(unittest.TestCase):
    def test_required_fields_and_unique_ids(self):
        ids = set()
        for target in TARGETS["targets"]:
            for key in ("id", "name", "label", "repo", "ref", "kind", "notes"):
                self.assertTrue(target.get(key), f"{target.get('id')}: {key} が空")
            self.assertNotIn(target["id"], ids, "ID が重複")
            ids.add(target["id"])
            self.assertIn(target["kind"], {"csharp", "rust", "monakime", "kswime"})
            self.assertRegex(target["name"], r"^[a-z0-9][a-z0-9.-]*$", "artifact 名に使える文字だけにする")
            self.assertIsInstance(target.get("layer_b", True), bool)
            if target["kind"] == "csharp":
                self.assertIn(target.get("generation"), (1, 2, 3))

    def test_excluded_entries_have_reasons(self):
        excluded = TARGETS["excluded"]
        self.assertGreaterEqual(len(excluded), 10)
        for item in excluded:
            self.assertTrue(item.get("label"))
            self.assertGreater(len(item.get("reason", "")), 10, "対象外の理由を書く")

    def test_spreadsheet_projects_are_covered(self):
        """スプレッドシートの主要プロジェクトが、測定対象か対象外のどちらかに入っている。"""
        covered = {t["repo"].lower() for t in TARGETS["targets"]} | {e["repo"].lower() for e in TARGETS["excluded"]}
        for repo in (
            "yksr-melt/Meltype",
            "appipinopi/Meltype-Flash",
            "roccho-dev/Meltype",
            "AoneSenbongi/Meltype",
            "KoheiTakada/Meltype",
            "sakusdev/Meltype-android",
            "H4Y4/Meltype",
            "hkrhd/Meltype",
            "koyashimano/Meltype",
            "Serugu/Meltype-Zenzai-Vulkan",
            "kazuki-N5/Meltype-jev",
            "ballban/Meltype",
            "cwysms/Meltype",
            "hrmcngs/Meltype",
            "Ginji001/Meltype",
            "lnkiai/Meltype",
            "xelaods/Meltype",
            "oto-lab/Galtype",
            "takumi-zer0/Meltype",
            "masuidrive/Meltype",
            "kumapuni/Meltype-Intel-Mac",
            "kurimochi/Meltype",
            "hisiragi/Meltype-WebEngine",
            "whitewater-png/Meltype-for-Mac",
            "AoneSenbongi/Meltype-Android",
            "Fugu0141/Meltia",
            "345ml/meltype-android",
            "patproport/Meltype-Enhanced",
            "sp9r/meltype-android",
            "hogehonya/fcitx5-meltype",
            "ytani01/meltype-fcitx5",
            "araramaa/MeltypeMC",
            "YoshiaKefasu/JustType",
            "rirannda/MonakIME",
            "k3-yamada/kswime",
            "mei28/kaede",
        ):
            self.assertIn(repo.lower(), covered, f"{repo} の扱いが決まっていない")

    def test_upstream_versions_and_flash_versions(self):
        by_id = {t["id"]: t for t in TARGETS["targets"]}
        self.assertEqual(by_id["upstream_v130"]["ref"], "v1.3.0")
        self.assertEqual(by_id["upstream_v160"]["ref"], "v1.6.0")
        self.assertEqual(by_id["upstream_v171"]["ref"], "v1.7.1")
        self.assertEqual(by_id["flash_1127"]["ref"], "flash-v0001.0001.0027")
        self.assertEqual(by_id["flash_2pre"]["ref"], "codex/2.0.0-pre")


class PlanTest(unittest.TestCase):
    def test_matrix_entries_only_carry_target_and_name(self):
        matrix = core_bench_plan.build(TARGETS)
        self.assertEqual(len(matrix), len(TARGETS["targets"]))
        for entry in matrix:
            self.assertEqual(set(entry), {"target", "name"})
        names = [entry["name"] for entry in matrix]
        self.assertEqual(len(names), len(set(names)), "artifact 名が重複")

    def test_only_filters_by_id_name_and_label(self):
        matrix = core_bench_plan.build(TARGETS, "upstream_v171,flash-1.1.27,本家 v1.6.0")
        self.assertEqual([entry["target"] for entry in matrix], ["upstream_v171", "flash_1127", "upstream_v160"])

    def test_only_keeps_requested_order_without_duplicates(self):
        matrix = core_bench_plan.build(TARGETS, "kswime,kswime,monakime")
        self.assertEqual([entry["target"] for entry in matrix], ["kswime", "monakime"])

    def test_empty_only_means_everything(self):
        self.assertEqual(core_bench_plan.build(TARGETS, ""), core_bench_plan.build(TARGETS, None))

    def test_unknown_only_fails_with_the_known_ids(self):
        with self.assertRaises(SystemExit) as caught:
            core_bench_plan.build(TARGETS, "nope")
        self.assertIn("upstream_v171", str(caught.exception))

    def test_github_output_writes_matrix(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "github_output"
            output.write_text("", encoding="utf-8")
            argv = ["core_bench_plan.py", "--only", "kswime", "--github-output"]
            with mock.patch.object(sys, "argv", argv), mock.patch.dict(os.environ, {"GITHUB_OUTPUT": str(output)}):
                core_bench_plan.main()
            line = output.read_text(encoding="utf-8").strip()
            self.assertTrue(line.startswith("core="))
            self.assertEqual(json.loads(line[len("core="):]), [{"target": "kswime", "name": "kswime"}])

    def test_describe_lists_excluded_reasons(self):
        text = core_bench_plan.describe(TARGETS, TARGETS["targets"][:2])
        self.assertIn("core jobs: 2", text)
        self.assertIn("対象外:", text)
        self.assertIn("Mithril-3d/Meltype", text)

    def test_load_targets_rejects_broken_entries(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "targets.json"
            path.write_text(json.dumps({"targets": [{"id": "x", "name": "x", "repo": "a/b", "ref": "main", "kind": "csharp"}]}), encoding="utf-8")
            with self.assertRaises(SystemExit):
                core_bench_plan.load_targets(path)


class GenerationTest(unittest.TestCase):
    def _tree(self, directory, files):
        root = Path(directory)
        for name, body in files.items():
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(body, encoding="utf-8")
        return root

    def test_gen1_composition_options(self):
        with tempfile.TemporaryDirectory() as directory:
            root = self._tree(directory, {"src/Meltype.Core/Composition/MeltypeSession.cs": "public MeltypeSession(CompositionDetector d, IKanjiConverter c, CompositionOptions options, Func<Settings> s)"})
            self.assertEqual(core_bench_runner.detect_generation(root), 1)

    def test_gen2_composition_services_wins_over_ime_host(self):
        with tempfile.TemporaryDirectory() as directory:
            root = self._tree(
                directory,
                {
                    "src/Meltype.Core/Composition/MeltypeSession.cs": "public MeltypeSession(CompositionDetector d, IKanjiConverter c, CompositionServices services, Func<Settings> s)",
                    "src/Meltype.Core/Ime/ImeHost.cs": "// v1.6 にも ImeHost.cs がある",
                },
            )
            self.assertEqual(core_bench_runner.detect_generation(root), 2)

    def test_gen3_without_meltypesession(self):
        with tempfile.TemporaryDirectory() as directory:
            root = self._tree(directory, {"src/Meltype.Core/Ime/ImeHost.cs": "public sealed class ImeHost"})
            self.assertEqual(core_bench_runner.detect_generation(root), 3)

    def test_unknown_layout(self):
        with tempfile.TemporaryDirectory() as directory:
            self.assertEqual(core_bench_runner.detect_generation(Path(directory)), 0)


class CorpusTest(unittest.TestCase):
    def test_cases_are_typed_as_romaji_with_kana_targets(self):
        cases = core_bench_runner.load_cases()
        self.assertEqual(len(cases), 10000)
        self.assertEqual(cases[0]["id"], min(case["id"] for case in cases))
        for case in cases[:50]:
            self.assertFalse(any("ア" <= c <= "ン" or "ぁ" <= c <= "ん" for c in case["typed"]), case["typed"])
            self.assertTrue(case["typed"])
            self.assertTrue(case["kana"])
            self.assertTrue(case["expected"])

    def test_limit_keeps_the_first_ids(self):
        cases = core_bench_runner.load_cases(limit="12")
        self.assertEqual(len(cases), 12)
        self.assertEqual(cases[-1]["id"], 12)

    def test_write_cases_keeps_typed_only(self):
        cases = core_bench_runner.load_cases(limit="3")
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "cases.jsonl"
            core_bench_runner.write_cases(path, cases)
            rows = [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()]
            self.assertEqual(len(rows), 3)
            self.assertEqual(set(rows[0]), {"id", "typed"})

    def test_missing_corpus_is_an_error(self):
        with self.assertRaises(core_bench_runner.BenchError):
            core_bench_runner.load_cases("/nonexistent-corpus")


class HarnessInstallTest(unittest.TestCase):
    def test_variants_exist_and_are_installed(self):
        for variant in ("gen1", "gen2", "gen3", "gen3empty"):
            with tempfile.TemporaryDirectory() as directory:
                project_dir = Path(directory) / "harness"
                core = Path(directory) / "repo/src/Meltype.Core/Meltype.Core.csproj"
                core.parent.mkdir(parents=True)
                core.write_text("<Project />", encoding="utf-8")
                core_bench_runner.install_csharp_harness(project_dir, variant, core)
                self.assertTrue((project_dir / "CoreBench.csproj").is_file())
                self.assertTrue((project_dir / "Program.cs").is_file())
                project = (project_dir / "CoreBench.csproj").read_text(encoding="utf-8")
                self.assertIn(str(core.resolve()), project)
                self.assertNotIn("__CORE_PROJECT__", project)
                self.assertIn("class", (project_dir / "Program.cs").read_text(encoding="utf-8"))

    def test_project_reference_is_a_placeholder_until_installed(self):
        text = (BENCH_DIR / "harness/csharp/CoreBench.csproj").read_text(encoding="utf-8")
        self.assertIn("__CORE_PROJECT__", text)
        self.assertIn("net10.0", text)
        self.assertIn("EnableDefaultCompileItems", text)

    def test_gen3_harness_uses_the_ime_host_protocol(self):
        text = (BENCH_DIR / "harness/csharp/Program.gen3.cs").read_text(encoding="utf-8")
        self.assertIn("new ImeHost(", text)
        self.assertIn("InstanceDestroyed", text)
        self.assertIn("Version = ImeHost.Version", text)
        self.assertIn("public InputMode? InputMethod => null;", text)

    def test_gen1_harness_creates_a_default_session(self):
        text = (BENCH_DIR / "harness/csharp/Program.gen1.cs").read_text(encoding="utf-8")
        self.assertIn("MeltypeSession.CreateDefault(converter, null, null)", text)
        self.assertIn("HandleKey(0x0D", text)

    def test_gen2_harness_uses_runtime_sessions(self):
        text = (BENCH_DIR / "harness/csharp/Program.gen2.cs").read_text(encoding="utf-8")
        self.assertIn("new MeltypeRuntime(dataDirectory)", text)
        self.assertIn("runtime.CreateSession(converter)", text)

    def test_csharp_executable_is_found(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            dll = root / "bin/Release/net10.0/CoreBench.dll"
            dll.parent.mkdir(parents=True)
            dll.write_text("", encoding="utf-8")
            self.assertEqual(core_bench_runner.csharp_executable(root), dll)
            self.assertIsNone(core_bench_runner.csharp_executable(root / "nowhere"))

    def test_rust_harness_matches_session_api(self):
        text = (BENCH_DIR / "harness/rust/core_bench.rs").read_text(encoding="utf-8")
        for fragment in ("Session::default()", "session.key(", "session.commit()", "session.settings.set(", "session.set_context("):
            self.assertIn(fragment, text)


class ParsingTest(unittest.TestCase):
    CASES = [{"id": 7, "typed": "kyou"}, {"id": 8, "typed": "hai"}]

    def test_read_jsonl_skips_broken_lines(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "out.jsonl"
            path.write_text('{"id":1,"enter":"きょう","space":"今日","ms":2.5}\nnot json\n{"id":2}\n', encoding="utf-8")
            rows = core_bench_runner.read_jsonl(path)
            self.assertEqual(rows[1]["enter"], "きょう")
            self.assertIn(2, rows)

    def test_parse_monakime_uses_segments(self):
        output = json.dumps({"candidates": ["今日は", "きょうは"], "segments": [{"reading": "きょうは", "text": "今日は"}]})
        rows = core_bench_runner.parse_monakime(output + "\n", self.CASES)
        self.assertEqual(rows[7]["enter"], "今日は")

    def test_parse_monakime_falls_back_to_candidates(self):
        output = json.dumps({"candidates": ["変換"]})
        rows = core_bench_runner.parse_monakime(output, self.CASES)
        self.assertEqual(rows[7]["enter"], "変換")

    def test_parse_kswime_maps_one_line_per_case(self):
        rows = core_bench_runner.parse_kswime("きょう\nはい\n", self.CASES)
        self.assertEqual(rows[8]["enter"], "はい")

    def test_merge_pairs_layers_by_id(self):
        cases = [{"id": 1}, {"id": 2}]
        layers = {"a": {1: {"enter": "かな"}}, "b": {1: {"enter": "漢字", "space": "漢字", "ms": 3.0}, 2: {"enter": "二", "space": "二", "ms": 5.0}}}
        results = core_bench_runner.merge(cases, layers)
        self.assertEqual(results[0], {"id": 1, "a": "かな", "b": "漢字", "bs": "漢字", "ms": 3.0})
        self.assertEqual(results[1]["a"], "")
        self.assertEqual(results[1]["ms"], 5.0)

    def test_summarize_reports_percentiles(self):
        results = [{"ms": ms} for ms in (1.0, 2.0, 3.0, 4.0, 100.0)]
        summary = core_bench_runner.summarize(results, [{}] * 5)
        self.assertEqual(summary["cases"], 5)
        self.assertEqual(summary["p50_ms"], 3.0)
        self.assertEqual(summary["p95_ms"], 100.0)

    def test_tail_keeps_the_end_of_long_logs(self):
        text = "x" * 5000
        shortened = core_bench_runner.tail(text)
        self.assertLess(len(shortened), 5000)
        self.assertTrue(shortened.endswith("x"))


class SessionTest(unittest.TestCase):
    def _session(self, deadline=60):
        import time

        target = TARGETS["targets"][0]
        return core_bench_runner.Session(target, TARGETS, tempfile.gettempdir(), time.time() + deadline)

    def test_missing_command_is_recorded_as_exec_error(self):
        session = self._session()
        with self.assertRaises(core_bench_runner.BenchError) as caught:
            session.run(["definitely-not-a-command-core-bench"], label="missing")
        self.assertEqual(caught.exception.stage, "exec")

    def test_nonzero_exit_keeps_the_log_tail(self):
        session = self._session()
        with self.assertRaises(core_bench_runner.BenchError) as caught:
            session.run([sys.executable, "-c", "import sys; print('broken'); sys.exit(3)"], label="fail")
        self.assertEqual(caught.exception.stage, "command")
        self.assertIn("broken", caught.exception.log)

    def test_timeout_is_recorded(self):
        session = self._session(deadline=3)
        with self.assertRaises(core_bench_runner.BenchError) as caught:
            session.run([sys.executable, "-c", "import time; time.sleep(10)"], timeout=2, label="slow")
        self.assertEqual(caught.exception.stage, "timeout")


class MozcTest(unittest.TestCase):
    def test_existing_helper_is_reused_from_the_environment(self):
        with tempfile.TemporaryDirectory() as directory:
            helper = Path(directory) / "meltype_mozc_helper"
            helper.write_text("#!/bin/sh\n", encoding="utf-8")
            session = core_bench_runner.Session(TARGETS["targets"][0], TARGETS, directory, 0)
            with mock.patch.dict(os.environ, {"CORE_BENCH_MOZC_HELPER": str(helper)}):
                self.assertEqual(core_bench_runner.ensure_mozc(session, directory), helper)

    def test_asset_metadata_points_at_the_upstream_release(self):
        mozc = TARGETS["mozc"]
        self.assertEqual(mozc["repo"], "yksr-melt/Meltype")
        self.assertEqual(mozc["release"], "v1.7.1")
        self.assertTrue(mozc["asset"].startswith("Meltype-1.7.1-linux"))
        self.assertTrue(mozc["member"].endswith("meltype_mozc_helper"))


class EntrypointTest(unittest.TestCase):
    def _run(self, args):
        return subprocess.run(
            [sys.executable, str(BENCH_DIR / "core_bench_runner.py")] + args,
            capture_output=True,
            text=True,
            timeout=120,
        )

    def test_unknown_target_writes_an_error_report_and_exits_zero(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / "result.json"
            result = self._run(["--target", "does-not-exist", "--out", str(out), "--work", directory])
            self.assertEqual(result.returncode, 0, result.stderr)
            payload = json.loads(out.read_text(encoding="utf-8"))
            self.assertEqual(payload["status"], "error")
            self.assertEqual(payload["failure"]["stage"], "input")

    def test_strict_turns_a_failure_into_exit_one(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / "result.json"
            result = self._run(["--target", "does-not-exist", "--out", str(out), "--work", directory, "--strict"])
            self.assertEqual(result.returncode, 1)

    def test_missing_corpus_is_reported_without_network_access(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / "result.json"
            result = self._run(["--target", "kswime", "--out", str(out), "--work", directory, "--corpus", str(Path(directory) / "none")])
            self.assertEqual(result.returncode, 0, result.stderr)
            payload = json.loads(out.read_text(encoding="utf-8"))
            self.assertEqual(payload["failure"]["stage"], "corpus")


if __name__ == "__main__":
    unittest.main()
