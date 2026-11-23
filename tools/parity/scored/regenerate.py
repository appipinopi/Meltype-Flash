#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Regenerate scored key expectations from verified, unmodified C# evidence."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[3]
BASELINE = "46db6ca2f941eb9580f4d5ed103ee0bee71111c7"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def projection(response):
    view = response["view"]
    return dict(consumed=response["consumed"], commits=response["commits"],
                text=view["text"] if view else None,
                converting=bool(view and view["converting"]))


def formatted(document):
    # Keep one key/response per line, matching the existing reviewed fixtures.
    metadata = {key: value for key, value in document.items() if key != "cases"}
    lines = json.dumps(metadata, ensure_ascii=False, indent=2).splitlines()[:-1]
    lines[-1] += ","
    lines.append('  "cases": [')
    for index, case in enumerate(document["cases"]):
        lines.extend(['    {', '      "name": ' + json.dumps(case["name"]) + ',',
                      '      "scored": true,', '      "keys": ['])
        for row, key in enumerate(case["keys"]):
            lines.append('        ' + json.dumps(key) + (',' if row + 1 < len(case["keys"]) else ''))
        lines.extend(['      ],', '      "expected": ['])
        for row, response in enumerate(case["expected"]):
            response_lines = json.dumps(response, ensure_ascii=False,
                                        indent=2 if response["commits"] else None).splitlines()
            lines.extend('        ' + line for line in response_lines)
            if row + 1 < len(case["expected"]):
                lines[-1] += ','
        lines.extend(['      ]', '    }' + (',' if index + 1 < len(document["cases"]) else '')])
    text = '\n'.join(lines + ['  ]', '}']) + '\n'
    assert json.loads(text) == document
    return text


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("evidence", type=Path, help="Output of check-core-parity.py --check --scored")
    parser.add_argument("--csharp", type=Path, required=True)
    parser.add_argument("--rust", type=Path, required=True)
    parser.add_argument("--reference-source", type=Path, required=True,
                        help="Unmodified source tree used to build the C# executable")
    parser.add_argument("--baseline", default=BASELINE)
    parser.add_argument("--output", type=Path,
                        default=ROOT / "rust/core/tests/data/upstream-scored-keys.json")
    args = parser.parse_args()
    commit = subprocess.check_output(
        ["git", "rev-parse", "--verify", args.baseline + "^{commit}"], cwd=ROOT,
        text=True, encoding="utf-8").strip()
    report = read(args.evidence / "report.json")
    cases = read(args.evidence / "cases.json")
    if not cases or report.get("baseline") != commit or not (
        report["total"] == report["output_pass"] == report["trace_pass"] == len(cases)
    ):
        raise SystemExit("Every C#/Rust key projection must match the pinned baseline before freezing")
    for label, binary in (("csharp", args.csharp), ("rust", args.rust)):
        if sha(binary) != report["executable_sha256"][label]:
            raise SystemExit(f"The {label} executable changed after comparison")
    if not all(case.get("scored") is True for case in cases):
        raise SystemExit("All captured cases must explicitly enable scored segmentation")

    # Check the reference inputs against Git, independently of the Rust output.
    archive = subprocess.check_output(
        ["git", "archive", "--format=zip", commit, "src/Meltype.Core", "dictionaries"], cwd=ROOT)
    source_hashes = {}
    expected_code = set()
    with zipfile.ZipFile(io.BytesIO(archive)) as files:
        for item in files.infolist():
            if item.is_dir():
                continue
            path = args.reference_source / item.filename
            expected = files.read(item)
            actual = path.read_bytes()
            if actual.replace(b"\r\n", b"\n") != expected.replace(b"\r\n", b"\n"):
                raise SystemExit(f"The C# reference source differs from {commit}: {item.filename}")
            source_hashes[item.filename] = hashlib.sha256(actual).hexdigest()
            if item.filename.endswith(".cs"):
                expected_code.add(item.filename)
    actual_code = {
        path.relative_to(args.reference_source).as_posix()
        for path in (args.reference_source / "src/Meltype.Core").rglob("*.cs")
        if not {"obj", "bin"}.intersection(path.relative_to(args.reference_source).parts)
    }
    if actual_code != expected_code:
        raise SystemExit("The C# reference contains added or missing source files")

    capture = args.evidence / "csharp.jsonl"
    rows = [json.loads(line) for line in capture.read_text(encoding="utf-8-sig").splitlines()]
    if len(rows) != len(cases):
        raise SystemExit("Missing or repeated C# response rows")
    output_cases = []
    for case, row in zip(cases, rows, strict=True):
        if case["name"] != row["case"] or len(case["keys"]) != len(row["responses"]):
            raise SystemExit("C# responses do not cover the input keys in order")
        # Expected values come solely from the C# capture, never from Rust.
        output_cases.append(dict(name=case["name"], scored=True, keys=case["keys"],
                                 expected=[projection(json.loads(reply)) for reply in row["responses"]]))
    harness = args.reference_source / "tools/performance/windows/Program.cs"
    document = dict(
        baseline=commit, builtin_spelling=True, scored=True,
        reference_executable_sha256=sha(args.csharp), reference_host_sha256=sha(harness),
        reference_source_sha256=source_hashes,
        reference_capture_sha256=sha(capture), comparison_report_sha256=sha(args.evidence / "report.json"),
        scope="Buffered Balanced scored segmentation; can-delete true; learning, live conversion and prediction disabled. Original 12 scored tests plus 75 existing word/edit/capitalization traces. Consumed, commits, view text and converting only.",
        cases=output_cases)
    args.output.write_text(formatted(document), encoding="utf-8")
    print(f"Regenerated {len(output_cases)} scored traces from C# {commit}")


if __name__ == "__main__":
    main()
