# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Original C# clause resize traces and converter callbacks through the native ABI."""
import argparse
import ctypes as c
import json
import os
from pathlib import Path
import sys
import tempfile

allocator = c.CDLL("ucrtbase") if os.name == "nt" else c.CDLL(None)
allocator.malloc.argtypes = [c.c_size_t]
allocator.malloc.restype = c.c_void_p
clause_type = c.CFUNCTYPE(c.c_void_p, c.c_char_p, c.c_char_p)
candidate_type = c.CFUNCTYPE(c.c_void_p, c.c_char_p)
queries = []
failure = False

def allocate(text):
    data = text.encode() + b"\0"
    pointer = allocator.malloc(len(data))
    assert pointer
    c.memmove(pointer, data, len(data))
    return pointer

@clause_type
def clauses(raw, _context):
    reading = raw.decode()
    queries.append(reading)
    if reading == "あいうえおかきくけこ":
        return allocate("あいうえお\t【あいうえお】\nかきくけこ\t【かきくけこ】")
    if reading == "あいうえおかきくけこさしす":
        return allocate("あいうえお\t【あいうえお】\nかきくけこ\t【かきくけこ】\nさしす\t【さしす】")
    return allocate(reading + "\t【" + reading + "】")

@candidate_type
def convert(raw):
    queries.append(raw.decode())
    if failure and raw.decode() == "あいうえおか":
        return None
    return allocate("【" + raw.decode() + "】")

@candidate_type
def candidates(raw):
    return allocate(raw.decode() + "★")


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("library", nargs="?", type=Path)
parser.add_argument("--golden", type=Path, default=Path(__file__).resolve().parents[2] / "core/tests/data/upstream-clause-resize.json")
args = parser.parse_args()
filename = "meltype_native.dll" if os.name == "nt" else "libmeltype_native.dylib" if sys.platform == "darwin" else "libmeltype_native.so"
api = c.CDLL(str(Path(args.library if args.library else Path("target/release") / filename).resolve()))
P = c.c_void_p
api.meltype_init.argtypes = [P, P, P]
assert api.meltype_init(c.cast(clauses, c.c_void_p), c.cast(candidates, c.c_void_p), None) == 1
api.meltype_set_converter.argtypes = [candidate_type]
assert api.meltype_set_converter(convert) == 1
api.meltype_create_at.argtypes = [c.c_char_p]
api.meltype_create_at.restype = P
api.meltype_handle_key.argtypes = [P, c.c_int, c.c_int, c.c_int, c.c_char_p, c.c_char_p]
api.meltype_handle_key.restype = P
api.meltype_destroy.argtypes = [P]
api.meltype_free.argtypes = [P]
api.meltype_set_can_delete.argtypes = [P, c.c_int]

golden = json.loads(args.golden.read_text(encoding="utf-8"))
with tempfile.TemporaryDirectory(prefix="meltype-clause-resize-") as directory:
    root = Path(directory)
    (root / "config.json").write_text(json.dumps(dict(ScoredSegmentation=False, LearningEnabled=False,
                                                     LiveConversion=False, PredictiveCandidates=False)), encoding="utf-8")
    for case in golden["cases"]:
        queries.clear()
        failure = case.get("failure", False)
        (root / "config.json").write_text(json.dumps(dict(ScoredSegmentation=False, LearningEnabled=False,
            LiveConversion=False, PredictiveCandidates=False, SeparateKeyRoles=case.get("separate", False))), encoding="utf-8")
        handle = api.meltype_create_at(str(root).encode())
        assert handle
        api.meltype_set_can_delete(handle, 1)
        assert len(case["keys"]) == len(case["expected"])
        try:
            for index, (key, expected) in enumerate(zip(case["keys"], case["expected"], strict=True)):
                # #1005: a trailing capital must not undo the established
                # Japanese reading. The frozen C# trace briefly displayed
                # たnniG before the following i repaired it. Assert the new
                # exact text and retain every other original field/check.
                if case["name"] == "mixed-shrink-before-english" and index == 5:
                    assert key == [71, 71, 1]
                    assert expected["view"]["text"] == "たnniG"
                    expected = dict(expected, view=dict(expected["view"], text="たんいG"))
                query_count = len(queries)
                pointer = api.meltype_handle_key(handle, *key, None, None)
                assert pointer, (case["name"], index, "null native response")
                try:
                    value = json.loads(c.string_at(pointer))
                finally:
                    api.meltype_free(pointer)
                if case["name"] in {"mixed-grow-last-english-noop", "mixed-grow-unreadable-next-noop"} and key == [39, 0, 1]:
                    assert len(queries) == query_count, (case["name"], "no-op resize queried converter")
                def projection(result):
                    view = result["view"]
                    fields = ("text", "converting", "clauses") + (("selectedIndex", "selectedClause") if view and view["converting"] else ())
                    return dict(consumed=result["consumed"], commits=result["commits"],
                                view={k:view[k] for k in fields} if view else None)
                assert projection(value) == projection(expected), (case["name"], index, projection(expected), projection(value))
                if expected["view"] and expected["view"]["converting"]:
                    # Check the converter candidate and the newly requested
                    # alternate without claiming parity of every dictionary.
                    specials = [x for x in expected["view"]["candidates"] if "★" in x or x.startswith("【")]
                    actual = [x for x in value["view"]["candidates"] if "★" in x or x.startswith("【")]
                    assert actual == specials, (case["name"], index, specials, actual)
            if case["name"] == "grow-select-next":
                assert "あいうえおか" in queries and "きくけこ" in queries, queries
        finally:
            api.meltype_destroy(handle)
print(f"{args.golden.stem} native ABI: {len(golden['cases'])} original key traces passed")
