# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Exercise the shipped Linux Mozc helper through the real native key ABI."""
import ctypes as c
import json
import os
from pathlib import Path
import sys
import tempfile

api = c.CDLL(str(Path(sys.argv[1]).resolve()))
pointer = c.c_void_p
api.meltype_init_mozc.argtypes = [c.c_char_p, c.c_char_p]
api.meltype_init_mozc.restype = c.c_int
api.meltype_create_at.argtypes = [c.c_char_p]
api.meltype_create_at.restype = pointer
api.meltype_handle_key.argtypes = [pointer, c.c_int, c.c_int, c.c_int, c.c_char_p, c.c_char_p]
api.meltype_handle_key.restype = pointer
api.meltype_select_candidate.argtypes = [pointer, c.c_int]
api.meltype_select_candidate.restype = pointer
api.meltype_destroy.argtypes = [pointer]
api.meltype_free.argtypes = [pointer]

with tempfile.TemporaryDirectory(prefix="meltype-linux-mozc-") as directory:
    root = Path(directory)
    helper = Path(os.environ["MELTYPE_MOZC_HELPER"]).resolve()
    assert api.meltype_init_mozc(os.fsencode(helper), os.fsencode(root / "mozc")) == 1, helper
    (root / "config.json").write_text(json.dumps({
        "LiveConversion": False, "PredictiveCandidates": False, "DetectionLevel": "Manual"
    }), encoding="utf-8")
    def read(result):
        assert result
        try:
            return json.loads(c.string_at(result))
        finally:
            api.meltype_free(result)

    def key(handle, char):
        vk = 13 if char == "\r" else 32 if char == " " else ord(char.upper())
        reply = read(api.meltype_handle_key(handle, vk, ord(char), 0, None, None))
        assert reply["consumed"], (char, reply)
        return reply

    for raw, expected in [("kanjihenkann", "漢字変換"), ("toukyou", "東京")]:
        handle = api.meltype_create_at(os.fsencode(root))
        assert handle
        try:
            for char in raw + " ":
                reply = key(handle, char)
                assert not reply["commits"], reply
            # Mozcの標準順位では「感じ」が先頭の場合もある。
            # 候補を選択して目的の漢字で確定できることを検証する。
            if reply["view"]["text"] != expected:
                candidates = reply["view"]["candidates"]
                choices = [i for i, value in enumerate(candidates) if value in ("漢字", "漢字変換")]
                assert raw == "kanjihenkann" and choices, (raw, candidates, reply)
                reply = read(api.meltype_select_candidate(handle, choices[0]))
            assert reply["view"]["text"] == expected, (raw, expected, reply)
            reply = key(handle, "\r")
            committed = "".join(edit["text"] for edit in reply["commits"])
            assert committed == expected and reply["view"] is None, (raw, expected, committed, reply)
        finally:
            api.meltype_destroy(handle)
print("PASS: packaged Mozc selects/commits 漢字変換 and converts/commits 東京 through native ABI")
