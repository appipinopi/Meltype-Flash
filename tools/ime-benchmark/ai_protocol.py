#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""AIモデル (GGUF) への入力整形と応答の後処理。

`rust/core/src/ai_catalog.rs` の `prompt` / `rust/core/src/romaji.rs` の
`katakana` / `rust/native/src/windows_converter/zenzai.rs` の `clean` を
そのまま写したもの。AIモデルの測定は本番の入力経路と同じ文字列で行う。
"""
import re
import unicodedata

START = "\uee00"  # 読みの開始
END = "\uee01"  # 読みの終了
CONTEXT = "\uee02"  # 文脈の開始
CONTROL = re.compile("[\uee00-\uee08\n]")


def katakana(text):
    """ひらがな U+3041..U+3096 を +0x60 してカタカナにする (romaji.rs と同じ)。"""
    return "".join(
        chr(ord(c) + 0x60) if "\u3041" <= c <= "\u3096" else c for c in text
    )


def prompt(protocol, reading, context):
    """プロトコル別の入力文字列。jinen だけ NFKC 正規化する。"""
    kana = katakana(reading)
    if protocol == "zenz-v1":
        return f"{START}{kana}{END}"
    if protocol == "zenz-v2":
        return f"{START}{kana}{CONTEXT}{context}{END}"
    if protocol == "jinen":
        return unicodedata.normalize("NFKC", f"{CONTEXT}{context}{START}{kana}{END}")
    return f"{CONTEXT}{context}{START}{kana}{END}"


def clean(content):
    """llama-server の content から変換結果を取り出す (zenzai.rs の clean と同じ)。

    制御文字・改行で切れた先頭を使い、`</s>` `<s>` を除き、空・192 UTF-16単位超・
    制御文字入りは None (本番と同じく採用しない)。
    """
    text = CONTROL.split(content or "", maxsplit=1)[0]
    text = text.replace("</s>", "").replace("<s>", "").strip()
    if not text:
        return None
    if len(text.encode("utf-16-le")) // 2 > 192:
        return None
    if any(unicodedata.category(c) == "Cc" for c in text):
        return None
    return text
