# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""別DLした実モデルでLinux native ABIを検証する。server/modelは必須。"""
import ctypes as c
import json
from pathlib import Path
import sys
import tempfile
import time

api = c.CDLL(str(Path(sys.argv[1]).resolve()))
api.meltype_create_at.argtypes = [c.c_char_p]
api.meltype_create_at.restype = c.c_void_p
api.meltype_handle_key.argtypes = [c.c_void_p, c.c_int, c.c_int, c.c_int, c.c_char_p, c.c_char_p]
api.meltype_handle_key.restype = c.c_void_p
api.meltype_free.argtypes = [c.c_void_p]
api.meltype_destroy.argtypes = [c.c_void_p]
with tempfile.TemporaryDirectory(prefix='meltype-zenzai-smoke-') as directory:
    config = Path(directory) / 'config.json'
    settings = {'Enabled': True, 'UseZenzai': True, 'ZenzaiServerPath': str(Path(sys.argv[2]).resolve()),
                'ZenzaiModelPath': str(Path(sys.argv[3]).resolve()), 'LiveConversion': False,
                'PredictiveCandidates': False, 'DetectionLevel': 'Manual', 'SettingsVersion': 7}
    config.write_text(json.dumps(settings))
    handle = api.meltype_create_at(directory.encode())
    assert handle
    def key(char):
        vk = {' ': 32, '\r': 13, '\x1b': 27}.get(char, ord(char.upper()))
        result = api.meltype_handle_key(handle, vk, ord(char), 0, None, None)
        assert result
        try:
            return json.loads(c.string_at(result))
        finally:
            api.meltype_free(result)
    try:
        key('a')
        key('\x1b')
        deadline = time.monotonic() + 40
        while True:
            time.sleep(2)
            for char in 'kyouhaiitenkidesu ': reply = key(char)
            text = (reply.get('view') or {}).get('text', '')
            if text == '今日はいい天気です':
                print('PASS: Linux Zenzai real model via native ABI: ' + text)
                break
            key('\r')
            assert time.monotonic() < deadline, reply
        key('\r')
    finally:
        settings['UseZenzai'] = False
        config.write_text(json.dumps(settings))
        key('a')
        key('\x1b')
        key('a')
        api.meltype_destroy(handle)
