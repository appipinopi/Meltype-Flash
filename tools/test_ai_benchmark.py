#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""tools/ime-benchmark のAIモデル測定部のユニットテスト (ネットワーク・GPU不要)。"""
import http.server
import importlib.util
import json
import sys
import tempfile
import threading
import unittest
from pathlib import Path

HERE = Path(__file__).parent
BENCH_DIR = HERE / "ime-benchmark"
sys.path.insert(0, str(BENCH_DIR))


def _load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


ai_protocol = _load("ai_protocol", BENCH_DIR / "ai_protocol.py")
ai_gguf_runner = _load("ai_gguf_runner", BENCH_DIR / "ai_gguf_runner.py")
ai_bench_plan = _load("ai_bench_plan", BENCH_DIR / "ai_bench_plan.py")
run_benchmark = _load("run_benchmark", BENCH_DIR / "run_benchmark.py")


class ProtocolTest(unittest.TestCase):
    def test_katakana(self):
        self.assertEqual(ai_protocol.katakana("きょうは"), "キョウハ")
        self.assertEqual(ai_protocol.katakana("abc・ー"), "abc・ー")

    def test_prompt_matches_rust(self):
        # rust/core/src/ai_catalog.rs の prompt テストと同じ期待値
        self.assertEqual(ai_protocol.prompt("zenz-v1", "きょう", "昨日"), "\uee00キョウ\uee01")
        self.assertEqual(
            ai_protocol.prompt("zenz-v2", "きょう", "昨日"), "\uee00キョウ\uee02昨日\uee01"
        )
        self.assertEqual(
            ai_protocol.prompt("jinen", "きょう", "ＡＢＣ①"), "\uee02ABC1\uee00キョウ\uee01"
        )
        self.assertEqual(
            ai_protocol.prompt("zenz-v3", "きょう", ""), "\uee02\uee00キョウ\uee01"
        )
        self.assertEqual(
            ai_protocol.prompt("zenz-v3", "きょう", "。"), "\uee02。\uee00キョウ\uee01"
        )

    def test_clean_matches_rust(self):
        self.assertEqual(ai_protocol.clean("今日は晴れ\uee01"), "今日は晴れ")
        self.assertEqual(ai_protocol.clean("</s>明日<s>"), "明日")
        self.assertEqual(ai_protocol.clean("一行目\n二行目"), "一行目")
        self.assertIsNone(ai_protocol.clean(""))
        self.assertIsNone(ai_protocol.clean("\uee01"))
        self.assertIsNone(ai_protocol.clean("a\x00b"))


class ShardTest(unittest.TestCase):
    def test_shards_cover_every_case_once(self):
        cases = [{"id": index, "input": "あ"} for index in range(1, 10001)]
        seen = []
        for shard in range(5):
            seen.extend(case["id"] for case in ai_gguf_runner.shard_slice(cases, shard, 5))
        self.assertEqual(seen, [case["id"] for case in cases])

    def test_small_corpus_shards(self):
        cases = [{"id": index, "input": "あ"} for index in range(7)]
        sizes = [len(ai_gguf_runner.shard_slice(cases, shard, 4)) for shard in range(4)]
        self.assertEqual(sizes, [1, 2, 2, 2])
        self.assertEqual(sum(sizes), 7)


class PlanTest(unittest.TestCase):
    def setUp(self):
        self.catalog = json.loads((HERE.parent / "rust" / "ai" / "model-catalog.json").read_text(encoding="utf-8"))

    def test_all_models_appear_once(self):
        plan = ai_bench_plan.build(self.catalog, {})
        listed = [entry["variants"].split() for entry in plan["shunti"]]
        shunti_ids = [model_id for group in listed for model_id in group]
        gguf_ids = [entry["model"] for entry in plan["gguf"]]
        self.assertEqual(len(shunti_ids) + len(gguf_ids), len(self.catalog["models"]))
        self.assertEqual(len(shunti_ids), len(set(shunti_ids)))
        self.assertEqual(len(gguf_ids), len(set(gguf_ids)))
        self.assertEqual(sum(len(group) for group in listed), 6)
        self.assertEqual(len(plan["shunti"]), 2)

    def test_shards_multiply_jobs(self):
        model_id = next(m["id"] for m in self.catalog["models"] if m["family"] != "Shunti")
        plan = ai_bench_plan.build(self.catalog, {model_id: 5})
        entries = [entry for entry in plan["gguf"] if entry["model"] == model_id]
        self.assertEqual([entry["shard"] for entry in entries], [0, 1, 2, 3, 4])
        self.assertTrue(all(entry["shards"] == 5 for entry in entries))
        self.assertEqual(len({entry["name"] for entry in plan["gguf"]}), len(plan["gguf"]))

    def test_only_filters(self):
        plan = ai_bench_plan.build(self.catalog, {}, "zenz-v1-q8_0")
        self.assertEqual([entry["model"] for entry in plan["gguf"]], ["zenz-v1-q8_0"])
        self.assertEqual(plan["shunti"], [])


class MockServer(http.server.BaseHTTPRequestHandler):
    """llama-server の /health と /completion だけを真似る。"""

    reply = "今日の天気\uee01"
    prompts = []

    def log_message(self, *args):
        pass

    def do_GET(self):
        if self.path == "/health":
            self.send_response(200)
            self.send_header("Content-Length", "2")
            self.end_headers()
            self.wfile.write(b"{}")
        else:
            self.send_error(404)

    def do_POST(self):
        length = int(self.headers["Content-Length"])
        body = json.loads(self.rfile.read(length))
        MockServer.prompts.append(body)
        payload = json.dumps({"content": MockServer.reply}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)


class RunnerTest(unittest.TestCase):
    def setUp(self):
        self.httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), MockServer)
        self.thread = threading.Thread(target=self.httpd.serve_forever, daemon=True)
        self.thread.start()
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        corpus = self.root / "corpus"
        corpus.mkdir()
        rows = [
            {"id": 1, "input": "きょうはいいてんき"},
            {"id": 2, "input": "あしたはあめ"},
            {"id": 3, "input": "ねこがすわった"},
            {"id": 4, "input": "ほんをよんだ"},
        ]
        (corpus / "part1.jsonl").write_text(
            "".join(json.dumps(row, ensure_ascii=False) + "\n" for row in rows), encoding="utf-8"
        )
        self.corpus = corpus
        MockServer.prompts = []

    def tearDown(self):
        self.httpd.shutdown()
        self.httpd.server_close()
        self.directory.cleanup()

    def _run(self, **overrides):
        port = self.httpd.server_address[1]
        original_install = ai_gguf_runner.ensure_install
        original_launch = ai_gguf_runner.launch
        ai_gguf_runner.ensure_install = lambda model, data_root: (Path("/bin/true"), Path("/bin/true"))
        ai_gguf_runner.launch = lambda server, model, tokenizer, log: ai_gguf_runner.Server(None, port)
        try:
            values = dict(
                model="zenz-v1-q8_0",
                shard=0,
                shards=1,
                corpus=str(self.corpus),
                out=str(self.root / "out.jsonl"),
                meta=str(self.root / "out.meta.json"),
                data_root=str(self.root / "data"),
                limit=0,
                max_seconds=0.0,
                request_timeout=5.0,
                startup_timeout=5.0,
            )
            values.update(overrides)
            return ai_gguf_runner.run(ai_gguf_runner.argparse.Namespace(**values))
        finally:
            ai_gguf_runner.ensure_install = original_install
            ai_gguf_runner.launch = original_launch

    def test_converts_every_case_in_shard(self):
        meta = self._run(shard=1, shards=2)
        rows = [
            json.loads(line)
            for line in (self.root / "out.jsonl").read_text(encoding="utf-8").splitlines()
        ]
        self.assertEqual([row["id"] for row in rows], [3, 4])
        self.assertEqual(rows[0]["actual"], "今日の天気")
        self.assertEqual(meta["completed"], 2)
        self.assertEqual(meta["reason"], "completed")
        self.assertEqual(len(MockServer.prompts), 2)
        self.assertEqual(MockServer.prompts[0]["prompt"], "\uee00ネコガスワッタ\uee01")
        self.assertEqual(MockServer.prompts[0]["n_predict"], 96)
        self.assertEqual(MockServer.prompts[0]["temperature"], 0)
        self.assertTrue(MockServer.prompts[0]["parse_special"])
        written = json.loads((self.root / "out.meta.json").read_text(encoding="utf-8"))
        self.assertEqual(written["cases_in_shard"], 2)

    def test_limit_applies_before_sharding(self):
        meta = self._run(shard=0, shards=2, limit=3)
        rows = [
            json.loads(line)
            for line in (self.root / "out.jsonl").read_text(encoding="utf-8").splitlines()
        ]
        self.assertEqual([row["id"] for row in rows], [1])
        self.assertEqual(meta["cases_in_shard"], 1)

    def test_deadline_stops_cleanly(self):
        meta = self._run(max_seconds=1e-9)
        self.assertEqual(meta["reason"], "deadline")
        self.assertEqual(meta["completed"], 0)
        self.assertTrue((self.root / "out.meta.json").exists())

    def test_score_matches_benchmark(self):
        cases = [
            {"id": 1, "category": "基本変換", "expected": "今日は晴れ"},
            {"id": 2, "category": "基本変換", "expected": "明日は雨"},
        ]
        actuals = {1: "今日は晴れ", 2: "明日は雪"}
        result = run_benchmark.score(cases, actuals, "expected")
        self.assertEqual(result["total"]["exact"], 1)
        self.assertEqual(result["total"]["total"], 2)
        self.assertAlmostEqual(result["total"]["exact_accuracy"], 0.5)
        self.assertLess(result["total"]["character_accuracy"], 1.0)


if __name__ == "__main__":
    unittest.main()
