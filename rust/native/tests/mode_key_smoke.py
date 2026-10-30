# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""実際の共有ライブラリで切替・ターミナル・fcitxイベントを検証する。"""
import ctypes as c
import json
from pathlib import Path
import sys
import tempfile
import unittest

api = c.CDLL(str(Path(sys.argv.pop(1)).resolve()))
P, I, U = c.c_void_p, c.c_int, c.c_uint32

def bind(name, result, args):
    fn = getattr(api, 'meltype_' + name)
    fn.restype, fn.argtypes = result, args
    return fn

create = bind('fcitx_create_at', P, [c.c_char_p])
destroy = bind('fcitx_destroy', None, [P])
free = bind('free', None, [P])
mode = bind('mode_key', P, [P, I, I])
current = bind('input_mode', I, [P])
key = bind('handle_key', P, [P, I, I, I, c.c_char_p, c.c_char_p])
attributes = bind('set_input_attributes', None, [P, U])
caret = bind('caret_moved', None, [P, I, I])
state = bind('fcitx_state', I, [P])
role = bind('role_key', P, [P, I])

class Surrounding(c.Structure):
    _fields_ = [('before', P), ('before_length', U), ('after', P), ('after_length', U)]
Query = c.CFUNCTYPE(I, P, U, c.POINTER(Surrounding))
Event = c.CFUNCTYPE(I, P, U, I, P, U)
dispatch = bind('fcitx_key_event', I, [P, U, U, U, I, P, Query, Event])

def take(pointer):
    if not pointer:
        return None
    try:
        return json.loads(c.string_at(pointer).decode('utf-8'))
    finally:
        free(pointer)

class ModeTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        root = Path(self.directory.name)
        (root / 'config.json').write_text(json.dumps({
            'SettingsVersion': 5, 'ToggleKeyboardShortcuts': 'Shift+Space; Ctrl+Space',
            'LiveConversion': False,
        }), encoding='utf-8')
        self.handle = create(str(root).encode('utf-8'))
        self.assertTrue(self.handle)
        self.addCleanup(lambda: destroy(self.handle))

    def type(self, text):
        result = None
        for char in text:
            result = take(key(self.handle, 0, ord(char), 0, None, None))
        return result

    def test_mode_api_keeps_normal_keys_and_configured_toggle_commits_once(self):
        self.assertIsNone(take(mode(self.handle, 65, 0)))
        self.type('kyou')
        result = take(mode(self.handle, 32, 1))
        self.assertEqual([edit['text'] for edit in result['commits']], ['きょう'])
        self.assertEqual(current(self.handle), 1)
        self.assertEqual(take(mode(self.handle, 32, 2))['commits'], [])
        self.assertEqual(current(self.handle), 0)
        self.assertEqual(self.type('nihongo')['view']['text'], 'にほんご')

    def test_terminal_switch_and_navigation_are_effective_in_native_abi(self):
        attributes(self.handle, 4)
        self.assertEqual(current(self.handle), 1)
        self.assertFalse(self.type('n')['consumed'])
        take(mode(self.handle, 0x19, 0))
        self.assertEqual(self.type('nihongo')['view']['text'], 'にほんご')
        self.assertEqual(take(mode(self.handle, 0x19, 0))['commits'][0]['text'], 'にほんご')
        self.assertEqual(current(self.handle), 1)
        take(mode(self.handle, 0x19, 0))
        take(key(self.handle, 0x25, 0, 0, None, None))
        self.assertEqual(current(self.handle), 1)
        take(mode(self.handle, 0x19, 0))
        caret(self.handle, 0, 0)
        self.assertEqual(current(self.handle), 1)

    def test_fcitx_dispatch_routes_shortcut_before_space_and_ignores_release(self):
        trace = []
        @Query
        def query(context, operation, surrounding):
            return 0
        @Event
        def event(context, operation, amount, pointer, size):
            text = c.string_at(pointer, size).decode('utf-8') if pointer and size else ''
            trace.append((operation, text))
            return 1
        self.type('kyou')
        self.assertEqual(dispatch(self.handle, 32, 32, 1, 1, None, query, event), 0)
        self.assertEqual(trace, [])
        self.assertEqual(dispatch(self.handle, 32, 32, 1, 0, None, query, event), 1)
        self.assertEqual(state(self.handle) & 1, 1)
        self.assertIn((14, 'A'), trace)
        self.assertTrue(any(text == 'きょう' for operation, text in trace))
        self.assertFalse(any(text == '　' for operation, text in trace))

    def test_stale_handle_is_inert(self):
        self.assertIsNone(take(mode(None, 32, 1)))
        self.assertEqual(current(None), -1)

    def test_terminal_override_does_not_leak_to_a_new_content_type(self):
        attributes(self.handle, 4)
        take(mode(self.handle, 0x19, 0))
        self.assertEqual(current(self.handle), 0)
        attributes(self.handle, 0)
        attributes(self.handle, 4)
        self.assertEqual(current(self.handle), 1)

    def test_newly_saved_shortcut_works_without_typing_another_key(self):
        path = Path(self.directory.name) / 'config.json'
        settings = json.loads(path.read_text(encoding='utf-8'))
        settings['ToggleKeyboardShortcuts'] = 'Ctrl+F12'
        path.write_text(json.dumps(settings), encoding='utf-8')
        self.assertTrue(take(mode(self.handle, 0x7b, 2))['consumed'])
        self.assertEqual(current(self.handle), 1)

    def test_muhenkan_option_routes_as_composition_role_in_native_abi(self):
        path = Path(self.directory.name) / 'config.json'
        settings = json.loads(path.read_text(encoding='utf-8'))
        settings['MuhenkanKatakanaCycle'] = True
        path.write_text(json.dumps(settings), encoding='utf-8')
        self.type('katakana')
        for expected in ['カタカナ', 'ｶﾀｶﾅ', 'かたかな']:
            result = take(role(self.handle, 0x1d))
            self.assertTrue(result['consumed'])
            self.assertEqual(result['commits'], [])
            self.assertEqual(result['view']['text'], expected)
            self.assertEqual(current(self.handle), 0)

if __name__ == '__main__':
    unittest.main(verbosity=2)
