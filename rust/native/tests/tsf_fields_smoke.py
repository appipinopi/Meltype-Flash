# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Input-scope privacy and mode compartments via controlled SDK-layout COM."""
import ctypes as c
from pathlib import Path
import sys
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parent))
from tsf_range_smoke import Host, FAIL
from tsf_com_smoke import Guid, P, U32, HR, UNKNOWN

INPUT = Guid.parse("1713dd5a-68e7-4a5b-9af6-592a595c778d")
SCOPE = Guid.parse("fde1eaee-6924-4cdf-91e7-da38cff5559d")
MANAGER = Guid.parse("7dcf57ac-18ad-438b-824d-979bffb74b7c")
OPEN = Guid.parse("58273aad-01bb-4164-95c6-755ba0b5162d")
DISABLED = Guid.parse("71a5b253-1951-466b-9fbc-9c8808fa84f2")
EMPTY = Guid.parse("d7487dbf-804e-41c5-894d-ad96fd4eea13")
ole = c.WinDLL("ole32")
ole.CoTaskMemAlloc.argtypes = [c.c_size_t]
ole.CoTaskMemAlloc.restype = P


class Variant(c.Structure):
    _fields_ = [("vt", c.c_uint16), ("reserved", c.c_uint16 * 3), ("payload", P * 2)]


class ScopeValue:
    def __init__(self, host, values=()):
        self.host, self.values = host, list(values)
        self.refs = 1
        self.kind = 13
        self.null = self.fail_query = self.fail_list = self.null_list = self.fail_value = False
        self.partial_query = False
        self.table = (P * 8)()
        host.bind(self.table, 0, HR, [c.POINTER(Guid), c.POINTER(P)], self.query)
        host.bind(self.table, 1, U32, [], lambda _: self.ref(1))
        host.bind(self.table, 2, U32, [], lambda _: self.ref(-1))
        host.bind(self.table, 3, HR, [c.POINTER(P), c.POINTER(U32)], self.scopes)
        self.object = host.Object(self.table)

    def ref(self, delta):
        self.refs += delta
        assert self.refs >= 1
        return self.refs

    def query(self, this, iid, out):
        out[0] = None
        if (self.fail_query and not self.partial_query) or bytes(iid.contents) not in [bytes(SCOPE), bytes(UNKNOWN)]:
            return HR(0x80004002).value
        self.ref(1)
        out[0] = c.addressof(self.object)
        return HR(0x80004002).value if self.fail_query else 0

    def scopes(self, this, out, count):
        count[0] = len(self.values)
        out[0] = None
        if self.null_list:
            return 0
        if self.values:
            block = ole.CoTaskMemAlloc(4 * len(self.values))
            assert block
            values = c.cast(block, c.POINTER(HR))
            for index, value in enumerate(self.values):
                values[index] = value
            out[0] = block  # Rust must CoTaskMemFree even when HRESULT fails.
        return FAIL if self.fail_list else 0

    def variant(self, out):
        c.memset(out, 0, c.sizeof(Variant))
        value = c.cast(out, c.POINTER(Variant)).contents
        value.vt = self.kind
        if self.kind == 13 and not self.null:
            self.ref(1)
            value.payload[0] = c.addressof(self.object)
        elif self.kind == 3:
            c.cast(out + 8, c.POINTER(HR))[0] = 1
        return FAIL if self.fail_value else 0


class FieldHost(Host):
    def read(self, *args):
        raise AssertionError("Field metadata inspection must not read document text")

    def write(self, *args):
        raise AssertionError("Field metadata inspection must not write document text")

    def __init__(self):
        super().__init__("")
        self.context_refs = 1
        self.app_refs = 1
        self.present = [True, True]
        self.partial_property = [False, False]
        self.values = [ScopeValue(self), ScopeValue(self)]
        self.value_reads = []
        self.bind(self.context_table, 1, U32, [], lambda _: self.context_ref(1))
        self.bind(self.context_table, 2, U32, [], lambda _: self.context_ref(-1))
        self.bind(self.context_table, 13, HR, [c.POINTER(Guid), c.POINTER(P)],
                  lambda *args: self.property_at(1, *args))
        self.bind(self.property_table, 5, HR, [U32, P, P], lambda *args: self.get_value(0, *args))
        self.app_table = (P * 7)()
        self.bind(self.app_table, 1, U32, [], lambda _: self.app_ref(1))
        self.bind(self.app_table, 2, U32, [], lambda _: self.app_ref(-1))
        self.bind(self.app_table, 5, HR, [U32, P, P], lambda *args: self.get_value(1, *args))
        self.app = self.Object(self.app_table)

    def context_ref(self, delta):
        self.context_refs += delta
        return self.context_refs

    def app_ref(self, delta):
        self.app_refs += delta
        return self.app_refs

    def property_at(self, index, this, guid, out):
        assert bytes(guid.contents) == bytes(INPUT)
        out[0] = None
        if self.present[index] or self.partial_property[index]:
            if index == 0:
                self.property_refs += 1
                out[0] = c.addressof(self.property)
            else:
                self.app_ref(1)
                out[0] = c.addressof(self.app)
        return 0 if self.present[index] else FAIL

    def get_property(self, *args):
        return self.property_at(0, *args)

    def get_value(self, index, this, cookie, range, out):
        assert cookie == 7 and range == self.original
        self.value_reads.append(index)
        return self.values[index].variant(out)

    def verify(self, test):
        super().verify(test)
        test.assertEqual((self.context_refs, self.app_refs), (1, 1))
        test.assertEqual([value.refs for value in self.values], [1, 1])
        test.assertEqual(self.writes, [])


class CompartmentHost(FieldHost):
    def __init__(self):
        super().__init__()
        self.manager_refs = 1
        self.fail_manager = self.fail_get = self.partial_get = False
        self.partial_manager = False
        self.fail_read = self.fail_write = False
        self.kind = 3
        self.reads, self.writes_mode = [], []
        self.integers = {bytes(OPEN): 1, bytes(DISABLED): 0, bytes(EMPTY): 0}
        self.compartment_refs = {key: 1 for key in self.integers}
        self.manager_table = (P * 6)()
        self.bind(self.context_table, 0, HR, [c.POINTER(Guid), c.POINTER(P)], self.query)
        self.bind(self.manager_table, 1, U32, [], lambda _: self.manager_ref(1))
        self.bind(self.manager_table, 2, U32, [], lambda _: self.manager_ref(-1))
        self.bind(self.manager_table, 3, HR, [c.POINTER(Guid), c.POINTER(P)], self.get_compartment)
        self.manager = self.Object(self.manager_table)
        self.compartments = {}
        for key in self.integers:
            table = (P * 5)()
            self.bind(table, 1, U32, [], lambda _, k=key: self.compartment_ref(k, 1))
            self.bind(table, 2, U32, [], lambda _, k=key: self.compartment_ref(k, -1))
            self.bind(table, 3, HR, [U32, P], lambda *args, k=key: self.set_mode(k, *args))
            self.bind(table, 4, HR, [P], lambda *args, k=key: self.read_mode(k, *args))
            self.compartments[key] = (table, self.Object(table))
        self.on_write = None

    def query(self, this, iid, out):
        out[0] = None
        if (self.fail_manager and not self.partial_manager) or bytes(iid.contents) != bytes(MANAGER):
            return HR(0x80004002).value
        self.manager_ref(1)
        out[0] = c.addressof(self.manager)
        return HR(0x80004002).value if self.fail_manager else 0

    def manager_ref(self, delta):
        self.manager_refs += delta
        return self.manager_refs

    def compartment_ref(self, key, delta):
        self.compartment_refs[key] += delta
        return self.compartment_refs[key]

    def get_compartment(self, this, guid, out):
        key = bytes(guid.contents)
        assert key in self.compartments
        out[0] = None
        if not self.fail_get or self.partial_get:
            self.compartment_ref(key, 1)
            out[0] = c.addressof(self.compartments[key][1])
        return FAIL if self.fail_get else 0

    def read_mode(self, key, this, out):
        self.reads.append(key)
        c.memset(out, 0, c.sizeof(Variant))
        c.cast(out, c.POINTER(Variant)).contents.vt = self.kind
        if self.kind == 3:
            c.cast(out + 8, c.POINTER(HR))[0] = self.integers[key]
        elif self.kind == 13:
            self.values[0].ref(1)
            c.cast(out, c.POINTER(Variant)).contents.payload[0] = c.addressof(self.values[0].object)
        return FAIL if self.fail_read else 0

    def set_mode(self, key, this, client, value):
        assert client == 17 and c.cast(value, c.POINTER(Variant)).contents.vt == 3
        integer = c.cast(value + 8, c.POINTER(HR))[0]
        self.writes_mode.append((key, client, integer))
        if self.fail_write:
            return FAIL
        self.integers[key] = integer
        if self.on_write:
            self.on_write()
        return 0

    def verify(self, test):
        super().verify(test)
        test.assertEqual(self.manager_refs, 1)
        test.assertEqual(list(self.compartment_refs.values()), [1, 1, 1])


class FieldTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.api = c.CDLL(str(Path(sys.argv[1]).resolve()))
        cls.api.meltype_tsf_input_scope.argtypes = [P, U32]
        cls.api.meltype_tsf_input_scope.restype = U32
        cls.api.meltype_tsf_browser_input_scope.argtypes = [P, U32, HR]
        cls.api.meltype_tsf_browser_input_scope.restype = U32
        cls.api.meltype_tsf_context_disabled.argtypes = [P]
        cls.api.meltype_tsf_read_open.argtypes = [P]
        cls.api.meltype_tsf_write_open.argtypes = [P, U32, HR]

    def host(self, compartments=False):
        host = CompartmentHost() if compartments else FieldHost()
        self.addCleanup(host.verify, self)
        return host

    def inspect(self, host):
        return self.api.meltype_tsf_input_scope(c.byref(host.context), 7)

    def test_each_secret_scope_in_either_metadata_source_blocks_processing(self):
        for source in [0, 1]:
            for scope in [31, 63, 64, 65, 66]:
                with self.subTest(source=source, scope=scope):
                    host = self.host()
                    host.values[source].values = [scope]
                    self.assertEqual(self.inspect(host), 0x300 | 1)
                    self.assertEqual(host.value_reads, [0, 1])

    def test_issue_992_ascii_only_fields_are_raw_without_being_secret(self):
        for source in [0, 1]:
            for scope in [4, 5, 40]:
                with self.subTest(source=source, scope=scope):
                    host=self.host()
                    host.values[source].values=[scope]
                    self.assertEqual(self.inspect(host), 0x300 | 8)
                    host.values[1-source].values=[61]
                    self.assertEqual(self.inspect(host), 0x300 | 8 | 2)
        # URL/検索・通常テキスト・全角指定欄では日本語入力を継続する。
        for scope in [0,1,2,3,6,39,41,50,52,57,58,9999]:
            host=self.host();host.values[0].values=[scope]
            self.assertEqual(self.inspect(host), 0x300)

    def test_private_scope_is_independent_and_combines_with_secret(self):
        host = self.host()
        host.values[0].values = [61]
        self.assertEqual(self.inspect(host), 0x300 | 2)
        host.values[1].values = [31]
        self.assertEqual(self.inspect(host), 0x300 | 3)
        host.values[0].values = []
        host.values[1].values = []
        self.assertEqual(self.inspect(host), 0x300)  # No previous result is cached.

    def test_issue_878_chromium_app_password_scope_preserves_real_password_guards(self):
        host = self.host(True)
        inspect = lambda: self.api.meltype_tsf_browser_input_scope(c.byref(host.context), 7, 1)
        host.values[1].values = [31, 61]
        self.assertEqual(inspect(), 0x300 | 2)
        self.assertEqual(self.inspect(host), 0x300 | 3)  # Other applications still block.
        host.values[0].values = [31]
        self.assertEqual(inspect(), 0x300 | 3)  # Windows password metadata wins.
        host.values[0].values = []
        host.integers[bytes(DISABLED)] = 1
        self.assertEqual(inspect(), 0x300 | 3)
        host.integers[bytes(DISABLED)] = 0
        host.values[1].fail_query = True
        self.assertTrue(inspect() & 5 == 5)  # Unreadable scopes remain blocked.

    def test_absent_property_selection_and_values_are_unspecified(self):
        host = self.host()
        host.present = [False, False]
        self.assertEqual(self.inspect(host), 0)
        self.assertEqual(host.value_reads, [])
        host.present = [True, False]
        host.fail_selection = True
        self.assertEqual(self.inspect(host), 0x100)
        host.fail_selection = False
        for kind in [0, 1, 13]:
            host.values[0].kind, host.values[0].null = kind, True
            self.assertEqual(self.inspect(host), 0x100)

    def test_failed_property_and_value_outputs_are_released(self):
        host = self.host()
        host.present, host.partial_property = [False, False], [True, True]
        self.assertEqual(self.inspect(host), 0)
        host.present = [True, True]
        host.values[0].fail_value = host.values[1].fail_value = True
        self.assertEqual(self.inspect(host), 0x300)
        self.assertEqual([value.refs for value in host.values], [1, 1])

    def test_unreadable_scope_query_list_and_variant_are_secret(self):
        for failure, reason in [("fail_query", 1), ("fail_list", 2), ("null_list", 2)]:
            with self.subTest(failure=failure):
                host = self.host()
                host.values[1].values = [61]
                setattr(host.values[0], failure, True)
                host.values[0].values = [31]
                self.assertEqual(self.inspect(host), 0x300 | 7 | reason << 16)
        host = self.host()
        host.values[0].kind = 3
        self.assertEqual(self.inspect(host), 0x300 | 5 | 3 << 16)

    def test_known_ascii_and_unknown_scope_flags_are_distinct(self):
        host = self.host()
        host.values[0].values = [0, 4, 9999]
        self.assertEqual(self.inspect(host), 0x308)
        host.values[0].values = [0, 50, 9999]
        self.assertEqual(self.inspect(host), 0x300)

    def test_failed_query_with_partial_interface_output_releases_references(self):
        host = self.host()
        host.values[0].fail_query = host.values[0].partial_query = True
        self.assertEqual(self.inspect(host), 0x300 | 5 | 1 << 16)
        manager = self.host(True)
        manager.fail_manager = manager.partial_manager = True
        self.assertEqual(self.api.meltype_tsf_read_open(c.byref(manager.context)), -2)

    def test_open_compartment_strict_integer_read_and_write(self):
        host = self.host(True)
        context = c.byref(host.context)
        self.assertEqual(self.api.meltype_tsf_read_open(context), 1)
        self.assertEqual(self.api.meltype_tsf_write_open(context, 17, 0), 1)
        self.assertEqual(self.api.meltype_tsf_read_open(context), 0)
        self.assertEqual(self.api.meltype_tsf_write_open(context, 17, -5), 1)
        self.assertEqual(host.writes_mode[-1], (bytes(OPEN), 17, 1))
        host.kind = 13
        self.assertEqual(self.api.meltype_tsf_read_open(context), 0)

    def test_open_unavailable_and_unsupported_are_distinct(self):
        host = self.host(True)
        context = c.byref(host.context)
        host.fail_manager = True
        self.assertEqual(self.api.meltype_tsf_read_open(context), -2)
        self.assertEqual(self.api.meltype_tsf_write_open(context, 17, 1), 0)
        host.fail_manager = False
        host.fail_get = host.partial_get = True
        self.assertEqual(self.api.meltype_tsf_read_open(context), -1)
        host.fail_get = False
        host.fail_read, host.kind = True, 13
        self.assertEqual(self.api.meltype_tsf_read_open(context), -1)
        host.fail_read, host.fail_write = False, True
        self.assertEqual(self.api.meltype_tsf_write_open(context, 17, 1), 0)

    def test_disabled_and_empty_context_flags_require_nonzero_i4(self):
        host = self.host(True)
        for disabled, empty in [(0, 0), (1, 0), (0, -1), (1, 1)]:
            host.integers[bytes(DISABLED)], host.integers[bytes(EMPTY)] = disabled, empty
            host.reads.clear()
            self.assertEqual(self.api.meltype_tsf_context_disabled(c.byref(host.context)), int(bool(disabled or empty)))
            self.assertEqual(host.reads, [bytes(DISABLED), bytes(EMPTY)])
        host.kind = 13
        self.assertEqual(self.api.meltype_tsf_context_disabled(c.byref(host.context)), 0)
        host.fail_manager = True
        self.assertEqual(self.api.meltype_tsf_context_disabled(c.byref(host.context)), 0)

    def test_write_can_reenter_read_without_unbalanced_com_references(self):
        host = self.host(True)
        observed = []
        host.on_write = lambda: observed.append(self.api.meltype_tsf_read_open(c.byref(host.context)))
        self.assertEqual(self.api.meltype_tsf_write_open(c.byref(host.context), 17, 0), 1)
        self.assertEqual(observed, [0])

    def test_null_context_is_disabled_and_has_unspecified_scope(self):
        self.assertEqual(self.api.meltype_tsf_input_scope(None, 7), 0)
        self.assertEqual(self.api.meltype_tsf_context_disabled(None), 1)
        self.assertEqual(self.api.meltype_tsf_read_open(None), -2)
        self.assertEqual(self.api.meltype_tsf_write_open(None, 17, 1), 0)


if __name__ == "__main__":
    unittest.main(argv=[sys.argv[0]], verbosity=2, failfast=True)
