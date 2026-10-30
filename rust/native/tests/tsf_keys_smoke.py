# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Original key-helper parity and controlled key-acceptance callbacks.

Only this test thread's keyboard-state array is changed/restored. No SendInput,
IME activation, real document edits or user keyboard-layout changes are used.
"""
import ctypes as c
from pathlib import Path
import string
import sys
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parent))
from tsf_com_smoke import P, U32, HR
from tsf_fields_smoke import CompartmentHost, DISABLED
from tsf_candidate_smoke import Message, user
U64, W = c.c_uint64, c.c_uint16
user.GetKeyboardState.argtypes = [c.POINTER(c.c_ubyte)]
user.SetKeyboardState.argtypes = [c.POINTER(c.c_ubyte)]


class Snapshot(c.Structure):
    _fields_ = [("composition", P), ("context", P), ("disabled", HR), ("open", HR),
                ("detection", HR), ("english", HR), ("tries", U32), ("vk", U32), ("time", U64),
                ("full_width_space", HR)]
READ = c.CFUNCTYPE(None, P, c.POINTER(Snapshot))
UPDATE = c.CFUNCTYPE(None, P, U32, U64, U64)
RESUME = c.CFUNCTYPE(None, P, P)
NOW = c.CFUNCTYPE(HR, P, HR)
LATER = c.CFUNCTYPE(None, P, HR)
ABANDON = c.CFUNCTYPE(None, P)
READY = c.CFUNCTYPE(HR, P)
SECRET = c.CFUNCTYPE(HR, P, U32, P, HR)
PASS = c.CFUNCTYPE(None, P, U32)


class Callbacks(c.Structure):
    _fields_ = [("owner", P), ("lifetime", P), ("read", READ), ("update", UPDATE),
                ("resume", RESUME), ("now", NOW), ("later", LATER), ("abandon", ABANDON),
                ("ready", READY), ("secret", SECRET), ("pass_reason", PASS)]


class KeyboardTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.api = c.CDLL(str(Path(sys.argv[1]).resolve()))
        cls.reference = c.CDLL(str(Path(sys.argv[2]).resolve()))
        for api, prefix in [(cls.api, "meltype_tsf_"), (cls.reference, "meltype_reference_")]:
            for name, args in [("key_flags", [U32, U64] + ([U32] if api is cls.api else [])),
                               ("key_char", [U32, U64]), ("key_down", [HR])]:
                method = getattr(api, prefix + name)
                method.argtypes, method.restype = args, U32
        cls.api.meltype_tsf_key_modifiers.restype = U32
        cls.api.meltype_tsf_would_eat.argtypes = [P, U32, U64, HR, c.POINTER(Callbacks), c.POINTER(W)]

    def setUp(self):
        message = Message()
        user.PeekMessageW(c.byref(message), None, 0, 0, 0)
        self.saved = (c.c_ubyte * 256)()
        self.assertTrue(user.GetKeyboardState(self.saved))
        self.addCleanup(user.SetKeyboardState, self.saved)
        self.mods(0)

    def mods(self, mask, caps=False):
        state = (c.c_ubyte * 256)()
        state[0x90] = 1  # NumLock, local to this test thread.
        state[0x14] = int(caps)
        for bit, keys in [(1, [0x10, 0xa0]), (2, [0x11, 0xa2]), (4, [0x12, 0xa4]), (8, [0x5b])]:
            for key in keys:
                if mask & bit:
                    state[key] = 0x80
        self.assertTrue(user.SetKeyboardState(state))

    def test_all_virtual_keys_modifiers_and_toggle_message_forms_match_original(self):
        messages = [0, 0x29 << 16, 1 << 29, (1 << 29) | (0x29 << 16), (1 << 29) | (0x29 << 16) | (1 << 24)]
        for mods in range(16):
            self.mods(mods)
            self.assertEqual(self.api.meltype_tsf_key_modifiers(), mods)
            for key in range(256):
                for message in messages:
                    expected = self.reference.meltype_reference_key_flags(key, message)
                    # #599: the frozen reference predates CompatibleShortcuts.
                    # K/L must reach the core, which decides from the setting.
                    if key in [0x4b, 0x4c] and mods & 2 and not mods & 12:
                        expected |= 8
                    self.assertEqual(self.api.meltype_tsf_key_flags(key, message, mods), expected, (key, message, mods))

    def test_utf16_translation_matches_original_for_all_keys_and_modifier_states(self):
        for caps in [False, True]:
            for mods in range(16):
                self.mods(mods, caps)
                for key in range(256):
                    message = int(user.MapVirtualKeyW(key, 0) & 0xff) << 16
                    self.assertEqual(self.api.meltype_tsf_key_char(key, message),
                        self.reference.meltype_reference_key_char(key, message), (key, mods, caps))

    def test_key_state_and_translation_leave_thread_state_unchanged(self):
        self.mods(3, True)
        before, after = (c.c_ubyte * 256)(), (c.c_ubyte * 256)()
        self.assertTrue(user.GetKeyboardState(before))
        for key in range(256):
            self.assertEqual(self.api.meltype_tsf_key_down(key), self.reference.meltype_reference_key_down(key))
            self.api.meltype_tsf_key_char(key, 0)
        self.assertTrue(user.GetKeyboardState(after))
        self.assertEqual(bytes(before), bytes(after))


class KeyHost(CompartmentHost):
    def __init__(self):
        super().__init__()
        self.on_metadata = None

    def read_mode(self, *args):
        result = super().read_mode(*args)
        if self.on_metadata:
            callback, self.on_metadata = self.on_metadata, None
            callback()
        return result


class AcceptanceTests(KeyboardTests):
    # Parity tables above are inherited intentionally only by KeyboardTests.
    test_all_virtual_keys_modifiers_and_toggle_message_forms_match_original = None
    test_utf16_translation_matches_original_for_all_keys_and_modifier_states = None
    test_key_state_and_translation_leave_thread_state_unchanged = None

    def setUp(self):
        super().setUp()
        self.host = KeyHost()
        self.addCleanup(self.host.verify, self)
        self.s = Snapshot(None, None, 0, 1, 0, 0, 0, 0, 0)
        self.events, self.errors = [], []
        self.checked = self.secret_while = False
        self.sync = self.active = True
        self.secret_result = 0
        def protect(function, default=None):
            def callback(*args):
                try:
                    assert args[0] == 123
                    self.assertGreaterEqual(self.host.context_refs, 3)
                    return function(*args)
                except BaseException as error:
                    self.errors.append(repr(error))
                    return default
            return callback
        self.read = READ(protect(lambda _, out: out.__setitem__(0, self.s)))
        self.update = UPDATE(protect(self.update_state))
        self.resume = RESUME(protect(lambda _, context: self.events.append(("resume", context))))
        self.now = NOW(protect(self.commit_now, 0))
        self.later = LATER(protect(lambda _, keep: self.events.append(("later", keep))))
        self.abandon = ABANDON(protect(self.abandon_old))
        self.ready = READY(protect(lambda _: int(self.active), 0))
        self.secret = SECRET(protect(self.read_secret, 1))
        self.pass_reason = PASS(protect(lambda _, reason: self.events.append(("pass", reason))))
        self.callbacks = Callbacks(123, c.addressof(self.host.context), self.read, self.update,
            self.resume, self.now, self.later, self.abandon, self.ready, self.secret, self.pass_reason)
        self.addCleanup(lambda: self.assertEqual(self.errors, []))

    def update_state(self, _, op, a, b):
        self.events.append(("update", op))
        if op == 0:
            self.checked = self.secret_while = False
        elif op == 1:
            self.secret_while = True
        elif op == 2:
            self.s.english = 0
        elif op == 3:
            self.s.tries, self.s.vk, self.s.time = a >> 32, a & 0xffffffff, b
        elif op == 4:
            self.checked = bool(a)

    def commit_now(self, _, keep):
        self.events.append(("now", keep))
        if self.sync:
            self.s.composition, self.s.context = None, None
        return int(self.sync)

    def abandon_old(self, _):
        self.events.append(("abandon",))
        self.s.composition, self.s.context, self.s.tries, self.s.vk = None, None, 0, 0

    def read_secret(self, _, key, context, test):
        assert context == c.addressof(self.host.context)
        self.events.append(("secret", key, test))
        return self.secret_result

    def dispatch(self, key=0x20, message=0, test=True):
        self.events.clear()
        out = W(999)
        result = self.api.meltype_tsf_would_eat(c.byref(self.host.context), key, message,
            int(test), c.byref(self.callbacks), c.byref(out))
        self.assertEqual(self.errors, [])
        return bool(result), out.value

    def composing(self, other=False):
        self.s.composition = 9876  # Identity token; acceptance never dereferences it.
        self.s.context = 1234 if other else c.addressof(self.host.context)

    def test_disabled_invalid_and_modifier_keys_do_not_resume_input(self):
        for key, disabled in [(0x10, 0), (256, 0), (0x41, 1)]:
            self.s.disabled = disabled
            self.assertEqual(self.dispatch(key), (False, 0))
            self.assertEqual(self.events, [("update", 0)])

    def test_issue_881_plain_space_depends_on_setting_and_japanese_mode(self):
        for enabled in [False, True]:
            for opened in [False, True]:
                self.s.open = int(opened)
                self.s.detection = 1
                self.s.full_width_space = int(enabled)
                self.mods(0)
                self.assertEqual(self.dispatch(0x20)[0], enabled and opened)
                self.mods(1)
                self.assertTrue(self.dispatch(0x20)[0])

    def test_unicode_packet_commits_before_passing_but_process_key_does_not(self):
        self.composing()
        self.sync = False
        self.assertEqual(self.dispatch(0xe7), (False, 0))
        self.assertEqual(self.events[-2:], [("now", 1), ("later", 1)])
        self.assertEqual(self.dispatch(0xe5), (False, 0))
        self.assertFalse(any(event[0] == "now" for event in self.events))

    def test_stale_field_counts_test_down_pair_once_then_abandons_on_next_key(self):
        self.composing(other=True)
        self.sync = False
        self.assertFalse(self.dispatch(0x41)[0])
        self.assertEqual(self.s.tries, 1)
        self.assertIn(("later", 1), self.events)
        kernel = c.WinDLL("kernel32")
        kernel.GetTickCount64.restype = U64
        self.s.time = kernel.GetTickCount64()
        self.assertFalse(self.dispatch(0x41, test=False)[0])
        self.assertEqual(self.s.tries, 1)
        self.assertNotIn(("later", 1), self.events)
        self.mods(1)
        self.assertTrue(self.dispatch(0x20)[0])
        self.assertIn(("abandon",), self.events)
        self.assertEqual(self.s.tries, 0)

    def test_disabled_field_during_composition_commits_or_defers_key_processing(self):
        self.host.integers[bytes(DISABLED)] = 1
        self.composing()
        self.assertFalse(self.dispatch(0x41)[0])
        self.assertIn(("now", 1), self.events)
        self.composing()
        self.sync = False
        self.assertTrue(self.dispatch(0x41)[0])
        self.assertTrue(self.secret_while)

    def test_composition_ctrl_shortcuts_remain_internal_and_other_shortcuts_commit(self):
        self.mods(2)
        for key in [0x48, 0x55, 0x49, 0x4b, 0x4c, 0x4f, 0x50]:
            self.composing()
            self.assertTrue(self.dispatch(key)[0])
            self.assertFalse(any(event[0] == "now" for event in self.events))
        self.composing()
        self.assertFalse(self.dispatch(0x53)[0])
        self.assertIn(("now", 0), self.events)
        self.composing()
        self.sync = False
        self.assertTrue(self.dispatch(0x53)[0])

    def test_mode_toggle_keys_bypass_core_and_field_checks_with_original_modifiers(self):
        self.host.integers[bytes(DISABLED)] = 1
        self.active = False
        self.assertTrue(self.dispatch(0x19)[0])
        self.assertTrue(self.dispatch(0xc0, 1 << 29)[0])
        self.mods(2)
        self.assertFalse(self.dispatch(0x19)[0])
        self.assertFalse(self.dispatch(0xc0, 1 << 29)[0])

    def test_inactive_english_modifiers_and_disabled_fields_pass_before_privacy_query(self):
        self.s.open = 0
        self.assertFalse(self.dispatch(0x41)[0])
        self.assertEqual(self.events[-1], ("pass", 1))
        self.s.open = 1
        self.mods(2)
        self.assertFalse(self.dispatch(0x41)[0])
        self.assertEqual(self.events[-1], ("pass", 2))
        self.mods(0)
        self.host.integers[bytes(DISABLED)] = 1
        self.assertFalse(self.dispatch(0x41)[0])
        self.assertEqual(self.events[-1], ("pass", 3))

    def test_shift_space_starts_and_plain_space_does_not(self):
        self.assertFalse(self.dispatch()[0])
        self.mods(1)
        self.assertEqual(self.dispatch(), (True, 0x20))
        self.assertTrue(self.checked)

    def test_explicit_convert_is_available_in_direct_mode_but_obeys_field_and_privacy(self):
        self.s.open=0;self.s.detection=0
        self.assertTrue(self.dispatch(0x1c)[0]);self.assertTrue(self.checked)
        self.mods(2);self.assertFalse(self.dispatch(0x1c)[0])
        self.mods(0);self.secret_result=1;self.assertFalse(self.dispatch(0x1c)[0])
        self.secret_result=0;self.host.integers[bytes(DISABLED)]=1
        self.assertFalse(self.dispatch(0x1c)[0])

    def test_secret_scope_blocks_and_unknown_scope_is_checked_during_edit(self):
        self.mods(1)
        self.secret_result = 1
        self.assertFalse(self.dispatch()[0])
        self.assertEqual(self.events[-1], ("pass", 6))
        self.secret_result = -1
        self.assertTrue(self.dispatch(test=False)[0])
        self.assertFalse(self.checked)
        self.assertIn(("secret", 0x20, 0), self.events)
        self.assertEqual(self.events[-1], ("pass", 0))

    def test_unavailable_core_does_not_query_secret_scope(self):
        self.mods(1)
        self.active = False
        self.assertFalse(self.dispatch()[0])
        self.assertEqual(self.events[-1], ("pass", 5))
        self.assertFalse(any(event[0] == "secret" for event in self.events))

    def test_direct_english_continuation_and_word_boundary(self):
        self.s.open, self.s.detection, self.s.english = 0, 1, 1
        key = 0x41
        ch = self.api.meltype_tsf_key_char(key, 0)
        accepted, _ = self.dispatch(key)
        if chr(ch) in string.ascii_letters + string.digits + "@_":
            self.assertFalse(accepted)
            self.assertEqual(self.events[-1], ("pass", 4))
        self.s.english = 1
        self.mods(1)
        self.assertTrue(self.dispatch()[0])
        self.assertEqual(self.s.english, 0)
        self.assertIn(("update", 2), self.events)

    def test_mode_state_is_refreshed_after_field_metadata_callbacks(self):
        self.s.open, self.s.detection, self.s.english = 0, 1, 1
        self.mods(1)
        self.host.on_metadata = lambda: setattr(self.s, "open", 1)
        self.assertTrue(self.dispatch()[0])
        self.assertEqual(self.s.english, 1)
        self.assertNotIn(("update", 2), self.events)

    def test_shift_state_is_refreshed_after_field_metadata_callbacks(self):
        self.host.on_metadata = lambda: self.mods(1)
        self.assertEqual(self.dispatch(), (True, 0x20))


if __name__ == "__main__":
    unittest.main(argv=[sys.argv[0]], verbosity=2, failfast=True)
