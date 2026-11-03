#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""公開リリースのタグをGitHub Issueフォームの選択肢へ反映する。"""
import argparse
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
FORMS = ("1-bug.yml", "2-misdetection.yml")


def update(text, tags):
    block = (
        "  - type: dropdown\n    id: version\n    attributes:\n"
        "      label: Meltype-FLASH の版\n"
        '      description: "使用中の版をリリースタグから選択してください。開発版はそのほかを選び、詳細に記入してください。"\n'
        "      options:\n"
        + "".join("        - " + json.dumps(tag, ensure_ascii=False) + "\n" for tag in tags)
        + '        - "そのほか / 開発版"\n        - "分からない"\n'
        "    validations:\n      required: true\n\n"
    )
    result, count = re.subn(
        r"  - type: (?:input|dropdown)\n    id: version\n.*?(?=  - type:|\Z)",
        lambda _: block, text, flags=re.S,
    )
    if count != 1:
        raise ValueError("版番号の項目が一つではありません")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--include", help="公開準備中のタグも先頭へ追加する")
    args = parser.parse_args()
    pages = json.loads(subprocess.check_output([
        "gh", "api", "repos/appipinopi/Meltype-Flash/releases?per_page=100",
        "--paginate", "--slurp",
    ], encoding="utf-8"))
    releases = sorted(
        (r for page in pages for r in page if not r["draft"]),
        key=lambda r: r["published_at"], reverse=True,
    )
    tags = list(dict.fromkeys(([args.include] if args.include else []) + [r["tag_name"] for r in releases]))
    if not tags:
        raise ValueError("公開リリースがありません")
    for name in FORMS:
        path = ROOT / ".github/ISSUE_TEMPLATE" / name
        path.write_text(update(path.read_text(encoding="utf-8"), tags), encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
