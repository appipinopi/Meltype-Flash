#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Compare every key against the original C# benchmark host, without timing claims.

Use the binaries built by benchmark-windows.py. No OS IME registration is used.
All input is synthetic, session-local, with learning/live/prediction disabled.
Both hosts must use the bundled spelling dictionary, with OS spelling disabled.
"""
import argparse
import datetime
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("benchmark_windows", Path(__file__).with_name("benchmark-windows.py"))
benchmark = importlib.util.module_from_spec(spec)
spec.loader.exec_module(benchmark)


def fixtures():
    def keys(text):
        return [[ord(c.upper()) if c.isascii() and c.isalpha() else 7, ord(c), int(c.isupper())] for c in text]
    words = ["gakt", "gakta", "gakkou", "stackoverflow", "procrasticle", "grokga", "grokniyoruto",
             "sdakega", "Xgawakaru", "kaxnji", "ulo", "kyouhagoogledekensaku", "googlede",
             "pythonno", "githubya", "commitshita", "reflectsareta", "hello", "thankyouforyourhelp",
             "nihongo", "konnichiwa", "kyouhaiitenkidesune", "hotel", "google", "github", "python",
             "repo", "go", "no", "ya", "a", "i", "kore", "make", "sushi", "mala", "tsu",
             "GitHub", "iPhone", "macOS", "https://example.com/", "C:\\test", "@kurakura"]
    result = [dict(name="type:" + w, keys=keys(w) + [[13, 0, 0]]) for w in words]
    for stem in ("gakkou", "kya", "nihongo", "google", "githubya", "pythonno"):
        for suffix in ("ta", "kou", "ya"):
            result.append(dict(name=f"edit:{stem}:back2:{suffix}",
                               keys=keys(stem) + [[8, 0, 0]] * 2 + keys(suffix) + [[13, 0, 0]]))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--csharp", required=True, type=Path)
    parser.add_argument("--rust", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--cases", type=Path)
    parser.add_argument("--baseline-ref", help="Reference used to build the supplied C# executable")
    parser.add_argument("--scored", action="store_true", help="enable scored segmentation in both hosts for every case")
    parser.add_argument("--profile", type=Path, help="isolated data directory with dictionaries/romaji.txt (never a user profile)")
    parser.add_argument("--check", action="store_true", help="exit unsuccessfully if any core trace differs")
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    if (output / "report.json").exists():
        parser.error("choose a new output directory to preserve evidence")
    cases = json.loads(args.cases.read_text(encoding="utf-8")) if args.cases else fixtures()
    if isinstance(cases, dict):
        cases = cases["cases"]
    if args.scored:
        cases = [dict(case, scored=True) for case in cases]
    fixture_path = output / "cases.json"
    fixture_path.write_text(json.dumps(cases, ensure_ascii=False, indent=2), encoding="utf-8")
    rows = {}
    for label, binary in (("csharp", args.csharp), ("rust", args.rust)):
        print("Checking", label, flush=True)
        with (output / f"{label}.jsonl").open("w", encoding="utf-8") as log:
            subprocess.run([str(binary.resolve()), str(fixture_path), "1", "1"] +
                           ([str(args.profile.resolve())] if args.profile else []), cwd=ROOT, stdout=log, check=True)
        rows[label] = {r["case"]: r for line in (output / f"{label}.jsonl").read_text(encoding="utf-8-sig").splitlines()
                       for r in [json.loads(line)]}
        if len(rows[label]) != len(cases):
            raise RuntimeError("missing or duplicate cases: " + label)
    results = []
    for case in cases:
        name = case["name"]
        reference, current = [benchmark.verify(rows[label][name]["responses"], case["keys"]) for label in rows]
        differences = [dict(key_index=i, key=case["keys"][i], expected=a, actual=b)
                       for i, (a, b) in enumerate(zip(reference[1], current[1], strict=True)) if a != b]
        results.append(dict(case=name, output_matches=reference[0] == current[0],
                            expected=reference[0], actual=current[0], differences=differences))
    report = dict(measured_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
                  scope=__doc__, baseline=args.baseline_ref, total=len(cases),
                  output_pass=sum(r["output_matches"] for r in results),
                  trace_pass=sum(not r["differences"] for r in results), cases=results,
                  executable_sha256={label: hashlib.sha256(p.read_bytes()).hexdigest()
                                     for label,p in (("csharp",args.csharp),("rust",args.rust))})
    (output / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2)+"\n", encoding="utf-8")
    print(f"Output {report['output_pass']}/{report['total']}; trace {report['trace_pass']}/{report['total']}")
    for r in results:
        if r["differences"]:
            print(r["case"], "first mismatch:", json.dumps(r["differences"][0], ensure_ascii=False))
    if args.check and report["trace_pass"] != report["total"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
