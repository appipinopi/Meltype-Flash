# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""凍結したC#比較の基準ソースと辞書を、上書きせず隔離展開する。"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[3]
BASELINE = "45e14f86558aa6ea34f2c2ecd038ba95ca8c3c46"
FIXTURES = (
    "upstream-engine-learning", "upstream-clause-raw", "upstream-clause-language",
    "upstream-clause-history", "upstream-single-choice", "upstream-engine-keys",
    "upstream-case-keys", "upstream-number-keys", "upstream-public-number-keys",
    "upstream-conversion-keys", "upstream-prediction-keys", "upstream-direct-typos",
    "upstream-misspelling-keys", "upstream-misspellings",
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    fixtures = {}
    for name in FIXTURES:
        path = ROOT / "rust/core/tests/data" / (name + ".json")
        data = path.read_bytes()
        if json.loads(data.decode("utf-8-sig")).get("baseline") != BASELINE:
            raise ValueError(f"凍結fixtureの比較元が異なります: {name}")
        fixtures[name] = hashlib.sha256(data).hexdigest()
    commit = subprocess.check_output(
        ["git", "rev-parse", "--verify", BASELINE + "^{commit}"], cwd=ROOT,
        text=True, encoding="utf-8").strip()
    if commit != BASELINE:
        raise ValueError("比較元のコミットSHAが一致しません")
    archive = subprocess.check_output(
        ["git", "archive", "--format=zip", BASELINE, "src/Meltype.Core", "dictionaries"], cwd=ROOT)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    with zipfile.ZipFile(io.BytesIO(archive)) as package:
        for member in package.infolist():
            target = (output / member.filename).resolve()
            if target == output or not target.is_relative_to(output):
                raise ValueError("比較元アーカイブのパスが隔離先の外を指しています")
        package.extractall(output)
    provenance = dict(baseline=BASELINE, archive_sha256=hashlib.sha256(archive).hexdigest(),
                      fixture_sha256=fixtures)
    (output / "reference-provenance.json").write_text(
        json.dumps(provenance, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(output / "src/Meltype.Core/Meltype.Core.csproj")


if __name__ == "__main__":
    main()
