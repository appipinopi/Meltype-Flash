# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""IBusの実際のルーティング関数をGIなしで検査する。"""
import ast
from pathlib import Path
from types import SimpleNamespace
import unittest


class ModeRoutingTests(unittest.TestCase):
    def setUp(self):
        source = Path(__file__).resolve().parents[1] / 'ibus-engine-meltype'
        tree = ast.parse(source.read_text(encoding='utf-8'))
        engine = next(node for node in tree.body if isinstance(node, ast.ClassDef) and node.name == 'MeltypeEngine')
        names = {'do_process_key_event', '_switch_mode', 'do_focus_in'}
        functions = [node for node in engine.body if isinstance(node, ast.FunctionDef) and node.name in names]
        self.calls = []
        self.ns = {
            'IBus': SimpleNamespace(ModifierType=SimpleNamespace(RELEASE_MASK=1 << 30)),
            'ROLE_KEYS': {0xff23: 0x1c, 0xff22: 0x1d},
            'ZENKAKU_HANKAKU_KEYS': (0xff2a,),
            'ALWAYS_DIRECT_KEYS': (0xff22,),
            'MODE_SWITCH_KEYS': (0xff2a, 0xff22, 0xff23),
            '_role_key': None, '_mode_key': None, '_input_mode': None, '_caret_moved': None,
            '_result': lambda result: result,
            'virtual_key': lambda key: (32, 32) if key == 32 else (None, 0),
            'modifier_flags': lambda state: state,
            '_lib': SimpleNamespace(
                meltype_commit=lambda handle: self.calls.append('commit') or {'commits': []},
                meltype_set_direct=lambda handle, direct: self.calls.append(('direct', direct)),
            ),
        }
        exec(compile(ast.Module(body=functions, type_ignores=[]), str(source), 'exec'), self.ns)
        cls = type('Engine', (), {name: self.ns[name] for name in names})
        self.engine = cls()
        self.engine._session = 7
        self.engine._direct = False
        self.engine._properties = []
        self.engine._check_replace = lambda: None
        self.engine._apply = lambda result: self.calls.append(('apply', result))
        self.engine.register_properties = lambda properties: self.calls.append('properties')

    def test_shift_space_reaches_mode_api_before_normal_space_processing(self):
        self.ns['_mode_key'] = lambda handle, vk, mods: self.calls.append((handle, vk, mods)) or {'commits': []}
        self.ns['_input_mode'] = lambda handle: 1
        self.assertTrue(self.engine.do_process_key_event(32, 0, 1))
        self.assertEqual(self.calls[0], (7, 32, 1))
        self.assertTrue(self.engine._direct)
        self.assertNotIn('commit', self.calls)

    def test_release_does_not_invoke_mode_api(self):
        self.ns['_mode_key'] = lambda *args: self.fail('release routed to mode API')
        self.ns['_input_mode'] = lambda handle: 1
        self.assertFalse(self.engine.do_process_key_event(32, 0, 1 << 30))

    def test_old_library_uses_existing_builtin_switch(self):
        self.assertTrue(self.engine.do_process_key_event(0xff2a, 0, 0))
        self.assertTrue(self.engine._direct)
        self.assertIn(('direct', 1), self.calls)

    def test_focus_reentry_clears_code_line_override_and_resynchronizes_mode(self):
        self.ns['_caret_moved'] = lambda *args: self.calls.append(args)
        self.ns['_input_mode'] = lambda handle: 1
        self.engine.do_focus_in()
        self.assertEqual(self.calls, [(7, 0, 0), 'properties'])
        self.assertTrue(self.engine._direct)


if __name__ == '__main__':
    unittest.main(verbosity=2)
