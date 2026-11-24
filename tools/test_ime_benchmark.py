#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""tools/ime-benchmark のユニットテスト (ネットワーク・cargo不要の部分)。"""
import importlib.util
import sys
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


kana2romaji = _load("kana2romaji", BENCH_DIR / "kana2romaji.py")
run_benchmark = _load("run_benchmark", BENCH_DIR / "run_benchmark.py")

FAKE_RELEASES = [
    {"tag": "flash-v0001.0001.0026", "version": (1, 1, 26), "published_at": "2026-11-21T00:00:00Z"},
    {"tag": "flash-v0001.0001.0025", "version": (1, 1, 25), "published_at": "2026-11-20T00:00:00Z"},
    {"tag": "flash-v0001.0001.0024", "version": (1, 1, 24), "published_at": "2026-11-18T00:00:00Z"},
]


class Kana2RomajiTest(unittest.TestCase):
    def test_basic(self):
        self.assertEqual(kana2romaji.convert("きょうは"), ("kyouha", "きょうは"))

    def test_sokuon(self):
        self.assertEqual(kana2romaji.convert("がっこう"), ("gakkou", "がっこう"))
        self.assertEqual(kana2romaji.convert("いっしょ"), ("issho", "いっしょ"))

    def test_n_rules(self):
        self.assertEqual(kana2romaji.convert("しんぶん"), ("shinbun", "しんぶん"))
        self.assertEqual(kana2romaji.convert("かんい"), ("kanni", "かんい"))
        self.assertEqual(kana2romaji.convert("ほん"), ("hon", "ほん"))

    def test_long_vowel_and_katakana(self):
        self.assertEqual(kana2romaji.convert("こーひー"), ("ko-hi-", "こーひー"))
        self.assertEqual(kana2romaji.convert("データ"), ("de-ta", "でーた"))

    def test_mixed_passthrough(self):
        typed, target = kana2romaji.convert("あしたはAPIをみる")
        self.assertEqual(typed, "ashitahaAPIwomiru")
        self.assertEqual(target, "あしたはAPIをみる")


class ResolveSpecTest(unittest.TestCase):
    def test_latestrelease(self):
        r = run_benchmark.resolve_spec("latestrelease", FAKE_RELEASES)
        self.assertEqual(r["tag"], "flash-v0001.0001.0026")

    def test_latestrelease_minus(self):
        self.assertEqual(run_benchmark.resolve_spec("latestrelease-1", FAKE_RELEASES)["tag"], "flash-v0001.0001.0025")
        self.assertEqual(run_benchmark.resolve_spec("latestrelease-2", FAKE_RELEASES)["tag"], "flash-v0001.0001.0024")

    def test_explicit_version_forms(self):
        for spec in ("1.1.25", "v1.1.25", "flash-v1.1.25", "flash-v0001.0001.0025", "0001.0001.0025"):
            self.assertEqual(run_benchmark.resolve_spec(spec, FAKE_RELEASES)["tag"], "flash-v0001.0001.0025", spec)

    def test_url_forms(self):
        self.assertEqual(run_benchmark.resolve_spec("https://github.com/appipinopi/Meltype-Flash/latestrelease", FAKE_RELEASES)["tag"], "flash-v0001.0001.0026")
        self.assertEqual(run_benchmark.resolve_spec("https://github.com/appipinopi/Meltype-Flash/latestrelease-1", FAKE_RELEASES)["tag"], "flash-v0001.0001.0025")
        self.assertEqual(run_benchmark.resolve_spec("https://github.com/appipinopi/Meltype-Flash/releases/tag/flash-v0001.0001.0024", FAKE_RELEASES)["tag"], "flash-v0001.0001.0024")

    def test_head_and_errors(self):
        self.assertIsNone(run_benchmark.resolve_spec("HEAD", FAKE_RELEASES)["tag"])
        with self.assertRaises(ValueError):
            run_benchmark.resolve_spec("latestrelease-9", FAKE_RELEASES)
        with self.assertRaises(ValueError):
            run_benchmark.resolve_spec("9.9.9", FAKE_RELEASES)


class ScoreTest(unittest.TestCase):
    def test_score_and_table(self):
        cases = [
            {"id": 1, "category": "基本変換", "kana_target": "あした", "expected": "明日"},
            {"id": 2, "category": "基本変換", "kana_target": "きのう", "expected": "昨日"},
            {"id": 3, "category": "複合語", "kana_target": "じょうほう", "expected": "情報"},
        ]
        actuals = {1: "あした", 2: "きの", 3: "じょうほう"}
        s = run_benchmark.score(cases, actuals, "kana_target")
        self.assertEqual(s["total"]["exact"], 2)
        self.assertEqual(s["total"]["total"], 3)
        self.assertAlmostEqual(s["per_category"]["基本変換"]["exact_accuracy"], 0.5)
        results = [{"version": {"tag": "flash-v0001.0001.0026"}, "layer_a_kana": s, "layer_b_kanji": None},
                   {"version": {"tag": "flash-v0001.0001.0025"}, "layer_a_kana": s, "layer_b_kanji": None}]
        table = run_benchmark.make_table(results)
        self.assertIn("基本変換", table)
        self.assertIn("flash-v0001.0001.0026", table)
        self.assertIn("未測定", table)


if __name__ == "__main__":
    unittest.main()
