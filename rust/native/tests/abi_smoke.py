# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Exercise real C callbacks/ownership and multi-edit results in a fresh process."""
import ctypes as c
import json
import os
import sys
import tempfile


def main(library):
    api = c.CDLL(library)
    allocator = c.CDLL("ucrtbase") if os.name == "nt" else c.CDLL(None)
    allocator.malloc.argtypes = [c.c_size_t]
    allocator.malloc.restype = c.c_void_p
    clauses_type = c.CFUNCTYPE(c.c_void_p, c.c_char_p, c.c_char_p)
    candidates_type = c.CFUNCTYPE(c.c_void_p, c.c_char_p)
    word_type = c.CFUNCTYPE(c.c_int, c.c_char_p)
    preview_probe = False
    preview_calls = []
    alternative_calls = []

    def allocate(text):
        value = text.encode("utf-8") + b"\0"
        ptr = allocator.malloc(len(value))
        if not ptr:
            return None
        c.memmove(ptr, value, len(value))
        return ptr

    @clauses_type
    def clauses(reading, _context):
        if preview_probe:
            preview_calls.append(reading)
            return allocate(reading.decode() + "\t語" + reading.decode())
        return allocate("はし\t橋") if reading == "はし".encode() else None

    @candidates_type
    def candidates(reading):
        if preview_probe:
            alternative_calls.append(reading)
            return allocate("別" + reading.decode())
        return allocate("橋\n箸") if reading == "はし".encode() else None

    @word_type
    def is_word(_word):
        return 0

    api.meltype_init.argtypes = [clauses_type, candidates_type, word_type]
    api.meltype_init.restype = c.c_int
    api.meltype_create.restype = c.c_void_p
    api.meltype_destroy.argtypes = [c.c_void_p]
    api.meltype_free.argtypes = [c.c_void_p]
    api.meltype_handle_key.argtypes = [c.c_void_p, c.c_int, c.c_int, c.c_int, c.c_char_p, c.c_char_p]
    api.meltype_handle_key.restype = c.c_void_p
    api.meltype_reconvert_selection.argtypes = [c.c_void_p,c.c_char_p,c.c_char_p]
    api.meltype_reconvert_selection.restype = c.c_void_p
    api.meltype_commit.argtypes = [c.c_void_p]
    api.meltype_commit.restype = c.c_void_p
    api.meltype_reset.argtypes = [c.c_void_p]
    api.meltype_reset.restype = c.c_void_p
    api.meltype_commit_preedit.argtypes = [c.c_void_p]
    api.meltype_commit_preedit.restype = c.c_void_p
    api.meltype_control.argtypes = [c.c_void_p, c.c_char_p]
    api.meltype_control.restype = c.c_void_p
    api.meltype_tip_create.restype = c.c_void_p
    api.meltype_tip_request.argtypes = [c.c_void_p, c.c_char_p]
    api.meltype_tip_request.restype = c.c_void_p
    api.meltype_tip_destroy.argtypes = [c.c_void_p]
    api.meltype_set_can_delete.argtypes = [c.c_void_p, c.c_int]
    api.meltype_set_private.argtypes = [c.c_void_p, c.c_int]
    api.meltype_set_direct.argtypes = [c.c_void_p, c.c_int]
    api.meltype_set_direct_detection.argtypes = [c.c_void_p, c.c_int]
    api.meltype_undo_commit.argtypes = [c.c_void_p, c.c_char_p]
    api.meltype_undo_commit.restype = c.c_void_p

    def result(ptr):
        assert ptr, "null ABI response"
        try:
            return json.loads(c.string_at(ptr).decode("utf-8"))
        finally:
            api.meltype_free(ptr)

    with tempfile.TemporaryDirectory(prefix="meltype-abi-") as directory:
        os.environ["MELTYPE_RUST_DATA_DIR"] = directory
        assert api.meltype_init(clauses, candidates, is_word) == 1
        handle = api.meltype_create()
        assert handle
        try:
            def key(vk=0, scalar=0, modifiers=0):
                return result(api.meltype_handle_key(handle, vk, scalar, modifiers, None, None))

            for letter in "hashi":
                value = key(scalar=ord(letter))
            assert value["view"]["text"] == "はし", "live conversion starts at four UTF-16 units"
            assert not value["view"]["converting"]
            assert key(32, 32)["view"]["clauses"] == ["橋"]
            value = key(scalar=ord("k"))
            assert value["commits"][0]["text"] == "橋"
            assert value["view"]["text"] == "k", "new preedit lost after old commit"
            assert key(scalar=ord("a"))["view"]["text"] == "か"
            assert result(api.meltype_commit(handle))["commits"][0]["text"] == "か"
            api.meltype_set_can_delete(handle, 1)
            value = result(api.meltype_undo_commit(handle, "橋か".encode()))
            assert value["commits"] == [{"deleteBefore": 1, "text": "", "expect": "か"}]
            assert value["view"]["text"] == "か"
            result(api.meltype_commit(handle))
            value = result(api.meltype_reconvert_selection(handle,"橋".encode(),None))
            assert value["selectionExpect"] == "橋"
            assert value["view"]["text"] == "橋"
            assert key(27)["commits"][0]["text"] == "橋"
            for letter in "hashi":
                key(scalar=ord(letter))
            value = key(32,32,1)
            assert value["commits"] == []
            assert value["view"]["converting"] and value["view"]["text"] == "橋"
            assert result(api.meltype_commit(handle))["commits"][0]["text"] == "橋"
            assert key(32,32,1)["commits"] == [{"deleteBefore":0,"text":"　"}]
            api.meltype_set_private(handle, 1)
            assert not key(scalar=ord("a"))["consumed"]
        finally:
            api.meltype_destroy(handle)
        connection = api.meltype_tip_create()
        assert connection
        try:
            def tip(letter, before, sid="completion", process="notepad.exe", code=False, vk=0):
                request = {"op":"key", "sid":sid, "process":process, "code":code,
                           "vk":vk, "ch":ord(letter), "before":before, "canDelete":True}
                return result(api.meltype_tip_request(connection, json.dumps(request).encode()))

            # 1.4の文章入力では先頭の/は・になる。明示候補/とコードの直接入力も確認する。
            for letter in "/ka":
                value = tip(letter, "", sid="prose")
                assert value["active"] and value["consumed"] and value["commits"] == []
            assert value["view"]["text"] == "・か"
            value = result(api.meltype_tip_request(connection, json.dumps(
                {"op":"commit", "sid":"prose", "process":"notepad.exe"}).encode()))
            assert value["commits"] == [{"deleteBefore":0,"text":"・か"}]

            value = tip("/", "")
            assert value["active"] and value["consumed"] and value["view"]["text"] == "・"
            value = tip(" ", "", vk=32)
            assert value["consumed"] and value["commits"] == []
            assert value["view"]["candidates"] == ["/", "・", "･", "／"]
            assert value["view"]["text"] == "/"
            before = ""
            for index, letter in enumerate("review"):
                value = tip(letter, before)
                assert value["active"] and not value["consumed"]
                expected = [{"deleteBefore":0,"text":"/"}] if index == 0 else []
                assert value["commits"] == expected and value["view"] is None
                if index == 0:
                    before = "/"
                before += letter
            for letter in "kyou":
                value = tip(letter, "/review ")
            assert value["consumed"] and value["view"]["text"] == "きょう"
            for process in ("Code.exe", "WindowsTerminal.exe"):
                before = ""
                for letter in "/review":
                    value = tip(letter, before, sid=process, process=process, code=True)
                    assert value["active"] and not value["consumed"]
                    assert value["commits"] == [] and value["view"] is None
                    before += letter
        finally:
            api.meltype_tip_destroy(connection)
        handle = api.meltype_create()
        try:
            api.meltype_set_direct(handle, 1)
            api.meltype_set_direct_detection(handle, 1)
            value = result(api.meltype_handle_key(handle, 78,110,0,b"kon",b""))
            assert not value["consumed"], "auto detection requires deletion capability"
            api.meltype_set_can_delete(handle, 1)
            value = result(api.meltype_handle_key(handle, 78,110,0,b"kon",b""))
            assert not value["consumed"] and value["commits"] == [], "existing text is not replay history (original timed fixture)"
            value = result(api.meltype_reset(handle))
            for letter, before in zip("konn", (b"", b"k", b"ko", b"kon")):
                value = result(api.meltype_handle_key(handle, ord(letter.upper()),ord(letter),0,before,b""))
            assert value["commits"] == [{"deleteBefore":3,"text":"","expect":"kon"}]
            assert value["view"]["text"] == "こん"
        finally:
            api.meltype_destroy(handle)
        handle = api.meltype_create()
        try:
            api.meltype_set_can_delete(handle, 1)
            for letter in "site PR":
                key(32 if letter == " " else 0, ord(letter))
            value = result(api.meltype_commit(handle))
            assert value["commits"] == [{"deleteBefore":2,"text":"して PR","expect":"して"}]
            def setting(name, value):
                response = result(api.meltype_control(handle, json.dumps({"op":"settings_set", "name":name, "value":value}).encode()))
                assert response.get("saved"), response

            setting("DetectionLevel", "Manual")
            setting("CorrectTypos", False)
            setting("TabConversion", False)
            for letter in "google":
                value = key(scalar=ord(letter))
            assert value["view"]["text"] == "ごおgぇ"
            assert "Tab → google" in value["view"]["hint"]
            # Upstream reserves suggestion for the formatted misspelling
            # message; the English proposal is already in hint.
            assert value["view"]["suggestion"] is None
            value = key(9)
            assert value["consumed"] and value["view"]["text"] == "google"
            assert result(api.meltype_commit(handle))["commits"][0]["text"] == "google"
            setting("DetectionLevel", "Conservative")
            for letter in "amazon":
                value = key(scalar=ord(letter))
            assert value["view"]["text"] == "あまぞn"
            result(api.meltype_commit(handle))
            for letter in "google":
                value = key(scalar=ord(letter))
            assert value["view"]["text"] == "google"
        finally:
            api.meltype_destroy(handle)
        handle = api.meltype_create()
        try:
            setting("DetectionLevel", "Balanced")
            setting("SigilWordsDirect", True)
            for letter in "@src/app.ts":
                value = key(scalar=ord(letter))
                assert not value["consumed"] and value["view"] is None
                assert value["commits"] == [], "sigil names must reach the app once, without synthetic commits"
            for letter in "kyou":
                value = result(api.meltype_handle_key(handle, 0, ord(letter), 0, b"@src/app.ts ", b""))
            assert value["view"]["text"] == "きょう", "host-only whitespace must end sigil passthrough"
            result(api.meltype_commit(handle))
        finally:
            api.meltype_destroy(handle)
        handle = api.meltype_create()
        try:
            for letter in "nihon":
                value = key(scalar=ord(letter))
            displayed = value["view"]["text"]
            value = result(api.meltype_commit_preedit(handle))
            assert value["commits"] == [{"deleteBefore": 0, "text": displayed}]
            assert value["view"] is None
            assert result(api.meltype_commit_preedit(handle))["commits"] == []
        finally:
            api.meltype_destroy(handle)
        handle = api.meltype_create()
        try:
            setting("DetectionLevel", "Manual")
            setting("LiveConversion", True)
            setting("PredictiveCandidates", False)
            preview_probe = True
            for letter in "nihongonyuuryoku":
                value = key(scalar=ord(letter))
                assert value["consumed"] and not value["commits"]
            assert preview_calls, "live preview must still convert primary clause text"
            assert not alternative_calls, "typing must not wait for candidate enumeration"
            assert not value["view"]["converting"]
            assert value["view"]["text"].startswith("語")
            value = key(32, 32)
            assert alternative_calls, "explicit conversion must retain alternatives"
            assert value["view"]["converting"]
            assert any(text.startswith("別") for text in value["view"]["candidates"])
        finally:
            preview_probe = False
            api.meltype_destroy(handle)
    print("Native ABI: callbacks, live preview without candidate waits, multi-edit, guarded undo/correction, selection reconversion, detection levels, manual Tab, sigil passthrough and privacy passed")


if __name__ == "__main__":
    filename = "meltype_native.dll" if os.name == "nt" else (
        "libmeltype_native.dylib" if sys.platform == "darwin" else "libmeltype_native.so")
    main(sys.argv[1] if len(sys.argv) > 1 else os.path.abspath(os.path.join("target", "release", filename)))
