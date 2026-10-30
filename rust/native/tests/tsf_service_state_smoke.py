# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Native state layout/defaults, independent storage and balanced lifetimes."""
import ctypes as c
import json
import os
import tempfile
import time
from pathlib import Path
import sys
import threading
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parent))
from tsf_com_smoke import Guid, HR, invoke, release
from tsf_startup_smoke import StartupHost,CATEGORY
from tsf_subscriptions_smoke import SubscriptionHost
from tsf_candidate_smoke import windows, user
from tsf_candidate_view_smoke import GeometryHost
from tsf_abandoned_smoke import DeferredHost
from tsf_fields_smoke import CompartmentHost,OPEN
from tsf_service_com_smoke import DISPATCH,DESTROY,Call,IDS
from tsf_edit_events_smoke import EventHost,QUERY
from tsf_open_change_smoke import Host as OpenHost
from tsf_shutdown_smoke import REQUEST as SHUTDOWN_REQUEST
from tsf_composition_owner_smoke import OwnerHost
from tsf_range_smoke import units
from tsf_mode_smoke import ModeHost
from tsf_key_state_smoke import SENDER
OBJECT_LOCK=c.CFUNCTYPE(None,c.c_int)
SELECT=c.CFUNCTYPE(None,c.c_void_p,c.c_int,c.c_int)
CORE_NOTICE=c.CFUNCTYPE(None,c.c_void_p,c.c_uint32,c.c_int)
END=c.CFUNCTYPE(None,c.c_void_p,c.c_uint32)
COMMIT_APPLY=c.CFUNCTYPE(c.c_int,c.c_void_p,c.c_uint32,c.c_void_p,c.c_void_p,c.c_uint32)
KEY_COMMIT=c.CFUNCTYPE(c.c_int,c.c_void_p,c.c_uint32,c.c_int)
KEY_HANDLE=c.CFUNCTYPE(None,c.c_void_p,c.c_void_p,c.c_uint32,c.c_uint16)
FOCUS_COMMIT=c.CFUNCTYPE(c.c_int,c.c_void_p,c.c_int)
REPLY=c.CFUNCTYPE(c.c_int,c.c_void_p,c.c_void_p,c.c_uint32)
CORE_CLOCK=c.CFUNCTYPE(c.c_uint64,c.c_void_p)
NOTICE=c.CFUNCTYPE(None,c.c_void_p,c.c_int)
READ=c.CFUNCTYPE(c.c_int,c.c_void_p)
TOGGLE=c.CFUNCTYPE(None,c.c_void_p)
class Point(c.Structure):
    _fields_=[('x',HR),('y',HR)]
P=c.c_void_p
U32=c.c_uint32
class Stale(c.Structure):
    _fields_=[("tries",U32),("key",U32),("time",c.c_uint64)]
class State(c.Structure):
    _fields_=[
        ('refs',c.c_int32),
        ('manager',P),
        ('client',U32),
        ('thread_subscription',P),
        ('key_subscription',P),
        ('mode_subscription',P),
        ('edit_subscription',P),
        ('edit_context',P),
        ('composition',P),
        ('composition_context',P),
        ('atom_input',U32),
        ('atom_converted',U32),
        ('atom_target',U32),
        ('keys',P),
        ('resume',P),
        ('abandoned',P),
        ('last_hello',c.c_uint64),
        ('focus_epoch',c.c_uint64),
        ('mode_epoch',c.c_uint64),
        ('shutdown_phase',U32),
        ('server_active',c.c_bool),
        ('direct_detection',c.c_bool),
        ('direct_word',c.c_bool),
        ('open',c.c_bool),
        ('code_app',c.c_bool),
        ('code_english',c.c_bool),
        ('disabled',c.c_bool),
        ('own_process',c.c_bool),
        ('last_converting',c.c_bool),
        ('caret_moved',c.c_bool),
        ('own_edit',c.c_bool),
        ('secret_checked',c.c_bool),
        ('secret_composing',c.c_bool),
        ('private_field',c.c_bool),
        ('keep_caret',c.c_bool),
        ('deactivated',c.c_bool),
        ('key_failed',c.c_bool),
        ('consumed',c.c_bool),
        ('scope_source',P),
        ('pass_reason',P),
        ('sid',P),
        ('transport',P),
        ('langbar',P),
        ('candidates',P),
        ('edit_epoch',c.c_uint64),
        ('full_width_space',c.c_bool),
    ]
class StateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.ole=c.WinDLL('ole32')
        if cls.ole.CoInitializeEx(None,2) not in (0,1):raise AssertionError('private COM apartment failed')
        cls.api=c.CDLL(str(Path(sys.argv[1]).resolve()))
        cls.api.meltype_tsf_service_state_create.restype=P
        cls.api.meltype_tsf_service_state_destroy.argtypes=[P]
        cls.api.meltype_tsf_service_state_destroy.restype=None
        cls.api.meltype_tsf_service_state_layout.argtypes=[c.POINTER(U32),U32]
        cls.api.meltype_tsf_service_state_layout.restype=U32
        cls.api.meltype_tsf_service_state_live_count.restype=U32
        cls.api.meltype_tsf_service_set_sid.argtypes=[P,P];cls.api.meltype_tsf_service_set_sid.restype=c.c_int
        cls.api.meltype_tsf_service_sid.argtypes=[P];cls.api.meltype_tsf_service_sid.restype=P
        cls.api.meltype_tsf_service_request.argtypes=[P,c.c_char_p,c.c_char_p,U32,c.c_int];cls.api.meltype_tsf_service_request.restype=P
        cls.api.meltype_tsf_service_close.argtypes=[P];cls.api.meltype_tsf_service_close.restype=None
        cls.api.meltype_tsf_service_initialize.argtypes=[P,U32,c.c_int,c.c_int,c.c_char_p];cls.api.meltype_tsf_service_initialize.restype=c.c_int
        cls.api.meltype_tsf_service_clear_resume.argtypes=[P];cls.api.meltype_tsf_service_clear_resume.restype=None
        cls.api.meltype_tsf_service_edit_sink.argtypes=[P,P,P,P];cls.api.meltype_tsf_service_edit_sink.restype=HR
        cls.api.meltype_tsf_service_mode.argtypes=[P];cls.api.meltype_tsf_service_mode.restype=U32
        cls.api.meltype_tsf_service_write_open.argtypes=[P,c.c_int];cls.api.meltype_tsf_service_write_open.restype=c.c_int
        cls.api.meltype_tsf_service_apply_mode.argtypes=[P,U32,P,P,NOTICE];cls.api.meltype_tsf_service_apply_mode.restype=c.c_int
        cls.api.meltype_tsf_service_com_create.argtypes=[P,DISPATCH,DESTROY];cls.api.meltype_tsf_service_com_create.restype=P
        cls.api.meltype_tsf_service_core.argtypes=[P,c.c_char_p,P,P,CORE_NOTICE,CORE_CLOCK,c.c_int];cls.api.meltype_tsf_service_core.restype=c.c_int
        cls.api.meltype_tsf_service_exchange.argtypes=[P,c.c_char_p,P,P,U32,U32,c.c_int,P,REPLY,TOGGLE];cls.api.meltype_tsf_service_exchange.restype=c.c_int
        cls.api.meltype_tsf_service_focus.argtypes=[P,c.c_char_p,P,P,P,c.c_int,P,FOCUS_COMMIT,TOGGLE,CORE_NOTICE];cls.api.meltype_tsf_service_focus.restype=HR
        cls.api.meltype_tsf_service_edited.argtypes=[P,P,P,P,U32,P,QUERY,TOGGLE];cls.api.meltype_tsf_service_edited.restype=c.c_int
        cls.api.meltype_tsf_start_composition.argtypes=[P,P,P,U32];cls.api.meltype_tsf_start_composition.restype=P
        cls.api.meltype_tsf_service_opened.argtypes=[P,P,c.POINTER(Guid),P,TOGGLE,NOTICE];cls.api.meltype_tsf_service_opened.restype=HR
        cls.api.meltype_tsf_service_deactivate.argtypes=[P,P,P,SHUTDOWN_REQUEST,END,NOTICE];cls.api.meltype_tsf_service_deactivate.restype=HR
        cls.api.meltype_tsf_end_composition.argtypes=[P,P,U32];cls.api.meltype_tsf_end_composition.restype=HR
        cls.api.meltype_tsf_service_activate.argtypes=[P,P,c.c_char_p,P,U32,U32,c.POINTER(P),P,SELECT,READ,TOGGLE,CORE_NOTICE,CATEGORY];cls.api.meltype_tsf_service_activate.restype=HR
        cls.api.meltype_tsf_service_apply_composition.argtypes=[P,P,P,P,U32,c.c_char_p];cls.api.meltype_tsf_service_apply_composition.restype=c.c_int
        cls.api.meltype_tsf_service_write_commit.argtypes=[P,P,P,P,P,U32,c.POINTER(c.c_uint16),U32];cls.api.meltype_tsf_service_write_commit.restype=c.c_int
        cls.api.meltype_tsf_service_end_composition.argtypes=[P,P,U32];cls.api.meltype_tsf_service_end_composition.restype=c.c_int
        cls.api.meltype_tsf_service_stop.argtypes=[P,P,P,SHUTDOWN_REQUEST,NOTICE];cls.api.meltype_tsf_service_stop.restype=HR
        cls.api.meltype_tsf_service_terminated.argtypes=[P,P,c.c_char_p,U32,P,P,TOGGLE];cls.api.meltype_tsf_service_terminated.restype=HR
        cls.api.meltype_tsf_service_resume.argtypes=[P,P,c.c_char_p,P,P,P,TOGGLE];cls.api.meltype_tsf_service_resume.restype=c.c_int
        cls.api.meltype_tsf_service_abandon.argtypes=[P,P,c.c_char_p,P,TOGGLE];cls.api.meltype_tsf_service_abandon.restype=c.c_int
        cls.api.meltype_tsf_abandoned_reset.argtypes=[P];cls.api.meltype_tsf_abandoned_reset.restype=None
        cls.api.meltype_tsf_service_input_scope.argtypes=[P,P,P,U32,P,CORE_NOTICE];cls.api.meltype_tsf_service_input_scope.restype=c.c_int
        cls.api.meltype_tsf_service_mode_action.argtypes=[P,P,c.c_char_p,P,U32,U32,c.c_int,P,CORE_NOTICE];cls.api.meltype_tsf_service_mode_action.restype=c.c_int
        cls.api.meltype_tsf_service_commit.argtypes=[P,P,c.c_char_p,U32,P,U32,c.c_int,P,COMMIT_APPLY,TOGGLE];cls.api.meltype_tsf_service_commit.restype=HR
        cls.api.meltype_tsf_service_select_candidate.argtypes=[P,P,c.c_char_p,c.c_int,c.c_int,P,COMMIT_APPLY,TOGGLE];cls.api.meltype_tsf_service_select_candidate.restype=HR
        cls.api.meltype_tsf_service_handle_key.argtypes=[P,P,P,c.c_char_p,P,U32,c.c_uint16,P,COMMIT_APPLY,CORE_NOTICE,SENDER];cls.api.meltype_tsf_service_handle_key.restype=HR
        cls.api.meltype_tsf_service_apply_reply.argtypes=[P,P,P,c.c_char_p,P,U32,P,U32,P,TOGGLE];cls.api.meltype_tsf_service_apply_reply.restype=c.c_int
        cls.api.meltype_tsf_service_shutdown.argtypes=[P,P,P,c.c_char_p,P,CORE_NOTICE];cls.api.meltype_tsf_service_shutdown.restype=HR
        cls.api.meltype_tsf_service_dispatch_key.argtypes=[P,P,P,c.c_char_p,U32,P,U32,c.c_uint64,P,CORE_NOTICE,CORE_NOTICE,SENDER];cls.api.meltype_tsf_service_dispatch_key.restype=c.c_int
        cls.api.meltype_tsf_service_focus_native.argtypes=[P,P,P,P,c.c_char_p,P,c.c_int,P,CORE_NOTICE];cls.api.meltype_tsf_service_focus_native.restype=HR
        cls.api.meltype_tsf_service_edited_native.argtypes=[P,P,P,c.c_char_p,P,P,U32,P,CORE_NOTICE];cls.api.meltype_tsf_service_edited_native.restype=c.c_int
        cls.api.meltype_tsf_service_opened_native.argtypes=[P,P,P,c.c_char_p,P,P,CORE_NOTICE];cls.api.meltype_tsf_service_opened_native.restype=HR
        cls.api.meltype_tsf_service_activate_native.argtypes=[P,P,c.c_char_p,P,U32,U32,P,P,CORE_NOTICE,CORE_NOTICE,CATEGORY];cls.api.meltype_tsf_service_activate_native.restype=HR
        cls.api.meltype_tsf_service_route.argtypes=[P,P,c.c_char_p,P,P,CORE_NOTICE,CORE_NOTICE,CORE_NOTICE,CORE_NOTICE];cls.api.meltype_tsf_service_route.restype=HR
        cls.api.meltype_tsf_service_object_create.argtypes=[c.c_char_p,P,c.POINTER(P),P,CORE_NOTICE,CORE_NOTICE,CORE_NOTICE,CORE_NOTICE,OBJECT_LOCK];cls.api.meltype_tsf_service_object_create.restype=HR
        cls.api.meltype_tsf_service_key_secret.argtypes=[P,P,P,U32,c.c_int,P,CORE_NOTICE];cls.api.meltype_tsf_service_key_secret.restype=c.c_int
        cls.api.meltype_tsf_service_would_eat.argtypes=[P,P,P,c.c_char_p,P,U32,c.c_uint64,c.c_int,P,KEY_COMMIT,CORE_NOTICE,c.POINTER(c.c_uint16)];cls.api.meltype_tsf_service_would_eat.restype=c.c_int
        cls.api.meltype_tsf_service_key_event.argtypes=[P,P,P,c.c_char_p,U32,P,U32,c.c_uint64,P,KEY_COMMIT,KEY_HANDLE,CORE_NOTICE];cls.api.meltype_tsf_service_key_event.restype=c.c_int
        cls.api.meltype_tsf_key_cache_clear.argtypes=[P];cls.api.meltype_tsf_key_cache_clear.restype=None
        cls.api.meltype_tsf_key_passed.argtypes=[P,c.c_int,c.c_int,P];cls.api.meltype_tsf_key_passed.restype=c.c_int
        user.GetKeyboardState.argtypes=[c.POINTER(c.c_ubyte)];user.SetKeyboardState.argtypes=[c.POINTER(c.c_ubyte)]
        cls.fixture=str(Path(sys.argv[2]).resolve()).encode() if len(sys.argv)>2 else None


        cls.api.meltype_tsf_key_state_create.restype=P
        cls.api.meltype_tsf_key_stale_write.argtypes=[P,U32,U32,c.c_uint64]
        cls.api.meltype_tsf_key_stale_read.argtypes=[P,c.POINTER(Stale)]
        cls.api.meltype_tsf_capture_resume.argtypes=[P,P,U32];cls.api.meltype_tsf_capture_resume.restype=P
        cls.api.meltype_tsf_abandoned_create.restype=P
        cls.api.meltype_tsf_abandoned_add.argtypes=[P,P,P,P,U32];cls.api.meltype_tsf_abandoned_add.restype=HR

        cls.api.meltype_tip_transport_request.argtypes=[P,c.c_char_p,U32,c.c_int];cls.api.meltype_tip_transport_request.restype=P
        cls.api.meltype_free.argtypes=[P]
        cls.api.meltype_tsf_service_create_langbar.argtypes=[P,P,READ,TOGGLE];cls.api.meltype_tsf_service_create_langbar.restype=P
        cls.api.meltype_tsf_service_remove_langbar.argtypes=[P,P];cls.api.meltype_tsf_service_remove_langbar.restype=None
        cls.api.meltype_tsf_service_update_langbar.argtypes=[P];cls.api.meltype_tsf_service_update_langbar.restype=None
        cls.api.meltype_tsf_service_create_candidates.argtypes=[P,P,SELECT];cls.api.meltype_tsf_service_create_candidates.restype=P
        cls.api.meltype_tsf_service_show_candidates.argtypes=[P,P,P,U32,c.c_char_p];cls.api.meltype_tsf_service_show_candidates.restype=c.c_int
        for operation in ['hide','drop']:
            getattr(cls.api,'meltype_tsf_service_'+operation+'_candidates').argtypes=[P]
            getattr(cls.api,'meltype_tsf_service_'+operation+'_candidates').restype=None
        cls.api.meltype_tsf_candidate_visible.argtypes=[P];cls.api.meltype_tsf_candidate_visible.restype=c.c_int
        cls.api.meltype_tsf_start_composition.argtypes=[P,P,P,U32];cls.api.meltype_tsf_start_composition.restype=P

        cls.worker=str(Path(sys.argv[1]).resolve().with_name('meltype-tip-worker.exe')).encode()

        size=cls.api.meltype_tsf_service_state_layout(None,0)
        metadata=(U32*size)();cls.api.meltype_tsf_service_state_layout(metadata,size)
        expected=[6,c.sizeof(State),c.alignment(State)]+[getattr(State,name).offset for name,_ in State._fields_]
        if list(metadata)!=expected:raise AssertionError('State ABI mismatch; do not borrow this allocation')
    @classmethod
    def tearDownClass(cls):cls.ole.CoUninitialize()
    def setUp(self):
        self.owned=[];self.callbacks=[];self.before=self.live();self.callback_errors=[]
        self.directory=tempfile.TemporaryDirectory(prefix='meltype-service-state-')
        self.old_data=os.environ.get('MELTYPE_RUST_DATA_DIR')
        os.environ['MELTYPE_RUST_DATA_DIR']=self.directory.name

    def tearDown(self):
        for data in self.owned:self.api.meltype_tsf_service_state_destroy(data)
        self.assertEqual(self.live(),self.before)
        if self.old_data is None:os.environ.pop('MELTYPE_RUST_DATA_DIR',None)
        else:os.environ['MELTYPE_RUST_DATA_DIR']=self.old_data
        deadline=time.monotonic()+3
        while True:
            try:self.directory.cleanup();break
            except PermissionError:
                if time.monotonic()>=deadline:raise
                time.sleep(.02)
        self.assertEqual(self.callback_errors,[])
    def checked(self,kind):
        # ctypes otherwise prints and swallows exceptions from native callbacks.
        def decorate(fn):
            def guarded(*args):
                try:return fn(*args)
                except BaseException as error:
                    self.callback_errors.append((fn.__name__,repr(error)));return 0
            return kind(guarded)
        return decorate
    def live(self):return self.api.meltype_tsf_service_state_live_count()
    def create(self):
        data=self.api.meltype_tsf_service_state_create()
        self.assertTrue(data);self.owned.append(data);return State.from_address(data)
    def test_layout_matches_all_fields_before_borrowing(self):
        size=self.api.meltype_tsf_service_state_layout(None,0)
        result=(U32*size)();self.assertEqual(self.api.meltype_tsf_service_state_layout(result,size),size)
        expected=[6,c.sizeof(State),c.alignment(State)]+[getattr(State,name).offset for name,_ in State._fields_]
        self.assertEqual(list(result),expected)
    def test_short_metadata_buffer_is_untouched(self):
        size=self.api.meltype_tsf_service_state_layout(None,0);result=(U32*(size+2))(*([0xaabbccdd]*(size+2)))
        self.assertEqual(self.api.meltype_tsf_service_state_layout(result,size-1),size)
        self.assertEqual(list(result),[0xaabbccdd]*(size+2))
    def test_defaults_match_upstream_service(self):
        state=self.create()
        for name,kind in State._fields_:
            with self.subTest(field=name):
                expected=1 if name=='refs' else name in ['open','consumed'] if kind==c.c_bool else None if kind==P else 0
                self.assertEqual(getattr(state,name),expected)
    def test_independent_instances_and_64bit_times(self):
        first=self.create();second=self.create();self.assertEqual(self.live(),self.before+2)
        first.server_active=True;first.open=False;first.last_hello=0xfedcba9876543210;first.focus_epoch=0x100000001
        self.assertFalse(second.server_active);self.assertTrue(second.open);self.assertEqual(second.last_hello,0)
        self.assertEqual(first.last_hello,0xfedcba9876543210);self.assertEqual(first.focus_epoch,0x100000001)
    def test_null_destroy_is_inert(self):
        self.api.meltype_tsf_service_state_destroy(None);self.assertEqual(self.live(),self.before)
    def test_independent_ui_threads_balance_storage(self):
        errors=[]
        def work():
            try:
                for _ in range(100):
                    data=self.api.meltype_tsf_service_state_create()
                    if not data:raise AssertionError('allocation failed')
                    state=State.from_address(data);state.open=False;state.mode_epoch=123456789
                    self.api.meltype_tsf_service_state_destroy(data)
            except BaseException as e:errors.append(repr(e))
        threads=[threading.Thread(target=work) for _ in range(4)]
        for thread in threads:thread.start()
        for thread in threads:thread.join()
        self.assertEqual(errors,[]);self.assertEqual(self.live(),self.before)
    def test_sid_copy_self_assignment_replace_and_independence(self):
        first=self.create();second=self.create();address=c.addressof(first)
        self.assertEqual(c.string_at(self.api.meltype_tsf_service_sid(address)),b'')
        source=c.create_string_buffer('日本語-session'.encode())
        self.assertEqual(self.api.meltype_tsf_service_set_sid(address,source),1)
        source[0]=b'X'
        borrowed=self.api.meltype_tsf_service_sid(address)
        self.assertEqual(c.string_at(borrowed),'日本語-session'.encode())
        self.assertEqual(self.api.meltype_tsf_service_set_sid(address,borrowed),1)
        self.assertEqual(c.string_at(self.api.meltype_tsf_service_sid(address)),'日本語-session'.encode())
        self.assertEqual(c.string_at(self.api.meltype_tsf_service_sid(c.addressof(second))),b'')
        self.assertEqual(self.api.meltype_tsf_service_set_sid(address,c.c_char_p(b'new')),1)
        self.assertEqual(c.string_at(self.api.meltype_tsf_service_sid(address)),b'new')
        self.assertEqual(self.api.meltype_tsf_service_set_sid(address,c.c_char_p(b'')),1)
        self.assertEqual(c.string_at(self.api.meltype_tsf_service_sid(address)),b'')
    def request(self,state,value):
        raw=self.api.meltype_tsf_service_request(c.addressof(state),self.worker,json.dumps(value).encode(),5000,1)
        self.assertTrue(raw)
        try:return json.loads(c.string_at(raw))
        finally:self.api.meltype_free(raw)
    def test_transport_owned_by_state_closes_and_reopens(self):
        state=self.create();self.assertFalse(state.transport)
        self.request(state,{'op':'hello','process':'test'})
        old=state.transport;self.assertTrue(old)
        self.request(state,{'op':'hello','process':'test'})
        self.assertEqual(state.transport,old)
        self.api.meltype_tsf_service_close(c.addressof(state));self.assertFalse(state.transport)
        self.assertFalse(self.api.meltype_tip_transport_request(old,b'{}',100,1))
        self.api.meltype_tsf_service_close(c.addressof(state))
        self.request(state,{'op':'hello','process':'test'})
        self.assertTrue(state.transport);self.assertNotEqual(state.transport,old)
    def test_destroy_retires_connection_without_explicit_close(self):
        first=self.create();second=self.create()
        self.request(first,{'op':'hello','process':'first'})
        self.request(second,{'op':'hello','process':'second'})
        old=first.transport;self.assertNotEqual(old,second.transport)
        address=c.addressof(first);self.owned.remove(address)
        self.api.meltype_tsf_service_state_destroy(address)
        self.assertFalse(self.api.meltype_tip_transport_request(old,b'{}',100,1))
        self.request(second,{'op':'hello','process':'second'})
    def test_null_resource_arguments_are_inert(self):
        state=self.create();address=c.addressof(state)
        self.assertEqual(self.api.meltype_tsf_service_set_sid(None,c.c_char_p(b'test')),0)
        self.assertEqual(self.api.meltype_tsf_service_set_sid(address,None),0)
        self.assertFalse(self.api.meltype_tsf_service_sid(None))
        self.api.meltype_tsf_service_close(None)
        for owner,worker,request in [(None,self.worker,b'{}'),(address,None,b'{}'),(address,self.worker,None)]:
            self.assertFalse(self.api.meltype_tsf_service_request(owner,worker,request,10,1))
        self.assertFalse(state.transport)
    def create_bar(self,state,mode=None):
        mode=mode if mode is not None else [False]
        @READ
        def read(owner):return int(mode[0])
        @self.checked(TOGGLE)
        def toggle(owner):mode[0]=not mode[0]
        self.callbacks.extend([read,toggle])
        bar=self.api.meltype_tsf_service_create_langbar(c.addressof(state),123,read,toggle)
        self.assertTrue(bar);return bar,mode
    def bar_text(self,bar):
        text=P();self.assertEqual(invoke(bar,11,HR,[c.POINTER(P)],c.byref(text)),0)
        allocator=c.WinDLL('oleaut32');allocator.SysFreeString.argtypes=[P]
        try:return c.wstring_at(text)
        finally:allocator.SysFreeString(text)
    def click_bar(self,bar):self.assertEqual(invoke(bar,7,HR,[HR,Point,P],0,Point(),None),0)
    def test_bar_replacement_detaches_retained_old_reference(self):
        state=self.create();old,mode=self.create_bar(state)
        invoke(old,1,U32,[]);self.assertEqual(self.bar_text(old),'A')
        current,_=self.create_bar(state,[True]);self.assertEqual(state.langbar,current)
        self.click_bar(old);self.assertFalse(mode[0]);self.assertEqual(self.bar_text(old),'あ')
        self.assertEqual(release(old),0)
        self.api.meltype_tsf_service_remove_langbar(c.addressof(state),None)
        self.assertFalse(state.langbar)
        self.api.meltype_tsf_service_remove_langbar(c.addressof(state),None)
    def test_bar_destruction_detaches_context_from_retained_button(self):
        state=self.create();bar,mode=self.create_bar(state);invoke(bar,1,U32,[])
        address=c.addressof(state);self.owned.remove(address)
        self.api.meltype_tsf_service_state_destroy(address)
        self.click_bar(bar);self.assertFalse(mode[0]);self.assertEqual(self.bar_text(bar),'あ')
        self.assertEqual(release(bar),0)
    def test_bar_manager_removal_preserves_reentrant_new_bar(self):
        state=self.create();old,mode=self.create_bar(state);invoke(old,1,U32,[])
        host=StartupHost();removed=[];replacement=[]
        def remove(_,bar):
            removed.append(bar);self.assertFalse(state.langbar)
            self.click_bar(bar);self.assertFalse(mode[0])
            replacement.append(self.create_bar(state,[True])[0]);return 0
        host.bind(host.bar_manager_table,6,HR,[P],remove)
        self.api.meltype_tsf_service_remove_langbar(c.addressof(state),c.addressof(host.context))
        self.assertEqual(removed,[old]);self.assertEqual(state.langbar,replacement[0])
        self.assertEqual(host.bar_manager_refs,1);self.assertEqual(release(old),0)
    def test_bar_failed_manager_query_releases_partial_reference(self):
        state=self.create();self.create_bar(state)
        host=StartupHost();host.fail_bar_query=host.partial_bar_query=True
        self.api.meltype_tsf_service_remove_langbar(c.addressof(state),c.addressof(host.context))
        self.assertFalse(state.langbar);self.assertEqual(host.bar_manager_refs,1)
    def advise_bar(self,bar,on_update=None,on_drop=None):
        refs=[1];events=[]
        class Sink(c.Structure):_fields_=[('table',c.POINTER(P))]
        @c.WINFUNCTYPE(HR,P,c.POINTER(Guid),c.POINTER(P))
        def query(_,iid,out):refs[0]+=1;out[0]=c.addressof(sink);return 0
        @c.WINFUNCTYPE(U32,P)
        def add(_):refs[0]+=1;return refs[0]
        @c.WINFUNCTYPE(U32,P)
        def drop(_):
            refs[0]-=1
            if on_drop:on_drop(refs[0])
            return refs[0]
        @c.WINFUNCTYPE(HR,P,U32)
        def update(_,flags):
            events.append(flags)
            if on_update:on_update()
            return 0
        table=(P*4)(*[c.cast(fn,P).value for fn in [query,add,drop,update]])
        sink=Sink(table);self.callbacks.extend([query,add,drop,update,table,sink])
        source=P();source_id=Guid.parse('4ea48a35-60ae-446f-8fd6-e6a8d82459f7')
        sink_id=Guid.parse('57dbe1a0-de25-11d2-afdd-00105a2799b5')
        self.assertEqual(invoke(bar,0,HR,[c.POINTER(Guid),c.POINTER(P)],c.byref(source_id),c.byref(source)),0)
        cookie=U32()
        self.assertEqual(invoke(source,3,HR,[c.POINTER(Guid),P,c.POINTER(U32)],c.byref(sink_id),c.byref(sink),c.byref(cookie)),0)
        release(source);return refs,events
    def test_bar_update_survives_service_destruction_in_notification(self):
        state=self.create();bar,_=self.create_bar(state);address=c.addressof(state)
        def destroy():
            self.owned.remove(address);self.api.meltype_tsf_service_state_destroy(address)
        refs,events=self.advise_bar(bar,on_update=destroy)
        self.api.meltype_tsf_service_update_langbar(address)
        self.assertEqual(events,[7]);self.assertEqual(refs,[1]);self.assertEqual(self.live(),self.before)
    def test_bar_create_detects_reentrant_replacement_from_sink_release(self):
        state=self.create();old,_=self.create_bar(state);replacement=[]
        def released(refs):
            if refs==1:replacement.append(self.create_bar(state,[True])[0])
        refs,_=self.advise_bar(old,on_drop=released)
        read=READ(lambda _:0);toggle=TOGGLE(lambda _:None);self.callbacks.extend([read,toggle])
        result=self.api.meltype_tsf_service_create_langbar(c.addressof(state),123,read,toggle)
        self.assertFalse(result);self.assertEqual(len(replacement),1)
        self.assertEqual(state.langbar,replacement[0]);self.assertEqual(refs,[1])
    def test_bar_stopping_and_null_state_are_inert(self):
        state=self.create();state.shutdown_phase=1
        read=READ(lambda _:0);toggle=TOGGLE(lambda _:None)
        self.assertFalse(self.api.meltype_tsf_service_create_langbar(c.addressof(state),None,read,toggle))
        self.assertFalse(self.api.meltype_tsf_service_create_langbar(None,None,read,toggle))
        self.api.meltype_tsf_service_update_langbar(None);self.api.meltype_tsf_service_remove_langbar(None,None)
        self.assertFalse(state.langbar)
    def create_candidates(self,state,on_select=None):
        events=[]
        @SELECT
        def select(owner,index,prediction):
            events.append((owner,index,prediction))
            if on_select:on_select()
        self.callbacks.append(select)
        token=self.api.meltype_tsf_service_create_candidates(c.addressof(state),456,select)
        self.assertTrue(token);self.assertEqual(state.candidates,token);return token,events
    def show_candidates(self,state,context=None,composition=None,view=None):
        view=view or {'predictions':['日本','😀'],'selectedPrediction':1}
        return self.api.meltype_tsf_service_show_candidates(c.addressof(state),context,composition,7,json.dumps(view,ensure_ascii=False).encode())
    def candidate_windows(self):return windows('Meltype-flashCandidateWindow')
    def click_candidate(self,window):
        scale=user.GetDpiForWindow(window)/96
        user.SendMessageW(window,0x202,0,(int(52*scale)<<16)|10)
    def test_candidates_hide_show_drop_and_stale_window(self):
        state=self.create();token,events=self.create_candidates(state)
        self.assertEqual(self.show_candidates(state),1);self.assertTrue(self.api.meltype_tsf_candidate_visible(token))
        window=self.candidate_windows()[0]
        self.api.meltype_tsf_service_hide_candidates(c.addressof(state))
        self.assertEqual(state.candidates,token);self.assertFalse(self.api.meltype_tsf_candidate_visible(token))
        self.assertEqual(self.show_candidates(state),1)
        self.api.meltype_tsf_service_drop_candidates(c.addressof(state))
        self.assertFalse(state.candidates);self.assertFalse(self.api.meltype_tsf_candidate_visible(token))
        user.SendMessageW(window,0x202,0,10|(52<<16));self.assertEqual(events,[])
        self.api.meltype_tsf_service_drop_candidates(c.addressof(state))
        self.assertFalse(self.candidate_windows());self.assertEqual(self.show_candidates(state),0)
    def test_candidates_replacement_retires_old_window(self):
        state=self.create();old,_=self.create_candidates(state);self.assertEqual(self.show_candidates(state),1)
        current,_=self.create_candidates(state);self.assertNotEqual(old,current)
        self.assertFalse(self.api.meltype_tsf_candidate_visible(old));self.assertFalse(self.candidate_windows())
        self.assertEqual(self.show_candidates(state),1)
    def test_candidates_click_can_destroy_own_service(self):
        state=self.create();address=c.addressof(state)
        def destroy():
            self.owned.remove(address);self.api.meltype_tsf_service_state_destroy(address)
        token,events=self.create_candidates(state,destroy);self.assertEqual(self.show_candidates(state),1)
        self.click_candidate(self.candidate_windows()[0])
        self.assertEqual(events,[(456,1,1)]);self.assertFalse(self.candidate_windows())
        self.assertFalse(self.api.meltype_tsf_candidate_visible(token));self.assertEqual(self.live(),self.before)
    def test_candidates_anchor_callback_can_destroy_service(self):
        state=self.create();address=c.addressof(state);token,_=self.create_candidates(state)
        host=GeometryHost()
        composition=self.api.meltype_tsf_start_composition(c.byref(host.context),c.byref(host.sink),host.original,7)
        self.assertTrue(composition)
        def destroy():
            host.on_extent=None;self.owned.remove(address);self.api.meltype_tsf_service_state_destroy(address)
        host.on_extent=destroy
        try:
            result=self.show_candidates(state,c.byref(host.context),composition,{'converting':True,'candidates':['前','文節'],'clauses':['前','😀','文節','後'],'selectedClause':2})
            self.assertEqual(result,0);self.assertEqual(self.live(),self.before)
            self.assertFalse(self.api.meltype_tsf_candidate_visible(token));self.assertFalse(self.candidate_windows())
        finally:release(composition)
        host.verify(self)
    def test_candidates_independent_service_owners(self):
        first=self.create();second=self.create();old,_=self.create_candidates(first);current,_=self.create_candidates(second)
        self.assertNotEqual(old,current);self.assertEqual(self.show_candidates(first),1);self.assertEqual(self.show_candidates(second),1)
        self.assertEqual(len(self.candidate_windows()),2)
        address=c.addressof(first);self.owned.remove(address);self.api.meltype_tsf_service_state_destroy(address)
        self.assertFalse(self.api.meltype_tsf_candidate_visible(old));self.assertTrue(self.api.meltype_tsf_candidate_visible(current))
        self.assertEqual(len(self.candidate_windows()),1)
    def test_candidates_null_and_stopping_state_are_inert(self):
        state=self.create();state.shutdown_phase=1;select=SELECT(lambda *_:None)
        self.assertFalse(self.api.meltype_tsf_service_create_candidates(c.addressof(state),None,select))
        self.assertFalse(self.api.meltype_tsf_service_create_candidates(None,None,select))
        self.assertEqual(self.api.meltype_tsf_service_show_candidates(None,None,None,7,b'{}'),0)
        self.api.meltype_tsf_service_hide_candidates(None);self.api.meltype_tsf_service_drop_candidates(None)
        self.assertFalse(state.candidates)
    def test_initialize_matches_exact_upstream_reset_and_preserves_other_fields(self):
        state=self.create();address=c.addressof(state)
        for name,kind in State._fields_:
            if kind==c.c_bool:setattr(state,name,True)
        state.mode_epoch=0xffffffffffffffff;state.last_hello=1234;state.client=99
        state.focus_epoch=777;state.shutdown_phase=0;state.pass_reason=1234;state.scope_source=5678
        before={name:getattr(state,name) for name,_ in State._fields_}
        self.assertEqual(self.api.meltype_tsf_service_initialize(address,17,0,-1,b'new-session'),1)
        changed={'mode_epoch':0,'client':17,'deactivated':False,'server_active':False,'direct_detection':False,'full_width_space':False,
                 'direct_word':False,'last_hello':0,'caret_moved':False,'own_edit':False,'secret_checked':False,
                 'last_converting':False,'pass_reason':None,'key_failed':False,'consumed':True,'own_process':True,'disabled':False}
        for name,_ in State._fields_:
            if name=='sid':continue
            with self.subTest(field=name):self.assertEqual(getattr(state,name),changed.get(name,before[name]))
        self.assertEqual(c.string_at(self.api.meltype_tsf_service_sid(address)),b'new-session')
        self.assertEqual(self.api.meltype_tsf_service_initialize(address,21,1,0,b'second'),1)
        self.assertEqual((state.client,state.mode_epoch,state.disabled,state.own_process),(21,1,True,False))
    def test_initialize_null_arguments_leave_all_state_unchanged(self):
        state=self.create();address=c.addressof(state);before=c.string_at(address,c.sizeof(State))
        self.assertEqual(self.api.meltype_tsf_service_initialize(address,17,1,1,None),0)
        self.assertEqual(c.string_at(address,c.sizeof(State)),before)
        self.assertEqual(self.api.meltype_tsf_service_initialize(None,17,1,1,b'test'),0)
    def capture_resume(self,host):
        composition=self.api.meltype_tsf_start_composition(c.byref(host.context),c.byref(host.sink),host.original,7)
        self.assertTrue(composition)
        try:token=self.api.meltype_tsf_capture_resume(c.byref(host.context),composition,7)
        finally:release(composition)
        self.assertTrue(token);return token
    def test_clear_resume_detaches_and_releases_owned_com_references(self):
        state=self.create();host=GeometryHost();state.resume=self.capture_resume(host)
        self.assertGreater(host.context_refs,1)
        self.api.meltype_tsf_service_clear_resume(c.addressof(state))
        self.assertFalse(state.resume);host.verify(self)
        self.api.meltype_tsf_service_clear_resume(c.addressof(state));self.api.meltype_tsf_service_clear_resume(None)
    def test_clear_resume_release_can_destroy_service_after_slot_detach(self):
        state=self.create();host=GeometryHost();state.resume=self.capture_resume(host)
        address=c.addressof(state);original=host.context_ref;events=[]
        def reference(delta):
            if delta==-1 and not events:
                events.append(state.resume);self.assertFalse(state.resume)
                self.owned.remove(address);self.api.meltype_tsf_service_state_destroy(address)
            return original(delta)
        host.context_ref=reference
        self.api.meltype_tsf_service_clear_resume(address)
        self.assertEqual(events,[None]);self.assertEqual(self.live(),self.before);host.verify(self)
    def test_destroy_owns_key_resume_and_abandoned_token_cleanup(self):
        state=self.create();address=c.addressof(state)
        keys=self.api.meltype_tsf_key_state_create();self.assertTrue(keys);state.keys=keys
        self.api.meltype_tsf_key_stale_write(keys,8,65,123456789)
        stale=Stale();self.api.meltype_tsf_key_stale_read(keys,c.byref(stale));self.assertEqual(stale.tries,8)
        resume_host=GeometryHost();state.resume=self.capture_resume(resume_host)
        abandoned_host=DeferredHost()
        composition=self.api.meltype_tsf_start_composition(c.byref(abandoned_host.context),c.byref(abandoned_host.sink),abandoned_host.original,7)
        self.assertTrue(composition)
        try:
            state.abandoned=self.api.meltype_tsf_abandoned_create();self.assertTrue(state.abandoned)
            self.assertEqual(self.api.meltype_tsf_abandoned_add(state.abandoned,c.byref(abandoned_host.context),composition,c.byref(abandoned_host.context),17),262912)
        finally:release(composition)
        self.assertGreater(resume_host.context_refs,1);self.assertGreater(abandoned_host.context_refs,1)
        self.owned.remove(address);self.api.meltype_tsf_service_state_destroy(address)
        self.api.meltype_tsf_key_stale_read(keys,c.byref(stale));self.assertEqual((stale.tries,stale.key,stale.time),(0,0,0))
        resume_host.verify(self)
        # A scheduled edit still pins its COM inputs until the host cancels it.
        self.assertGreater(abandoned_host.context_refs,1)
        abandoned_host.cancel();abandoned_host.verify(self)
    def edit_sink(self,state,host,document=True):
        return self.api.meltype_tsf_service_edit_sink(c.addressof(state),c.byref(host.document_manager) if document else None,c.byref(host.context),c.byref(host.sink) if document else None)
    def test_edit_sink_owns_source_context_and_resets_only_nonnull_bind(self):
        state=self.create();host=SubscriptionHost();state.client=17;state.own_edit=True
        try:
            self.assertEqual(self.edit_sink(state,host),0);self.assertTrue(state.edit_subscription)
            self.assertEqual(state.edit_context,c.addressof(host.context));self.assertFalse(state.own_edit)
            state.own_edit=True;self.assertEqual(self.edit_sink(state,host,False),0)
            self.assertFalse(state.edit_subscription);self.assertFalse(state.edit_context);self.assertTrue(state.own_edit)
        finally:self.edit_sink(state,host,False)
        host.verify(self)
    def test_edit_sink_reentrant_unadvise_preserves_new_binding(self):
        state=self.create();first=SubscriptionHost();second=SubscriptionHost();state.client=17
        self.edit_sink(state,first);old=state.edit_subscription
        first.sources[0].on_unadvise=lambda:self.edit_sink(state,second)
        try:
            self.assertEqual(self.edit_sink(state,first),0)
            self.assertNotEqual(state.edit_subscription,old);self.assertEqual(state.edit_context,c.addressof(second.context))
            self.assertFalse(first.sources[0].registrations);self.assertEqual(len(second.sources[0].registrations),1)
        finally:self.edit_sink(state,second,False)
        first.verify(self);second.verify(self)
    def test_edit_sink_reentrant_advise_discards_late_old_registration(self):
        state=self.create();first=SubscriptionHost();second=SubscriptionHost();state.client=17
        original=first.sources[0].advise
        def advise(this,iid,sink,cookie):
            result=original(this,iid,sink,cookie);self.edit_sink(state,second);return result
        first.bind(first.sources[0].table,3,HR,[c.POINTER(Guid),P,c.POINTER(U32)],advise)
        try:
            self.assertEqual(self.edit_sink(state,first),0)
            self.assertEqual(state.edit_context,c.addressof(second.context));self.assertFalse(first.sources[0].registrations)
            self.assertEqual(len(second.sources[0].registrations),1)
        finally:self.edit_sink(state,second,False)
        first.verify(self);second.verify(self)
    def test_edit_sink_focus_epoch_or_shutdown_during_advise_retires_late_token(self):
        for kind in ['focus','shutdown']:
            with self.subTest(kind=kind):
                state=self.create();host=SubscriptionHost();state.client=17;original=host.sources[0].advise
                def advise(this,iid,sink,cookie):
                    result=original(this,iid,sink,cookie)
                    if kind=='focus':state.focus_epoch+=1
                    else:state.shutdown_phase=1;state.deactivated=True
                    return result
                host.bind(host.sources[0].table,3,HR,[c.POINTER(Guid),P,c.POINTER(U32)],advise)
                self.assertEqual(self.edit_sink(state,host),0)
                self.assertFalse(state.edit_subscription);self.assertFalse(state.edit_context);host.verify(self)
    def test_edit_sink_partial_failures_release_all_references(self):
        for kind in ['top','source','advise']:
            with self.subTest(kind=kind):
                state=self.create();host=SubscriptionHost();state.client=17;state.own_edit=True
                if kind=='top':host.fail_top=host.partial_top=True
                elif kind=='source':host.fail_source=host.partial_source=True
                else:host.sources[0].fail=host.sources[0].partial=True
                self.assertEqual(self.edit_sink(state,host),0)
                self.assertFalse(state.edit_subscription);self.assertFalse(state.edit_context);self.assertFalse(state.own_edit)
                host.verify(self)
    def test_edit_sink_reentrant_detach_keeps_slots_empty(self):
        state=self.create();host=SubscriptionHost();state.client=17;state.edit_epoch=0xffffffffffffffff
        self.edit_sink(state,host);self.assertEqual(state.edit_epoch,0)
        def detached():
            self.assertFalse(state.edit_subscription);self.assertFalse(state.edit_context)
            self.edit_sink(state,host,False)
        host.sources[0].on_unadvise=detached
        self.assertEqual(self.edit_sink(state,host,False),0)
        self.assertFalse(state.edit_subscription);self.assertFalse(state.edit_context);host.verify(self)
    def test_edit_sink_invalid_arguments_preserve_existing_binding(self):
        state=self.create();host=SubscriptionHost();state.client=17;self.edit_sink(state,host)
        old=state.edit_subscription
        try:
            for owner,document,lifetime,sink in [(None,None,c.byref(host.context),None),(c.addressof(state),None,None,None),
                (c.addressof(state),c.byref(host.document_manager),c.byref(host.context),None)]:
                self.assertEqual(self.api.meltype_tsf_service_edit_sink(owner,document,lifetime,sink),HR(0x80070057).value)
                self.assertEqual(state.edit_subscription,old)
        finally:self.edit_sink(state,host,False)
        host.verify(self)
    def test_mode_all_state_reply_combinations_preserve_unrelated_fields(self):
        state=self.create();host=SubscriptionHost();address=c.addressof(state);notices=[]
        @NOTICE
        def notice(owner,english):
            self.assertEqual(owner,123);self.assertEqual(state.code_english,bool(english))
            notices.append(english)
        for before in range(16):
            for reply in range(32):
                state.open=bool(before&1);state.code_app=bool(before&2);state.code_english=bool(before&4);state.server_active=bool(before&8)
                self.assertEqual(self.api.meltype_tsf_service_mode(address),before)
                notices.clear();code=reply&3==3;english=code and bool(reply&4)
                self.assertEqual(self.api.meltype_tsf_service_apply_mode(address,reply,c.byref(host.context),123,notice),1)
                expected=(before&9)|(2 if code else 0)|(4 if english else 0)
                self.assertEqual(self.api.meltype_tsf_service_mode(address),expected)
                self.assertEqual(notices,[int(english)] if bool(before&4)!=english else [])
        host.verify(self)
    def test_mode_state_is_published_before_notice_and_bar_update(self):
        state=self.create();host=SubscriptionHost();address=c.addressof(state);events=[]
        bar,_=self.create_bar(state)
        refs,updates=self.advise_bar(bar,on_update=lambda:events.append(('ui',state.code_app,state.code_english)))
        @NOTICE
        def notice(_,english):events.append(('notice',state.code_app,state.code_english))
        self.assertEqual(self.api.meltype_tsf_service_apply_mode(address,7,c.byref(host.context),None,notice),1)
        self.assertEqual(events,[('notice',True,True),('ui',True,True)])
        self.assertEqual(updates,[7]);events.clear()
        self.assertEqual(self.api.meltype_tsf_service_apply_mode(address,7,c.byref(host.context),None,notice),1)
        self.assertEqual(events,[])
        self.assertEqual(self.api.meltype_tsf_service_apply_mode(address,0,c.byref(host.context),None,NOTICE()),1)
        self.assertEqual(events,[('ui',False,False)])
        self.api.meltype_tsf_service_remove_langbar(address,None);self.assertEqual(refs,[1]);host.verify(self)
    def test_mode_dispatch_pin_keeps_service_alive_through_notice_and_ui(self):
        state=self.create();address=c.addressof(state);destroyed=[];root=[None];events=[]
        @DISPATCH
        def dispatch(_owner,_call):return 0
        @self.checked(DESTROY)
        def destroy(_owner):
            self.owned.remove(address);self.api.meltype_tsf_service_state_destroy(address);destroyed.append(True)
        root[0]=self.api.meltype_tsf_service_com_create(None,dispatch,destroy);self.assertTrue(root[0])
        bar,_=self.create_bar(state)
        refs,updates=self.advise_bar(bar,on_update=lambda:events.append(('ui',self.live())))
        @NOTICE
        def notice(_,english):
            old=root[0];root[0]=None;self.assertEqual(release(old),1)
            self.assertEqual(destroyed,[]);events.append(('notice',self.live()))
        try:self.assertEqual(self.api.meltype_tsf_service_apply_mode(address,7,root[0],None,notice),1)
        finally:
            if root[0]:release(root[0])
        self.assertEqual(events,[('notice',self.before+1),('ui',self.before+1)])
        self.assertEqual(destroyed,[True]);self.assertEqual(refs,[1]);self.assertEqual(updates,[7])
    def test_mode_reentrant_notice_preserves_nested_update(self):
        state=self.create();host=SubscriptionHost();address=c.addressof(state);events=[]
        @NOTICE
        def notice(_,english):
            events.append(english)
            if english:self.api.meltype_tsf_service_apply_mode(address,1,c.byref(host.context),None,notice)
        self.assertEqual(self.api.meltype_tsf_service_apply_mode(address,7,c.byref(host.context),None,notice),1)
        self.assertEqual(events,[1,0]);self.assertFalse(state.code_app);self.assertFalse(state.code_english);host.verify(self)
    def test_write_open_uses_service_client_and_survives_state_destruction(self):
        state=self.create();address=c.addressof(state);host=CompartmentHost();state.manager=c.addressof(host.context);state.client=17
        self.assertEqual(self.api.meltype_tsf_service_write_open(address,0),1)
        self.assertEqual(host.writes_mode,[(bytes(OPEN),17,0)])
        def destroy():
            self.owned.remove(address);self.api.meltype_tsf_service_state_destroy(address)
        host.on_write=destroy
        self.assertEqual(self.api.meltype_tsf_service_write_open(address,99),1)
        self.assertEqual(host.writes_mode[-1],(bytes(OPEN),17,1));host.verify(self)
    def test_mode_null_inputs_do_not_mutate_state_or_notify(self):
        state=self.create();address=c.addressof(state);before=c.string_at(address,c.sizeof(State));events=[]
        notice=NOTICE(lambda *_:events.append(1))
        self.assertEqual(self.api.meltype_tsf_service_apply_mode(address,7,None,None,notice),0)
        self.assertEqual(self.api.meltype_tsf_service_apply_mode(None,7,None,None,notice),0)
        self.assertEqual(c.string_at(address,c.sizeof(State)),before);self.assertEqual(events,[])
        self.assertEqual(self.api.meltype_tsf_service_mode(None),0)
        self.assertEqual(self.api.meltype_tsf_service_write_open(None,1),0)
        self.assertEqual(self.api.meltype_tsf_service_write_open(address,1),0)
    def core(self,state,host,explicit=False,clock=None,notice=None,worker=None):
        clock=clock or CORE_CLOCK(lambda _:3000)
        notice=notice or CORE_NOTICE(lambda *_:None)
        return self.api.meltype_tsf_service_core(c.addressof(state),worker or self.worker,c.byref(host.context),123,notice,clock,int(explicit))
    def test_core_readiness_strict_retry_boundary_and_active_short_path(self):
        state=self.create();host=SubscriptionHost();now=[2000];times=[];events=[]
        clock=CORE_CLOCK(lambda owner:times.append(owner) or now[0]);notice=CORE_NOTICE(lambda owner,op,value:events.append((owner,op,value)))
        self.assertEqual(self.core(state,host,clock=clock,notice=notice),0);self.assertFalse(state.transport)
        now[0]=2001
        self.assertEqual(self.core(state,host,clock=clock,notice=notice),1)
        self.assertTrue(state.server_active);self.assertEqual(state.last_hello,2001)
        self.assertEqual(events,[(123,0,1)]);before=len(times)
        self.assertEqual(self.core(state,host,clock=clock,notice=notice),1);self.assertEqual(len(times),before)
        host.verify(self)
    def test_core_disabled_and_ended_service_do_not_read_clock_or_start_worker(self):
        state=self.create();host=SubscriptionHost();calls=[];clock=CORE_CLOCK(lambda _:calls.append(1) or 4000)
        for field in ['disabled','deactivated']:
            setattr(state,field,True)
            self.assertEqual(self.core(state,host,clock=clock),0);self.assertEqual(self.core(state,host,explicit=True,clock=clock),0)
            setattr(state,field,False)
        self.assertEqual(calls,[]);self.assertFalse(state.transport);host.verify(self)
    def fixture_core(self,state,host,reply,notice=None):
        if not self.fixture:self.skipTest('Pass the private framed transport fixture executable')
        root=Path(self.directory.name);response=root/'hello-reply.json';request=root/'hello-request.json'
        response.write_text(reply,encoding='utf-8')
        changes={'MELTYPE_TEST_HELLO_REPLY_FILE':str(response),'MELTYPE_TEST_HELLO_REQUEST_FILE':str(request)}
        previous={name:os.environ.get(name) for name in changes}
        try:
            os.environ.update(changes)
            self.api.meltype_tsf_service_close(c.addressof(state))
            # Warm only the private fixture, then require a fresh action record.
            # Native core policy/deadlines are exercised unchanged below.
            response=self.api.meltype_tsf_service_request(c.addressof(state),self.fixture,b'{}',5000,0)
            self.assertTrue(response,'private core fixture readiness');self.api.meltype_free(response)
            request.unlink()
            result=self.core(state,host,explicit=True,notice=notice,worker=self.fixture)
        finally:
            for name,value in previous.items():
                if value is None:os.environ.pop(name,None)
                else:os.environ[name]=value
        return result,json.loads(request.read_text(encoding='utf-8'))
    def test_core_hello_native_process_thread_metadata_and_state_publication(self):
        state=self.create();host=SubscriptionHost();state.own_process=True;state.direct_word=True;events=[]
        notice=CORE_NOTICE(lambda owner,op,value:events.append((op,value,state.server_active,state.direct_detection,state.code_app,state.direct_word)))
        result,request=self.fixture_core(state,host,'{"active":true,"directDetection":true,"code":true,"fullWidthSpace":true}',notice)
        self.assertEqual(result,1);self.assertEqual(request['op'],'hello')
        kernel=c.WinDLL('kernel32');self.assertEqual(request['tid'],kernel.GetCurrentThreadId())
        self.assertEqual(request['process'].lower(),'python');self.assertEqual(state.last_hello,3000)
        self.assertEqual(events,[(0,1,True,True,True,False)]);host.verify(self)
        self.assertTrue(state.full_width_space)
    def test_core_inactive_and_non_boolean_fields_are_normalized(self):
        state=self.create();host=SubscriptionHost()
        for reply in ['{"active":false,"code":true,"directDetection":true}','{"active":1,"code":true}',
                      '{"active":true,"code":"true","directDetection":1}']:
            state.direct_word=True
            result,_=self.fixture_core(state,host,reply)
            expected=json.loads(reply);active=expected.get('active') is True
            self.assertEqual(result,int(active));self.assertEqual(state.server_active,active)
            self.assertEqual(state.code_app,active and expected.get('code') is True)
            self.assertEqual(state.direct_detection,active and expected.get('directDetection') is True);self.assertFalse(state.direct_word)
        host.verify(self)
    def test_core_invalid_json_reports_error_then_inactive(self):
        state=self.create();host=SubscriptionHost();events=[]
        notice=CORE_NOTICE(lambda owner,op,value:events.append((op,value)))
        result,_=self.fixture_core(state,host,'not-json',notice)
        self.assertEqual(result,0);self.assertEqual(events,[(1,0),(0,0)])
        self.assertFalse(state.server_active);host.verify(self)

    def test_issue_881_setting_reaches_service_snapshot_and_is_cleared_when_inactive(self):
        state=self.create();host=SubscriptionHost()
        for active,enabled in [(True,True),(True,False),(False,True)]:
            self.fixture_core(state,host,json.dumps({'active':active,'fullWidthSpace':enabled}))
            self.assertEqual(state.full_width_space,active and enabled)
        host.verify(self)
    def test_core_reentrant_notice_supersedes_return_and_keeps_new_state(self):
        state=self.create();host=SubscriptionHost()
        def changed(owner,op,value):
            state.focus_epoch+=1;state.server_active=False;state.code_app=False
        notice=CORE_NOTICE(changed)
        result,_=self.fixture_core(state,host,'{"active":true,"code":true}',notice)
        # Notification runs after publication; return value remains the hello result,
        # while the nested generation/state change is preserved.
        self.assertEqual(result,1);self.assertFalse(state.server_active);self.assertFalse(state.code_app);host.verify(self)
    def test_core_null_arguments_do_not_mutate_state_or_start_transport(self):
        state=self.create();address=c.addressof(state);before=c.string_at(address,c.sizeof(State))
        for data,worker,lifetime in [(None,self.worker,None),(address,None,None),(address,self.worker,None)]:
            self.assertEqual(self.api.meltype_tsf_service_core(data,worker,lifetime,None,CORE_NOTICE(),CORE_CLOCK(),0),0)
        self.assertEqual(c.string_at(address,c.sizeof(State)),before)
    def exchange(self,state,host,text=b'{"op":"hello"}',reply=None,notice=None,worker=None,timeout=5000,optional=False,lifetime=None):
        reply=reply or REPLY(lambda *_:1);notice=notice or TOGGLE(lambda *_:None)
        return self.api.meltype_tsf_service_exchange(c.addressof(state),worker or self.worker,lifetime or c.byref(host.context),text,len(text),timeout,int(optional),123,reply,notice)
    def test_exchange_native_transport_unicode_and_thread_metadata(self):
        state=self.create();host=SubscriptionHost();state.own_process=True
        if not self.fixture:self.skipTest('Pass framed transport fixture')
        root=Path(self.directory.name);response=root/'exchange-reply.json';request=root/'exchange-request.json'
        response.write_text('{"view":{"text":"試験😀"}}',encoding='utf-8')
        changes={'MELTYPE_TEST_HELLO_REPLY_FILE':str(response),'MELTYPE_TEST_HELLO_REQUEST_FILE':str(request)}
        previous={key:os.environ.get(key) for key in changes};events=[]
        @self.checked(REPLY)
        def parsed(owner,data,length):
            events.append((owner,json.loads(c.string_at(data,length))));return 1
        try:
            os.environ.update(changes)
            self.assertEqual(self.exchange(state,host,b'{"op":"probe","text":"Unicode"}',parsed,worker=self.fixture),1)
        finally:
            for key,value in previous.items():
                if value is None:os.environ.pop(key,None)
                else:os.environ[key]=value
        captured=json.loads(request.read_text(encoding='utf-8'))
        self.assertEqual(captured['tid'],c.WinDLL('kernel32').GetCurrentThreadId())
        self.assertEqual(captured['text'],'Unicode');self.assertEqual(events,[(123,{'view':{'text':'試験😀'}})])
        host.verify(self)
    def test_exchange_parse_failure_notifies_after_reply(self):
        state=self.create();host=SubscriptionHost();events=[]
        reply=REPLY(lambda owner,data,length:events.append(('reply',owner)) or 0)
        notice=TOGGLE(lambda owner:events.append(('notice',owner)))
        self.assertEqual(self.exchange(state,host,reply=reply,notice=notice),0)
        self.assertEqual(events,[('reply',123),('notice',123)]);host.verify(self)
    def test_exchange_reentrant_epoch_change_discards_result_and_notice(self):
        state=self.create();host=SubscriptionHost();events=[]
        @self.checked(REPLY)
        def parsed(owner,data,length):
            state.focus_epoch+=1;state.server_active=True;return 0
        self.assertEqual(self.exchange(state,host,reply=parsed,notice=TOGGLE(lambda _:events.append(1))),0)
        self.assertTrue(state.server_active);self.assertEqual(events,[]);host.verify(self)
    def test_exchange_reentrant_shutdown_discards_success(self):
        state=self.create();host=SubscriptionHost()
        @self.checked(REPLY)
        def parsed(owner,data,length):state.deactivated=True;return 1
        self.assertEqual(self.exchange(state,host,reply=parsed),0);host.verify(self)
    def test_exchange_nested_request_keeps_outer_reply_bytes_alive(self):
        state=self.create();host=SubscriptionHost();events=[]
        @self.checked(REPLY)
        def inner(owner,data,length):
            events.append(json.loads(c.string_at(data,length))['active']);return 1
        @self.checked(REPLY)
        def outer(owner,data,length):
            before=c.string_at(data,length)
            self.assertEqual(self.exchange(state,host,b'{"op":"reset","sid":"nested"}',reply=inner),1)
            self.assertEqual(c.string_at(data,length),before)
            events.append(json.loads(before)['active']);return 1
        self.assertEqual(self.exchange(state,host,reply=outer),1)
        self.assertEqual(events,[False,True]);host.verify(self)
    def test_exchange_ended_and_invalid_inputs_do_not_start_transport(self):
        state=self.create();host=SubscriptionHost();events=[];reply=REPLY(lambda *_:events.append(1) or 1)
        state.deactivated=True;self.assertEqual(self.exchange(state,host,reply=reply),0);state.deactivated=False
        address=c.addressof(state);unknown=c.byref(host.context)
        for data,worker,lifetime,raw,count,callback in [(None,self.worker,unknown,b'{}',2,reply),
            (address,None,unknown,b'{}',2,reply),(address,self.worker,None,b'{}',2,reply),
            (address,self.worker,unknown,None,1,reply),(address,self.worker,unknown,b'{}',2,REPLY())]:
            self.assertEqual(self.api.meltype_tsf_service_exchange(data,worker,lifetime,raw,count,5000,0,None,callback,TOGGLE()),0)
        self.assertEqual(self.exchange(state,host,text=b'{}\0garbage',reply=reply),0)
        self.assertFalse(state.transport);self.assertEqual(events,[]);host.verify(self)
    def test_exchange_zero_timeout_skips_reply_and_optional_reconnects(self):
        state=self.create();host=SubscriptionHost();events=[]
        reply=REPLY(lambda *_:events.append(1) or 1)
        self.assertEqual(self.exchange(state,host,reply=reply,timeout=0,optional=True),0)
        self.assertEqual(events,[])
        self.assertEqual(self.exchange(state,host,reply=reply,optional=True),1)
        self.assertEqual(events,[1]);host.verify(self)
    def test_exchange_native_lifetime_pins_service_during_reply(self):
        state=self.create();address=c.addressof(state);host=SubscriptionHost();events=[]
        dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):
            events.append('destroy');self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed)
        self.assertTrue(root);self.owned.remove(address)
        @self.checked(REPLY)
        def parsed(owner,data,length):
            events.append('reply');release(root)
            self.assertEqual(self.live(),self.before+1);return 1
        try:
            self.assertEqual(self.exchange(state,host,reply=parsed,lifetime=root),1)
            self.assertEqual(events,['reply','destroy']);self.assertEqual(self.live(),self.before)
        finally:
            if 'reply' not in events:release(root)
    def focus(self,state,host,event=-1,document=None,commit=None,resume=None,notice=None,lifetime=None):
        return self.api.meltype_tsf_service_focus(c.addressof(state),self.worker,lifetime or c.byref(host.context),c.byref(host.sink),document,event,123,
            commit or FOCUS_COMMIT(lambda *_:1),resume or TOGGLE(lambda *_:None),notice or CORE_NOTICE(lambda *_:None))
    def test_focus_native_flags_reset_and_hello_publication(self):
        state=self.create();host=SubscriptionHost();events=[];state.direct_word=True
        resume=TOGGLE(lambda owner:events.append(('resume',owner,state.direct_word,state.caret_moved)))
        notice=CORE_NOTICE(lambda owner,op,value:events.append(('notice',owner,op,value,state.direct_word,state.caret_moved)))
        self.assertEqual(self.focus(state,host,resume=resume,notice=notice),0)
        self.assertEqual(events,[('resume',123,True,False),('notice',123,0,1,False,True)])
        self.assertTrue(state.server_active);self.assertEqual(state.focus_epoch,1);host.verify(self)
    def test_focus_native_edit_binding_and_disabled_hello_skip(self):
        state=self.create();host=SubscriptionHost();state.disabled=True;events=[]
        self.assertEqual(self.focus(state,host,document=c.byref(host.document_manager),notice=CORE_NOTICE(lambda *_:events.append(1))),0)
        self.assertTrue(state.edit_subscription);self.assertEqual(state.edit_context,c.addressof(host.context))
        self.assertEqual(events,[]);self.assertFalse(state.transport)
        self.edit_sink(state,host,False);host.verify(self)
    def test_focus_native_sync_failure_queues_async_before_resume(self):
        state=self.create();host=SubscriptionHost();state.disabled=True;state.composition=123;state.direct_word=True;events=[]
        @self.checked(FOCUS_COMMIT)
        def commit(owner,mode):events.append(('commit',owner,mode,state.direct_word));return 0
        @self.checked(TOGGLE)
        def resume(owner):events.append(('resume',owner,state.direct_word));state.composition=None
        self.assertEqual(self.focus(state,host,commit=commit,resume=resume),0)
        self.assertEqual(events,[('commit',123,0,True),('commit',123,1,True),('resume',123,True)])
        self.assertFalse(state.direct_word);self.assertTrue(state.caret_moved);host.verify(self)
    def test_focus_native_nested_focus_keeps_new_state(self):
        state=self.create();host=SubscriptionHost();state.disabled=True;events=[]
        @self.checked(TOGGLE)
        def resume(owner):
            events.append('outer')
            self.assertEqual(self.focus(state,host,resume=TOGGLE(lambda _:events.append('inner'))),0)
            state.direct_word=True;state.caret_moved=False
        self.assertEqual(self.focus(state,host,resume=resume),0)
        self.assertEqual(events,['outer','inner']);self.assertEqual(state.focus_epoch,2)
        self.assertTrue(state.direct_word);self.assertFalse(state.caret_moved);host.verify(self)
    def test_focus_native_stopping_document_is_inert(self):
        state=self.create();host=SubscriptionHost();events=[]
        for field,value in [('deactivated',True),('shutdown_phase',1)]:
            setattr(state,field,value)
            self.assertEqual(self.focus(state,host,resume=TOGGLE(lambda _:events.append(1))),0)
            setattr(state,field,0)
        self.assertEqual(events,[]);self.assertEqual(state.focus_epoch,0);self.assertFalse(state.transport);host.verify(self)
    def test_focus_native_foreground_preserves_document_state(self):
        state=self.create();host=SubscriptionHost();state.direct_word=True;events=[]
        for event in [1,0]:self.assertEqual(self.focus(state,host,event=event,resume=TOGGLE(lambda _:events.append(1))),0)
        self.assertEqual(events,[]);self.assertTrue(state.direct_word);self.assertFalse(state.caret_moved)
        self.assertEqual(state.focus_epoch,0);self.assertFalse(state.transport);host.verify(self)
    def test_focus_native_lifetime_pins_state_after_resume_releases_caller(self):
        state=self.create();address=c.addressof(state);host=SubscriptionHost();state.disabled=True;events=[]
        dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):events.append('destroy');self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        @self.checked(TOGGLE)
        def resume(owner):
            events.append('resume');release(root);self.assertEqual(self.live(),self.before+1)
        try:
            self.assertEqual(self.focus(state,host,lifetime=root,resume=resume),0)
            self.assertEqual(events,['resume','destroy']);self.assertEqual(self.live(),self.before)
        finally:
            if 'resume' not in events:release(root)
    def test_focus_native_invalid_arguments_preserve_state(self):
        state=self.create();host=SubscriptionHost();address=c.addressof(state);before=c.string_at(address,c.sizeof(State))
        for data,worker,lifetime,sink,document,event,commit,resume in [
            (None,self.worker,c.byref(host.context),None,None,-1,FOCUS_COMMIT(lambda *_:1),TOGGLE(lambda *_:None)),
            (address,None,c.byref(host.context),None,None,-1,FOCUS_COMMIT(lambda *_:1),TOGGLE(lambda *_:None)),
            (address,self.worker,None,None,None,-1,FOCUS_COMMIT(lambda *_:1),TOGGLE(lambda *_:None)),
            (address,self.worker,c.byref(host.context),None,c.byref(host.document_manager),-1,FOCUS_COMMIT(lambda *_:1),TOGGLE(lambda *_:None)),
            (address,self.worker,c.byref(host.context),None,None,2,FOCUS_COMMIT(lambda *_:1),TOGGLE(lambda *_:None)),
            (address,self.worker,c.byref(host.context),None,None,-1,FOCUS_COMMIT(),TOGGLE(lambda *_:None)),
            (address,self.worker,c.byref(host.context),None,None,-1,FOCUS_COMMIT(lambda *_:1),TOGGLE())]:
            self.assertEqual(self.api.meltype_tsf_service_focus(data,worker,lifetime,sink,document,event,None,commit,resume,CORE_NOTICE()),HR(0x80070057).value)
        self.assertEqual(c.string_at(address,c.sizeof(State)),before);host.verify(self)
    def edited(self,state,host,query=None,commit=None,record=True,lifetime=None):
        return self.api.meltype_tsf_service_edited(c.addressof(state),lifetime or c.byref(host.context),c.byref(host.context),
            c.byref(host.record) if record else None,7,123,query or QUERY(lambda *_:None),commit or TOGGLE(lambda *_:None))
    def event_composition(self,state,host):
        pointer=self.api.meltype_tsf_start_composition(c.byref(host.context),c.byref(host.sink),host.original,7)
        self.assertTrue(pointer);state.composition=pointer;state.composition_context=c.addressof(host.context)
        self.addCleanup(host.verify,self);self.addCleanup(release,pointer);return pointer
    def test_edited_native_null_record_and_own_edit_consumption(self):
        state=self.create();host=EventHost();state.own_edit=True;events=[]
        query=QUERY(lambda *_:events.append('query'))
        self.assertEqual(self.edited(state,host,query=query,record=False),1);self.assertTrue(state.own_edit)
        self.assertEqual(self.edited(state,host,query=query),1);self.assertFalse(state.own_edit)
        self.assertEqual(events,[]);self.assertEqual(host.record_reads,0);host.verify(self)
    def test_edited_native_mode_query_precedes_moved_state(self):
        state=self.create();host=EventHost();events=[]
        @self.checked(QUERY)
        def query(owner,cookie,context,changed):
            self.assertEqual((owner,cookie,context,changed),(123,7,c.addressof(host.context),1))
            events.append(state.caret_moved)
        self.assertEqual(self.edited(state,host,query=query),1)
        self.assertEqual(events,[False]);self.assertTrue(state.caret_moved);host.verify(self)
    def test_edited_native_unchanged_or_failed_status_preserves_flags(self):
        state=self.create();host=EventHost();state.direct_word=True;events=[]
        query=QUERY(lambda owner,cookie,context,changed:events.append(changed))
        host.changed=False;self.assertEqual(self.edited(state,host,query=query),1)
        host.changed=True;host.fail_status=True;self.assertEqual(self.edited(state,host,query=query),1)
        self.assertEqual(events,[0]);self.assertTrue(state.direct_word);self.assertFalse(state.caret_moved);host.verify(self)
    def test_edited_native_outside_composition_marks_before_commit(self):
        state=self.create();host=EventHost();pointer=self.event_composition(state,host);events=[]
        state.direct_word=True;host.selection=(0,0)
        @self.checked(TOGGLE)
        def commit(owner):events.append((owner,state.caret_moved,state.direct_word))
        try:
            self.assertEqual(self.edited(state,host,commit=commit),1)
            self.assertEqual(events,[(123,True,False)]);self.assertEqual(state.composition,pointer)
        finally:state.composition=None;state.composition_context=None
    def test_edited_native_boundary_selection_keeps_composition(self):
        state=self.create();host=EventHost();self.event_composition(state,host);events=[]
        try:
            for selection in [(2,2),(3,3),(5,5),(2,5)]:
                host.selection=selection;self.assertEqual(self.edited(state,host,commit=TOGGLE(lambda _:events.append(1))),1)
            self.assertEqual(events,[]);self.assertFalse(state.caret_moved)
        finally:state.composition=None;state.composition_context=None
    def test_edited_native_reentrant_range_replacement_suppresses_commit(self):
        state=self.create();host=EventHost();self.event_composition(state,host);events=[];host.selection=(0,0)
        host.on_range=lambda:setattr(state,'composition',None)
        try:
            self.assertEqual(self.edited(state,host,commit=TOGGLE(lambda _:events.append(1))),1)
            self.assertEqual(events,[]);self.assertFalse(state.caret_moved)
        finally:state.composition=None;state.composition_context=None
    def test_edited_native_query_can_release_last_caller_reference(self):
        state=self.create();address=c.addressof(state);host=EventHost();events=[]
        dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):
            events.append(('destroy',state.caret_moved));self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        @self.checked(QUERY)
        def query(*args):
            events.append(('query',state.caret_moved));release(root);self.assertEqual(self.live(),self.before+1)
        try:
            self.assertEqual(self.edited(state,host,query=query,lifetime=root),1)
            self.assertEqual(events,[('query',False),('destroy',True)]);host.verify(self)
        finally:
            if not events:release(root)
    def test_edited_native_invalid_arguments_leave_state_unchanged(self):
        state=self.create();host=EventHost();address=c.addressof(state);state.own_edit=True;before=c.string_at(address,c.sizeof(State))
        for data,lifetime,query,commit in [(None,c.byref(host.context),QUERY(lambda *_:None),TOGGLE(lambda *_:None)),
            (address,None,QUERY(lambda *_:None),TOGGLE(lambda *_:None)),(address,c.byref(host.context),QUERY(),TOGGLE(lambda *_:None)),
            (address,c.byref(host.context),QUERY(lambda *_:None),TOGGLE())]:
            self.assertEqual(self.api.meltype_tsf_service_edited(data,lifetime,c.byref(host.context),c.byref(host.record),7,None,query,commit),0)
        self.assertEqual(c.string_at(address,c.sizeof(State)),before);host.verify(self)
    def opened(self,state,host,commit=None,notice=None,guid=OPEN,lifetime=None):
        return self.api.meltype_tsf_service_opened(c.addressof(state),lifetime or c.byref(host.context),c.byref(guid),123,
            commit or TOGGLE(lambda *_:None),notice or NOTICE(lambda *_:None))
    def test_opened_native_publishes_strict_mode_and_clears_direct_word(self):
        state=self.create();host=OpenHost();state.manager=c.addressof(host.context);events=[]
        notice=NOTICE(lambda owner,mode:events.append((owner,mode,state.open,state.direct_word)))
        try:
            for value in [-1,0,1]:
                state.direct_word=True;host.integers[bytes(OPEN)]=value
                self.assertEqual(self.opened(state,host,notice=notice),0)
                self.assertEqual(events[-1],(123,int(value!=0),value!=0,False))
            self.assertEqual(state.mode_epoch,3);host.verify(self)
        finally:state.manager=None
    def test_opened_native_close_orders_notice_commit_and_bar_update(self):
        state=self.create();host=OpenHost();state.manager=c.addressof(host.context);state.composition=123;state.direct_word=True
        host.integers[bytes(OPEN)]=0;events=[];bar,_=self.create_bar(state)
        refs,updates=self.advise_bar(bar,on_update=lambda:events.append(('ui',state.open,state.direct_word)))
        notice=NOTICE(lambda owner,mode:events.append(('notice',state.open,state.direct_word)))
        commit=TOGGLE(lambda _:events.append(('commit',state.open,state.direct_word)))
        try:
            self.assertEqual(self.opened(state,host,commit,notice),0)
            self.assertEqual(events,[('notice',False,False),('commit',False,False),('ui',False,False)])
            self.assertEqual(updates,[7]);host.verify(self)
        finally:state.manager=None;state.composition=None
    def test_opened_native_failed_read_preserves_flags_but_commits_closed_input(self):
        state=self.create();host=OpenHost();state.manager=c.addressof(host.context);state.composition=123
        state.open=False;state.direct_word=True;host.fail_read=True;events=[]
        try:
            self.assertEqual(self.opened(state,host,TOGGLE(lambda _:events.append('commit')),NOTICE(lambda owner,mode:events.append(('notice',mode)))),0)
            self.assertEqual(events,[('notice',0),'commit']);self.assertTrue(state.direct_word);host.verify(self)
        finally:state.manager=None;state.composition=None
    def test_opened_native_unrelated_guid_preserves_entire_state(self):
        state=self.create();host=OpenHost();state.manager=c.addressof(host.context);before=c.string_at(c.addressof(state),c.sizeof(State))
        try:
            self.assertEqual(self.opened(state,host,guid=Guid.parse('00000000-0000-0000-0000-000000000000')),0)
            self.assertEqual(c.string_at(c.addressof(state),c.sizeof(State)),before);self.assertEqual(host.reads,[]);host.verify(self)
        finally:state.manager=None
    def test_opened_native_shutdown_and_ended_services_do_not_read_mode(self):
        state=self.create();host=OpenHost();state.manager=c.addressof(host.context);events=[]
        try:
            for field,value in [('shutdown_phase',1),('deactivated',True)]:
                setattr(state,field,value)
                self.assertEqual(self.opened(state,host,notice=NOTICE(lambda *_:events.append(1))),0)
                setattr(state,field,0)
            self.assertEqual(events,[]);self.assertEqual(host.reads,[]);host.verify(self)
        finally:state.manager=None
    def test_opened_native_nested_notice_preserves_new_mode_and_skips_old_commit(self):
        state=self.create();host=OpenHost();state.manager=c.addressof(host.context);state.composition=123
        host.integers[bytes(OPEN)]=0;events=[];bar,_=self.create_bar(state)
        refs,updates=self.advise_bar(bar,on_update=lambda:events.append(('ui',state.open)))
        @self.checked(NOTICE)
        def notice(owner,mode):
            events.append(('notice',mode));host.integers[bytes(OPEN)]=1
            self.assertEqual(self.opened(state,host,notice=NOTICE(lambda owner,mode:events.append(('nested',mode)))),0)
        try:
            self.assertEqual(self.opened(state,host,TOGGLE(lambda _:events.append('old commit')),notice),0)
            self.assertEqual(events,[('notice',0),('nested',1),('ui',True)])
            self.assertTrue(state.open);self.assertEqual(state.mode_epoch,2);host.verify(self)
        finally:state.manager=None;state.composition=None
    def test_opened_native_manager_replacement_during_read_discards_reply(self):
        state=self.create();host=OpenHost();other=OpenHost();state.manager=c.addressof(host.context);events=[]
        host.integers[bytes(OPEN)]=0;host.on_read=lambda:setattr(state,'manager',c.addressof(other.context))
        state.direct_word=True
        try:
            self.assertEqual(self.opened(state,host,notice=NOTICE(lambda *_:events.append(1))),0)
            self.assertTrue(state.open);self.assertTrue(state.direct_word);self.assertEqual(events,[])
            host.verify(self);other.verify(self)
        finally:state.manager=None
    def test_opened_native_lifetime_pins_state_after_notice_releases_caller(self):
        state=self.create();address=c.addressof(state);host=OpenHost();events=[]
        state.manager=c.addressof(host.context);state.composition=123;host.integers[bytes(OPEN)]=0
        dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):
            events.append(('destroy',state.open));state.manager=None;state.composition=None
            self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        @self.checked(NOTICE)
        def notice(owner,mode):
            events.append(('notice',mode));release(root);self.assertEqual(self.live(),self.before+1)
        try:
            self.assertEqual(self.opened(state,host,TOGGLE(lambda _:events.append('commit')),notice,lifetime=root),0)
            self.assertEqual(events,[('notice',0),'commit',('destroy',False)]);host.verify(self)
        finally:
            if not events:release(root)
    def test_opened_native_invalid_arguments_preserve_state(self):
        state=self.create();host=OpenHost();address=c.addressof(state);before=c.string_at(address,c.sizeof(State))
        for data,lifetime,guid,commit in [(None,c.byref(host.context),c.byref(OPEN),TOGGLE(lambda *_:None)),
            (address,None,c.byref(OPEN),TOGGLE(lambda *_:None)),(address,c.byref(host.context),None,TOGGLE(lambda *_:None)),
            (address,c.byref(host.context),c.byref(OPEN),TOGGLE())]:
            self.assertEqual(self.api.meltype_tsf_service_opened(data,lifetime,guid,None,commit,NOTICE()),HR(0x80070057).value)
        self.assertEqual(c.string_at(address,c.sizeof(State)),before);host.verify(self)
    def deactivate(self,state,host,request=None,end=None,notice=None,lifetime=None):
        lifetime=lifetime or c.byref(host.context);request=request or SHUTDOWN_REQUEST(lambda *_:None);notice=notice or NOTICE(lambda *_:None)
        if end is not None:return self.api.meltype_tsf_service_deactivate(c.addressof(state),lifetime,123,request,end,notice)
        return self.api.meltype_tsf_service_stop(c.addressof(state),lifetime,123,request,notice)
    def test_deactivate_native_close_metadata_precedes_stopped_state(self):
        state=self.create();host=SubscriptionHost();state.client=17;events=[]
        sid='終了"😀';self.assertEqual(self.api.meltype_tsf_service_set_sid(c.addressof(state),sid.encode()),1)
        @self.checked(SHUTDOWN_REQUEST)
        def request(owner,text,cookie,context,target):
            events.append((owner,json.loads(text),cookie,context,target,state.shutdown_phase,state.deactivated,state.client))
        self.assertEqual(self.deactivate(state,host,request=request),0)
        self.assertEqual(events,[(123,{'op':'close','sid':sid},0,None,None,1,False,17)])
        self.assertEqual((state.shutdown_phase,state.deactivated,state.client,state.focus_epoch),(2,True,0,1));host.verify(self)
    def test_deactivate_native_disabled_skips_close_and_repeated_call_is_inert(self):
        state=self.create();host=SubscriptionHost();state.disabled=True;events=[]
        request=SHUTDOWN_REQUEST(lambda *_:events.append(1))
        self.assertEqual(self.deactivate(state,host,request=request),0);self.assertEqual(self.deactivate(state,host,request=request),0)
        self.assertEqual(events,[]);self.assertEqual(state.focus_epoch,1);self.assertEqual(state.shutdown_phase,2);host.verify(self)
    def test_deactivate_native_owns_manager_edit_binding_ui_resume_and_transport_cleanup(self):
        state=self.create();host=SubscriptionHost();resume_host=GeometryHost();state.client=17
        state.manager=c.addressof(host.context);host.context_ref(1);self.edit_sink(state,host)
        state.resume=self.capture_resume(resume_host);self.create_bar(state);self.create_candidates(state)
        response=self.api.meltype_tsf_service_request(c.addressof(state),self.worker,b'{"op":"hello"}',5000,0)
        self.assertTrue(response);self.api.meltype_free(response)
        self.assertEqual(self.deactivate(state,host),0)
        for field in ['manager','edit_subscription','edit_context','resume','langbar','candidates','transport']:
            self.assertFalse(getattr(state,field),field)
        host.verify(self);resume_host.verify(self)
    def test_deactivate_native_nested_close_returns_without_duplicate_request(self):
        state=self.create();host=SubscriptionHost();events=[]
        @self.checked(SHUTDOWN_REQUEST)
        def request(*args):
            events.append('close');self.assertEqual(self.deactivate(state,host,request=SHUTDOWN_REQUEST(lambda *_:events.append('nested'))),0)
        self.assertEqual(self.deactivate(state,host,request=request),0)
        self.assertEqual(events,['close']);self.assertEqual(state.shutdown_phase,2);host.verify(self)
    def test_deactivate_native_reactivation_supersedes_old_cleanup(self):
        state=self.create();host=SubscriptionHost();bar,_=self.create_bar(state)
        @self.checked(SHUTDOWN_REQUEST)
        def request(*args):
            state.focus_epoch+=1;state.shutdown_phase=0
            self.assertEqual(self.api.meltype_tsf_service_initialize(c.addressof(state),23,0,0,b'new'),1)
        self.assertEqual(self.deactivate(state,host,request=request),0)
        self.assertEqual((state.shutdown_phase,state.deactivated,state.client,state.langbar),(0,False,23,bar));host.verify(self)
    def shutdown_composition(self,state,host):
        state.client=17
        state.composition=self.api.meltype_tsf_start_composition(c.byref(host.context),c.byref(host.sink),host.original,7)
        self.assertTrue(state.composition);state.composition_context=c.addressof(host.context);host.context_ref(1)
    def cleanup_shutdown_composition(self,state,host):
        host.cancel()
        if state.composition:release(state.composition);state.composition=None
        if state.composition_context:release(state.composition_context);state.composition_context=None
        host.verify(self)
    def test_deactivate_native_sync_composition_commit_end_then_close(self):
        state=self.create();host=DeferredHost();host.mode='sync';self.shutdown_composition(state,host);target=state.composition;events=[]
        @self.checked(SHUTDOWN_REQUEST)
        def request(owner,text,cookie,context,pointer):
            events.append((json.loads(text)['op'],cookie,context,pointer))
        @self.checked(END)
        def end(owner,cookie):
            self.assertEqual((owner,cookie),(123,7));events.append('end')
            self.api.meltype_tsf_end_composition(state.composition_context,state.composition,cookie)
        try:
            self.assertEqual(self.deactivate(state,host,request,end),0)
            self.assertEqual(events,[('commit',7,c.addressof(host.context),target),'end',('close',0,None,None)])
            self.assertFalse(state.composition);self.assertFalse(state.composition_context)
        finally:self.cleanup_shutdown_composition(state,host)
    def test_deactivate_native_late_edit_session_never_uses_stack_adapter(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host);events=[]
        try:
            self.assertEqual(self.deactivate(state,host,SHUTDOWN_REQUEST(lambda owner,text,*_:events.append(json.loads(text)['op']))),0)
            self.assertEqual(events,['close']);self.assertTrue(host.pending)
            pending,host.pending=host.pending,[]
            for session in pending:
                try:self.assertEqual(invoke(session,3,HR,[U32],7),HR(0x80004004).value)
                finally:release(session)
            self.assertEqual(events,['close'])
        finally:self.cleanup_shutdown_composition(state,host)
    def test_deactivate_native_refused_edit_releases_composition_slots(self):
        state=self.create();host=DeferredHost();host.mode='reject';self.shutdown_composition(state,host);events=[]
        try:
            self.assertEqual(self.deactivate(state,host,SHUTDOWN_REQUEST(lambda owner,text,*_:events.append(json.loads(text)['op']))),0)
            self.assertEqual(events,['close']);self.assertFalse(state.composition);self.assertFalse(state.composition_context)
        finally:self.cleanup_shutdown_composition(state,host)
    def test_deactivate_native_abandoned_failure_reports_notice_and_continues(self):
        state=self.create();host=SubscriptionHost();old=DeferredHost();events=[]
        state.client=17;state.abandoned=self.api.meltype_tsf_abandoned_create()
        pointer=self.api.meltype_tsf_start_composition(c.byref(old.context),c.byref(old.sink),old.original,7)
        self.assertTrue(pointer)
        try:self.assertGreaterEqual(self.api.meltype_tsf_abandoned_add(state.abandoned,c.byref(old.context),pointer,c.byref(host.context),17),0)
        finally:release(pointer)
        old.mode='reject'
        self.assertEqual(self.deactivate(state,host,notice=NOTICE(lambda owner,status:events.append((owner,status)))),0)
        self.assertEqual(len(events),1);self.assertEqual(events[0][0],123);self.assertLess(events[0][1],0)
        self.assertEqual(state.shutdown_phase,2);old.cancel();old.verify(self);host.verify(self)
    def test_deactivate_native_last_caller_release_during_close_pins_cleanup(self):
        state=self.create();address=c.addressof(state);host=SubscriptionHost();events=[];self.create_bar(state)
        dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):
            events.append(('destroy',state.shutdown_phase,state.deactivated,state.langbar));self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        @self.checked(SHUTDOWN_REQUEST)
        def request(*args):events.append('close');release(root);self.assertEqual(self.live(),self.before+1)
        try:
            self.assertEqual(self.deactivate(state,host,request=request,lifetime=root),0)
            self.assertEqual(events,['close',('destroy',2,True,None)]);host.verify(self)
        finally:
            if not events:release(root)
    def test_deactivate_native_invalid_arguments_preserve_state(self):
        state=self.create();host=SubscriptionHost();address=c.addressof(state);before=c.string_at(address,c.sizeof(State))
        for data,lifetime,request,end in [(None,c.byref(host.context),SHUTDOWN_REQUEST(lambda *_:None),END(lambda *_:None)),
            (address,None,SHUTDOWN_REQUEST(lambda *_:None),END(lambda *_:None)),(address,c.byref(host.context),SHUTDOWN_REQUEST(),END(lambda *_:None)),
            (address,c.byref(host.context),SHUTDOWN_REQUEST(lambda *_:None),END())]:
            self.assertEqual(self.api.meltype_tsf_service_deactivate(data,lifetime,None,request,end,NOTICE()),HR(0x80070057).value)
        self.assertEqual(c.string_at(address,c.sizeof(State)),before);host.verify(self)
    def activate(self,state,host,flags=0,notice=None,toggle=None,lifetime=None):
        sinks=(P*4)(*[c.addressof(host.sink)]*4)
        select=SELECT(lambda *_:None);read=READ(lambda _:int(state.open and not state.code_english))
        toggle=toggle or TOGGLE(lambda *_:None);notice=notice or CORE_NOTICE(lambda *_:None)
        @self.checked(CATEGORY)
        def category(owner):
            self.assertEqual(owner,123);host.count('category_refs',1);return c.addressof(host.category)
        self.callbacks.extend([select,read,toggle,notice,category])
        return self.api.meltype_tsf_service_activate(c.addressof(state),lifetime or c.byref(host.context),self.worker,c.byref(host.context),17,flags,sinks,123,select,read,toggle,notice,category)
    def finish_activation(self,state,host):
        try:self.assertEqual(self.deactivate(state,host),0)
        finally:host.clear_bars()
        host.verify(self)
    def test_activate_native_initializes_state_slots_ui_and_hello(self):
        state=self.create();host=StartupHost();events=[]
        try:
            self.assertEqual(self.activate(state,host,notice=CORE_NOTICE(lambda owner,op,value:events.append((owner,op,value)))),0)
            self.assertEqual(events,[(123,0,0),(123,1,1)])
            self.assertEqual((state.client,state.open,state.deactivated,state.shutdown_phase),(17,True,False,0))
            self.assertTrue(state.server_active);self.assertEqual(state.manager,c.addressof(host.context))
            for field in ['keys','abandoned','thread_subscription','key_subscription','mode_subscription','edit_subscription','candidates','langbar']:
                self.assertTrue(getattr(state,field),field)
            self.assertEqual((state.atom_input,state.atom_converted,state.atom_target),(700,701,702))
            self.assertRegex(c.string_at(self.api.meltype_tsf_service_sid(c.addressof(state))).decode(),r'^\d+-\d+-\d+$')
        finally:self.finish_activation(state,host)
    def test_activate_native_secure_mode_skips_bar_mode_subscription_and_hello(self):
        state=self.create();host=StartupHost();events=[]
        try:
            self.assertEqual(self.activate(state,host,flags=2,notice=CORE_NOTICE(lambda owner,op,value:events.append((op,value)))),0)
            self.assertEqual(events,[(0,1)]);self.assertTrue(state.disabled);self.assertFalse(state.open)
            self.assertTrue(state.candidates);self.assertFalse(state.langbar);self.assertFalse(state.mode_subscription)
            self.assertTrue(state.thread_subscription);self.assertTrue(state.key_subscription);self.assertFalse(state.transport)
        finally:self.finish_activation(state,host)
    def test_activate_native_persistent_bar_callbacks_work_after_return(self):
        state=self.create();host=StartupHost();events=[]
        @self.checked(TOGGLE)
        def toggle(owner):self.assertEqual(owner,123);events.append('toggle')
        try:
            self.assertEqual(self.activate(state,host,toggle=toggle),0)
            self.assertEqual(self.bar_text(state.langbar),'あ');self.click_bar(state.langbar)
            state.open=False;self.assertEqual(self.bar_text(state.langbar),'A');self.assertEqual(events,['toggle'])
        finally:self.finish_activation(state,host)
    def test_activate_native_reactivates_same_service_after_shutdown(self):
        state=self.create();host=StartupHost()
        try:
            self.assertEqual(self.activate(state,host,flags=2),0);self.assertEqual(self.deactivate(state,host),0)
            state.key_failed=True;state.caret_moved=True;state.last_hello=123
            self.assertEqual(self.activate(state,host),0)
            self.assertFalse(state.deactivated);self.assertFalse(state.disabled);self.assertFalse(state.key_failed);self.assertFalse(state.caret_moved)
            self.assertTrue(state.consumed);self.assertTrue(state.open);self.assertTrue(state.langbar)
            self.assertEqual(state.focus_epoch,3);self.assertEqual(state.shutdown_phase,0)
        finally:self.finish_activation(state,host)
    def test_activate_native_nested_shutdown_from_initialize_stops_late_setup(self):
        state=self.create();host=StartupHost();events=[]
        @self.checked(CORE_NOTICE)
        def notice(owner,op,value):
            events.append((op,value));self.assertEqual(self.deactivate(state,host),0)
        try:
            self.assertEqual(self.activate(state,host,notice=notice),0)
            self.assertEqual(events,[(0,0)]);self.assertEqual(state.shutdown_phase,2);self.assertTrue(state.deactivated)
            for field in ['manager','keys','thread_subscription','key_subscription','mode_subscription','edit_subscription','candidates','langbar']:
                self.assertFalse(getattr(state,field),field)
        finally:self.finish_activation(state,host)
    def test_activate_native_partial_focus_and_category_failures_release_refs(self):
        state=self.create();host=StartupHost();host.fail_focus=host.partial_focus=True;host.fail_register=1
        try:
            self.assertEqual(self.activate(state,host),0)
            self.assertFalse(state.edit_subscription);self.assertTrue(state.server_active)
            self.assertEqual((state.atom_input,state.atom_converted,state.atom_target),(700,0,702))
            self.assertEqual(host.document_refs,1)
        finally:self.finish_activation(state,host)
    def test_activate_native_last_caller_release_during_initialize_keeps_owner_alive(self):
        state=self.create();address=c.addressof(state);host=StartupHost();events=[]
        dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):
            events.append(('destroy',state.disabled,state.candidates is not None))
            self.finish_activation(state,host);self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        @self.checked(CORE_NOTICE)
        def notice(owner,op,value):
            self.assertEqual(op,0);events.append('initialize');release(root);self.assertEqual(self.live(),self.before+1)
        try:
            self.assertEqual(self.activate(state,host,flags=2,notice=notice,lifetime=root),0)
            self.assertEqual(events,['initialize',('destroy',True,True)]);host.verify(self)
        finally:
            if not events:release(root)
    def test_activate_native_invalid_arguments_preserve_state(self):
        state=self.create();host=StartupHost();address=c.addressof(state);before=c.string_at(address,c.sizeof(State))
        sink=c.addressof(host.sink);valid=(P*4)(sink,sink,sink,sink);missing=(P*4)(sink,sink,sink,None)
        for data,lifetime,worker,manager,sinks,select,read,toggle in [
            (None,c.byref(host.context),self.worker,c.byref(host.context),valid,SELECT(lambda *_:None),READ(lambda _:1),TOGGLE(lambda _:None)),
            (address,None,self.worker,c.byref(host.context),valid,SELECT(lambda *_:None),READ(lambda _:1),TOGGLE(lambda _:None)),
            (address,c.byref(host.context),None,c.byref(host.context),valid,SELECT(lambda *_:None),READ(lambda _:1),TOGGLE(lambda _:None)),
            (address,c.byref(host.context),self.worker,None,valid,SELECT(lambda *_:None),READ(lambda _:1),TOGGLE(lambda _:None)),
            (address,c.byref(host.context),self.worker,c.byref(host.context),None,SELECT(lambda *_:None),READ(lambda _:1),TOGGLE(lambda _:None)),
            (address,c.byref(host.context),self.worker,c.byref(host.context),missing,SELECT(lambda *_:None),READ(lambda _:1),TOGGLE(lambda _:None)),
            (address,c.byref(host.context),self.worker,c.byref(host.context),valid,SELECT(),READ(lambda _:1),TOGGLE(lambda _:None)),
            (address,c.byref(host.context),self.worker,c.byref(host.context),valid,SELECT(lambda *_:None),READ(),TOGGLE(lambda _:None)),
            (address,c.byref(host.context),self.worker,c.byref(host.context),valid,SELECT(lambda *_:None),READ(lambda _:1),TOGGLE())]:
            self.assertEqual(self.api.meltype_tsf_service_activate(data,lifetime,worker,manager,17,0,sinks,None,select,read,toggle,CORE_NOTICE(),CATEGORY()),HR(0x80070057).value)
        self.assertEqual(c.string_at(address,c.sizeof(State)),before);host.verify(self)
    def owned_apply(self,state,host,view=None,lifetime=None):
        return self.api.meltype_tsf_service_apply_composition(c.addressof(state),lifetime or c.byref(host.context),c.byref(host.sink),c.byref(host.context),7,
            json.dumps(view or {'text':'かな'},ensure_ascii=False).encode())
    def owned_commit(self,state,host,text='確定😀'):
        value=units(text);buffer=(c.c_uint16*len(value))(*value)
        return self.api.meltype_tsf_service_write_commit(c.addressof(state),c.byref(host.context),c.byref(host.sink),c.byref(host.context),host.original,7,buffer,len(value))
    def owned_end(self,state,host,lifetime=None):
        return self.api.meltype_tsf_service_end_composition(c.addressof(state),lifetime or c.byref(host.context),7)
    def clear_owned_pair(self,state):
        composition,context=state.composition,state.composition_context
        state.composition=state.composition_context=None
        if composition:release(composition)
        if context:release(context)
    def stale_state(self,state):
        if not state.keys:state.keys=self.api.meltype_tsf_key_state_create()
        self.assertTrue(state.keys);self.api.meltype_tsf_key_stale_write(state.keys,8,65,1234)
    def stale_tries(self,state):
        value=Stale();self.api.meltype_tsf_key_stale_read(state.keys,c.byref(value));return value.tries
    def test_owned_native_start_uses_service_attributes_and_utf16_view(self):
        state=self.create();host=OwnerHost();host.selection=(0,3)
        state.atom_input=11;state.atom_converted=22;state.atom_target=33
        try:
            self.assertEqual(self.owned_apply(state,host,{'text':'日本😀','converting':True,'clauses':['日','本😀'],'selectedClause':1}),1)
            self.assertEqual(state.composition,c.addressof(host.composition));self.assertEqual(state.composition_context,c.addressof(host.context))
            self.assertEqual(host.attributes,[(0,1,22),(1,4,33)]);self.assertEqual(host.document,units('日本😀'))
            self.assertEqual(self.owned_apply(state,host),1);self.assertEqual(len(host.started),1)
        finally:self.clear_owned_pair(state);host.verify(self)
    def test_owned_native_refused_start_preserves_empty_slots(self):
        state=self.create();host=OwnerHost();host.fail_start=True
        self.assertEqual(self.owned_apply(state,host),0);self.assertFalse(state.composition);self.assertFalse(state.composition_context);host.verify(self)
    def test_owned_native_commit_uses_keep_caret_and_clears_stale_after_end(self):
        for keep in [False,True]:
            state=self.create();host=OwnerHost();host.selection=(1,1);state.keep_caret=keep;self.stale_state(state)
            try:
                self.assertEqual(self.owned_commit(state,host),3)
                self.assertEqual(host.document,units('確定😀'));self.assertEqual(host.selection,(1,1) if keep else (4,4))
                self.assertEqual(self.stale_tries(state),0);self.assertFalse(state.composition);self.assertFalse(state.composition_context)
            finally:self.clear_owned_pair(state);host.verify(self)
    def test_owned_native_write_failure_and_start_failure_have_distinct_cleanup(self):
        for failed,result,stale in [('fail_start',1,8),('fail_write',2,0)]:
            state=self.create();host=OwnerHost();self.stale_state(state);setattr(host,failed,True)
            try:
                self.assertEqual(self.owned_commit(state,host),result);self.assertEqual(self.stale_tries(state),stale)
                self.assertEqual(host.document,units('old') if failed=='fail_write' else units('確定😀'))
            finally:self.clear_owned_pair(state);host.verify(self)
    def test_owned_native_end_clears_pair_and_stale_once(self):
        state=self.create();host=OwnerHost();self.stale_state(state)
        try:
            self.assertEqual(self.owned_apply(state,host),1);self.assertEqual(self.owned_end(state,host),1)
            self.assertEqual(self.stale_tries(state),0);self.assertEqual(self.owned_end(state,host),0);self.assertEqual(host.ended,1)
        finally:self.clear_owned_pair(state);host.verify(self)
    def test_owned_native_end_preserves_reentrant_composition_replacement(self):
        state=self.create();host=OwnerHost();other=OwnerHost()
        def replaced():
            target=self.api.meltype_tsf_start_composition(c.byref(other.context),c.byref(other.sink),other.original,7)
            self.assertTrue(target);self.clear_owned_pair(state)
            state.composition=target;state.composition_context=c.addressof(other.context);other.context_ref(1)
        try:
            self.assertEqual(self.owned_apply(state,host),1);host.on_end=replaced
            self.assertEqual(self.owned_end(state,host),1)
            self.assertEqual(state.composition,c.addressof(other.composition));self.assertEqual(state.composition_context,c.addressof(other.context))
        finally:self.clear_owned_pair(state);host.verify(self);other.verify(self)
    def test_owned_native_outer_pin_survives_last_release_through_stale_cleanup(self):
        state=self.create();address=c.addressof(state);host=OwnerHost();events=[];self.stale_state(state)
        self.assertEqual(self.owned_apply(state,host),1)
        dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):
            events.append(('destroy',self.stale_tries(state),state.composition,state.composition_context))
            self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        def ended():events.append('end');release(root);self.assertEqual(self.live(),self.before+1)
        host.on_end=ended
        try:
            self.assertEqual(self.owned_end(state,host,lifetime=root),1)
            self.assertEqual(events,['end',('destroy',0,None,None)]);host.verify(self)
        finally:
            if not events:host.on_end=None;self.clear_owned_pair(state);release(root)
    def test_owned_native_invalid_arguments_preserve_slots(self):
        state=self.create();host=OwnerHost();address=c.addressof(state);life=c.byref(host.context);sink=c.byref(host.sink);context=c.byref(host.context)
        before=c.string_at(address,c.sizeof(State))
        for data,lifetime in [(None,life),(address,None)]:
            self.assertEqual(self.api.meltype_tsf_service_apply_composition(data,lifetime,sink,context,7,b'{}'),-1)
            self.assertEqual(self.api.meltype_tsf_service_write_commit(data,lifetime,sink,context,host.original,7,None,0),-1)
            self.assertEqual(self.api.meltype_tsf_service_end_composition(data,lifetime,7),-1)
        self.assertEqual(self.api.meltype_tsf_service_apply_composition(address,life,sink,context,7,None),-1)
        self.assertEqual(self.api.meltype_tsf_service_write_commit(address,life,sink,context,host.original,7,None,1),-1)
        self.assertEqual(c.string_at(address,c.sizeof(State)),before);host.verify(self)
    def test_stop_native_ends_composition_without_operation_adapter(self):
        state=self.create();host=DeferredHost();host.mode='sync';self.shutdown_composition(state,host);self.stale_state(state);events=[]
        try:
            self.assertEqual(self.deactivate(state,host,request=SHUTDOWN_REQUEST(lambda owner,text,*_:events.append(json.loads(text)['op']))),0)
            self.assertEqual(events,['commit','close']);self.assertEqual(host.ended,1);self.assertEqual(self.stale_tries(state),0)
            self.assertFalse(state.composition);self.assertFalse(state.composition_context);self.assertEqual(state.shutdown_phase,2)
        finally:self.cleanup_shutdown_composition(state,host)
    def test_stop_native_invalid_arguments_preserve_state(self):
        state=self.create();host=SubscriptionHost();address=c.addressof(state);before=c.string_at(address,c.sizeof(State))
        for data,lifetime,request in [(None,c.byref(host.context),SHUTDOWN_REQUEST(lambda *_:None)),
            (address,None,SHUTDOWN_REQUEST(lambda *_:None)),(address,c.byref(host.context),SHUTDOWN_REQUEST())]:
            self.assertEqual(self.api.meltype_tsf_service_stop(data,lifetime,None,request,NOTICE()),HR(0x80070057).value)
        self.assertEqual(c.string_at(address,c.sizeof(State)),before);host.verify(self)
    def terminated(self,state,host,target=None,worker=None,notice=None,lifetime=None):
        return self.api.meltype_tsf_service_terminated(c.addressof(state),lifetime or c.byref(host.context),worker or self.worker,7,
            target or state.composition,123,notice or TOGGLE(lambda *_:None))
    def recovered(self,state,host,worker=None,notice=None,context='same'):
        pointer=c.byref(host.context) if context=='same' else context
        return self.api.meltype_tsf_service_resume(c.addressof(state),c.byref(host.context),worker or self.worker,c.byref(host.sink),pointer,123,notice or TOGGLE(lambda *_:None))
    def fixture_recovery(self,state,reply,action):
        if not self.fixture:self.skipTest('Pass framed transport fixture')
        root=Path(self.directory.name);response=root/'recovery-reply.json';request=root/'recovery-request.json'
        response.write_text(reply,encoding='utf-8');changes={'MELTYPE_TEST_HELLO_REPLY_FILE':str(response),'MELTYPE_TEST_HELLO_REQUEST_FILE':str(request)}
        previous={key:os.environ.get(key) for key in changes}
        try:
            os.environ.update(changes);self.api.meltype_tsf_service_close(c.addressof(state))
            # This suite checks service policy, not cold-process scheduling. Warm
            # the private fixture before the product's 800/100ms action. Transport
            # deadline/backoff tests retain their own cold/fault scenarios.
            response=self.api.meltype_tsf_service_request(c.addressof(state),self.fixture,b'{}',5000,0)
            self.assertTrue(response,'private fixture readiness');self.api.meltype_free(response)
            request.unlink() # A warm-up record must never satisfy action assertions.
            result=action(self.fixture)
        finally:
            for key,value in previous.items():
                if value is None:os.environ.pop(key,None)
                else:os.environ[key]=value
        return result,json.loads(request.read_text(encoding='utf-8'))
    def cleanup_recovered(self,state,host):
        host.cancel();self.clear_owned_pair(state);self.api.meltype_tsf_service_clear_resume(c.addressof(state))
        if state.edit_context:release(state.edit_context);state.edit_context=None
        host.verify(self)
    def test_terminated_native_captures_resume_and_clears_stale_without_writing(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host);self.stale_state(state)
        try:
            self.assertEqual(self.terminated(state,host),0)
            self.assertTrue(state.resume);self.assertFalse(state.composition);self.assertFalse(state.composition_context)
            self.assertEqual(self.stale_tries(state),0);self.assertFalse(state.last_converting);self.assertFalse(state.transport)
            self.assertFalse(state.caret_moved);self.assertEqual(host.document,units('old'));self.assertEqual(host.writes,[])
        finally:self.cleanup_recovered(state,host)
    def test_terminated_native_disabled_and_deactivated_flags_prevent_snapshot(self):
        for flag in ['disabled','deactivated']:
            state=self.create();host=DeferredHost();self.shutdown_composition(state,host);setattr(state,flag,True);state.direct_word=True
            try:
                self.assertEqual(self.terminated(state,host),0);self.assertFalse(state.resume);self.assertFalse(state.composition)
                self.assertFalse(state.direct_word);self.assertTrue(state.caret_moved);self.assertFalse(state.transport)
                self.assertEqual(host.writes,[])
            finally:self.cleanup_recovered(state,host)
    def test_terminated_native_unicode_metadata_discards_commit_reply(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host);state.last_converting=True;state.direct_word=True
        sid='確定"😀';self.api.meltype_tsf_service_set_sid(c.addressof(state),sid.encode())
        try:
            result,request=self.fixture_recovery(state,'{"commits":[{"text":"MUST NOT INSERT"}],"active":true}',lambda worker:self.terminated(state,host,worker=worker))
            self.assertEqual(result,0);self.assertEqual(request,{'op':'commit','sid':sid,'moved':True})
            self.assertFalse(state.last_converting);self.assertFalse(state.resume);self.assertFalse(state.direct_word);self.assertTrue(state.caret_moved)
            self.assertEqual(host.document,units('old'));self.assertEqual(host.writes,[])
        finally:self.cleanup_recovered(state,host)
    def test_terminated_native_old_target_preserves_current_input(self):
        state=self.create();host=DeferredHost();other=DeferredHost();self.shutdown_composition(state,host);target=state.composition
        pointer=self.api.meltype_tsf_start_composition(c.byref(other.context),c.byref(other.sink),other.original,7);self.assertTrue(pointer)
        try:
            self.assertEqual(self.terminated(state,host,target=pointer),0);self.assertEqual(state.composition,target)
            self.assertFalse(state.resume);self.assertFalse(state.caret_moved);self.assertFalse(state.transport)
        finally:release(pointer);other.verify(self);self.cleanup_recovered(state,host)
    def test_terminated_native_parse_failure_notice_pins_last_caller_reference(self):
        if not self.fixture:self.skipTest('Pass framed transport fixture')
        state=self.create();address=c.addressof(state);host=DeferredHost();self.shutdown_composition(state,host);state.last_converting=True;events=[]
        dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):
            events.append(('destroy',state.composition,state.resume));self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        @self.checked(TOGGLE)
        def notice(owner):
            self.assertEqual(owner,123);events.append('invalid');release(root);self.assertEqual(self.live(),self.before+1)
        try:
            result,_=self.fixture_recovery(state,'invalid JSON',lambda worker:self.terminated(state,host,worker=worker,notice=notice,lifetime=root))
            self.assertEqual(result,0);self.assertEqual(events,['invalid',('destroy',None,None)]);host.verify(self)
        finally:
            if not events:self.cleanup_recovered(state,host);release(root)
    def test_recovered_native_compatible_json_validation_accepts_unpaired_surrogate(self):
        state=self.create();host=DeferredHost();state.client=17;state.resume=self.capture_resume(host);events=[]
        try:
            result,request=self.fixture_recovery(state,'{"text":"\\uD800"}',lambda worker:self.recovered(state,host,worker=worker,context=None,notice=TOGGLE(lambda _:events.append('invalid'))))
            self.assertEqual(result,2);self.assertEqual(request['op'],'commit');self.assertEqual(events,[])
            self.assertFalse(state.resume);self.assertEqual(host.writes,[])
        finally:self.cleanup_recovered(state,host)
    def test_recovered_native_resumes_without_reinserting_and_publishes_own_edit(self):
        state=self.create();host=DeferredHost();host.mode='sync';self.shutdown_composition(state,host);state.atom_input=11
        state.edit_context=c.addressof(host.context);host.context_ref(1);events=[]
        def style(this,cookie,range,value):
            events.append(state.own_edit);return host.set_value(this,cookie,range,value)
        host.bind(host.property_table,9,HR,[U32,P,P],style)
        try:
            self.assertEqual(self.terminated(state,host),0);self.assertTrue(state.resume)
            self.assertEqual(self.recovered(state,host),1)
            self.assertFalse(state.resume);self.assertTrue(state.composition);self.assertFalse(state.own_edit)
            self.assertEqual(events,[True]);self.assertEqual(host.attributes,[(0,3,11)])
            self.assertEqual(host.document,units('old'));self.assertEqual(host.writes,[]);self.assertFalse(state.transport)
        finally:self.cleanup_recovered(state,host)
    def test_recovered_native_no_snapshot_does_not_contact_worker_or_change_state(self):
        state=self.create();host=DeferredHost();before=c.string_at(c.addressof(state),c.sizeof(State))
        self.assertEqual(self.recovered(state,host),0);self.assertEqual(c.string_at(c.addressof(state),c.sizeof(State)),before);host.verify(self)
    def test_recovered_native_changed_text_commits_without_reinsertion(self):
        state=self.create();host=DeferredHost();host.mode='sync';state.resume=self.capture_resume(host);state.client=17;state.direct_word=True
        host.document=units('new')
        try:
            result,request=self.fixture_recovery(state,'{"commits":[{"text":"duplicate"}]}',lambda worker:self.recovered(state,host,worker=worker))
            self.assertEqual(result,2);self.assertEqual(request['op'],'commit');self.assertTrue(request['moved'])
            self.assertFalse(state.resume);self.assertFalse(state.composition);self.assertFalse(state.direct_word);self.assertTrue(state.caret_moved)
            self.assertEqual(host.document,units('new'));self.assertEqual(host.writes,[])
        finally:self.cleanup_recovered(state,host)
    def test_recovered_native_rejected_and_delayed_edits_release_snapshot(self):
        for mode in ['reject','async']:
            state=self.create();host=DeferredHost();state.resume=self.capture_resume(host);state.client=17;state.own_edit=True;host.mode=mode
            try:
                self.assertEqual(self.recovered(state,host),2);self.assertFalse(state.resume);self.assertFalse(state.own_edit);self.assertFalse(state.composition)
                if host.pending:self.assertEqual(invoke(host.pending[0],3,HR,[U32],7),HR(0x80004004).value)
                self.assertEqual(host.writes,[])
            finally:self.cleanup_recovered(state,host)
    def test_recovered_native_retermination_during_style_preserves_new_snapshot(self):
        state=self.create();host=DeferredHost();host.mode='sync';self.shutdown_composition(state,host);state.atom_input=11
        try:
            self.assertEqual(self.terminated(state,host),0)
            def style(this,cookie,range,value):
                result=host.set_value(this,cookie,range,value)
                self.assertEqual(self.terminated(state,host),0);return result
            host.bind(host.property_table,9,HR,[U32,P,P],style)
            self.assertEqual(self.recovered(state,host),1);self.assertFalse(state.composition);self.assertTrue(state.resume)
            self.assertEqual(host.writes,[]);self.assertFalse(state.transport)
        finally:self.cleanup_recovered(state,host)
    def test_recovered_native_invalid_arguments_preserve_state(self):
        state=self.create();host=DeferredHost();address=c.addressof(state);life=c.byref(host.context);before=c.string_at(address,c.sizeof(State))
        for data,lifetime,worker in [(None,life,self.worker),(address,None,self.worker),(address,life,None)]:
            self.assertEqual(self.api.meltype_tsf_service_terminated(data,lifetime,worker,7,None,None,TOGGLE()),HR(0x80070057).value)
            self.assertEqual(self.api.meltype_tsf_service_resume(data,lifetime,worker,c.byref(host.sink),c.byref(host.context),None,TOGGLE()),-1)
        self.assertEqual(c.string_at(address,c.sizeof(State)),before);host.verify(self)
    def abandoned(self,state,host,worker=None,lifetime=None):
        return self.api.meltype_tsf_service_abandon(c.addressof(state),lifetime or c.byref(host.context),worker or self.worker,123,TOGGLE())
    def cleanup_native_abandoned(self,state,host):
        self.api.meltype_tsf_abandoned_reset(state.abandoned);host.cancel();self.clear_owned_pair(state);host.verify(self)
    def test_abandoned_native_detaches_old_pair_and_late_cleanup_does_not_insert(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host);self.stale_state(state);state.direct_word=True
        try:
            result,request=self.fixture_recovery(state,'{"commits":[{"text":"duplicate"}]}',lambda worker:self.abandoned(state,host,worker=worker))
            self.assertEqual(result,1);self.assertEqual(request['op'],'commit');self.assertTrue(request['moved'])
            self.assertFalse(state.composition);self.assertFalse(state.composition_context);self.assertTrue(state.abandoned)
            self.assertEqual(self.stale_tries(state),0);self.assertFalse(state.direct_word);self.assertTrue(state.caret_moved)
            self.assertTrue(host.pending);host.flush();self.assertEqual(host.ended,1)
            self.assertEqual(host.document,units('old'));self.assertEqual(host.writes,[])
        finally:self.cleanup_native_abandoned(state,host)
    def test_abandoned_native_reentrant_new_input_preserves_bookkeeping(self):
        state=self.create();host=DeferredHost();other=DeferredHost();self.shutdown_composition(state,host);self.stale_state(state);state.direct_word=True
        def queue(this,client,session,flags,out):
            self.assertFalse(state.composition);self.assertFalse(state.composition_context)
            result=host.request(this,client,session,flags,out)
            pointer=self.api.meltype_tsf_start_composition(c.byref(other.context),c.byref(other.sink),other.original,7)
            self.assertTrue(pointer);state.composition=pointer;state.composition_context=c.addressof(other.context);other.context_ref(1)
            return result
        host.bind(host.context_table,3,HR,[U32,P,U32,c.POINTER(HR)],queue)
        try:
            self.assertEqual(self.abandoned(state,host),0)
            self.assertEqual(state.composition,c.addressof(other.composition));self.assertEqual(self.stale_tries(state),8)
            self.assertTrue(state.direct_word);self.assertFalse(state.caret_moved);self.assertFalse(state.transport)
        finally:self.cleanup_native_abandoned(state,host);other.verify(self)
    def test_abandoned_native_pending_cleanup_keeps_service_alive_after_return(self):
        state=self.create();address=c.addressof(state);host=DeferredHost();self.shutdown_composition(state,host);events=[]
        dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):events.append('destroy');self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        def queue(this,client,session,flags,out):
            result=host.request(this,client,session,flags,out)
            events.append('queued');release(root);self.assertEqual(self.live(),self.before+1);return result
        host.bind(host.context_table,3,HR,[U32,P,U32,c.POINTER(HR)],queue)
        try:
            self.assertEqual(self.abandoned(state,host,lifetime=root),1)
            self.assertEqual(events,['queued']);self.assertEqual(self.live(),self.before+1)
            host.cancel();self.assertEqual(events,['queued','destroy']);self.assertEqual(self.live(),self.before);host.verify(self)
        finally:
            if not events:self.cleanup_native_abandoned(state,host);release(root)
            else:host.cancel()
    def test_abandoned_native_invalid_arguments_preserve_owned_state(self):
        state=self.create();host=DeferredHost();address=c.addressof(state);before=c.string_at(address,c.sizeof(State))
        for data,lifetime,worker in [(None,c.byref(host.context),self.worker),(address,None,self.worker),(address,c.byref(host.context),None)]:
            self.assertEqual(self.api.meltype_tsf_service_abandon(data,lifetime,worker,None,TOGGLE()),-1)
        self.assertEqual(c.string_at(address,c.sizeof(State)),before);host.verify(self)
    def test_issue_992_halfwidth_and_email_scopes_pass_through_service(self):
        for scope in [4,5,40]:
            state=self.create();host=CompartmentHost()
            host.values[0].values=[scope]
            self.assertEqual(self.scope(state,host),1)
            self.assertFalse(state.private_field)
            host.values[0].values=[61]
            self.assertEqual(self.scope(state,host),0)
            self.assertTrue(state.private_field)
            host.verify(self)

    def scope(self,state,host,notice=None,lifetime=None):
        return self.api.meltype_tsf_service_input_scope(c.addressof(state),lifetime or c.byref(host.context),c.byref(host.context),7,123,notice or CORE_NOTICE(lambda *_:None))
    def mode_action(self,state,host,op=1,value=1,worker=None,notice=None,context='same'):
        pointer=c.byref(host.context) if context=='same' else context
        return self.api.meltype_tsf_service_mode_action(c.addressof(state),c.byref(host.context),worker or self.worker,pointer,7,op,value,123,notice or CORE_NOTICE(lambda *_:None))
    def fixture_mode(self,state,host,reply,op=1,value=1,notice=None,context='same'):
        def action(worker):
            if op==2:
                # Prepare the private fixture before exercising the 100ms query.
                response=self.api.meltype_tsf_service_request(c.addressof(state),worker,b'{}',5000,0)
                self.assertTrue(response);self.api.meltype_free(response)
            return self.mode_action(state,host,op,value,worker,notice,context)
        return self.fixture_recovery(state,reply,action)
    def test_scope_native_private_secret_source_labels_and_duplicate_suppression(self):
        state=self.create();host=ModeHost();events=[];host.values[0].values=[61]
        notice=CORE_NOTICE(lambda owner,op,value:events.append((owner,op,value)))
        self.assertEqual(self.scope(state,host,notice),0);self.assertTrue(state.private_field)
        self.assertEqual(c.wstring_at(state.scope_source),'Windows とアプリ')
        self.assertEqual(self.scope(state,host,notice),0);self.assertEqual(events,[(123,4,3)])
        host.values[0].values=[31];host.present[1]=False
        self.assertEqual(self.scope(state,host,notice),1);self.assertFalse(state.private_field)
        self.assertEqual(events[-1],(123,4,1));self.assertEqual(c.wstring_at(state.scope_source),'Windows');host.verify(self)
    def test_scope_native_unreadable_metadata_preserves_private_and_notifies_reason(self):
        state=self.create();host=ModeHost();host.values[0].fail_query=host.values[0].partial_query=True;host.values[1].values=[61];events=[]
        notice=CORE_NOTICE(lambda owner,op,value:events.append((op,value)))
        self.assertEqual(self.scope(state,host,notice),1);self.assertTrue(state.private_field)
        self.assertEqual(events,[(4,3),(5,1)])
        self.assertEqual(self.scope(state,host,notice),1);self.assertEqual(events[-1],(5,1));self.assertEqual(events.count((4,3)),1);host.verify(self)
    def test_scope_native_last_caller_release_keeps_state_through_notice(self):
        state=self.create();address=c.addressof(state);host=ModeHost();host.values[0].values=[61];events=[]
        dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):events.append(('destroy',state.private_field));self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        @self.checked(CORE_NOTICE)
        def notice(owner,op,value):
            self.assertEqual((owner,op,value),(123,4,3));self.assertTrue(state.private_field);events.append('scope');release(root)
            self.assertEqual(self.live(),self.before+1)
        try:
            self.assertEqual(self.scope(state,host,notice,root),0);self.assertEqual(events,['scope',('destroy',True)]);host.verify(self)
        finally:
            if not events:release(root)
    def test_scope_native_invalid_owner_is_secret_without_mutating_state(self):
        state=self.create();host=ModeHost();address=c.addressof(state);before=c.string_at(address,c.sizeof(State))
        for data,lifetime in [(None,c.byref(host.context)),(address,None)]:
            self.assertEqual(self.api.meltype_tsf_service_input_scope(data,lifetime,c.byref(host.context),7,None,CORE_NOTICE()),1)
        self.assertEqual(c.string_at(address,c.sizeof(State)),before);self.assertEqual(host.value_reads,[]);host.verify(self)
    def test_mode_action_native_plain_toggle_uses_owned_compartment_state(self):
        state=self.create();host=ModeHost();state.client=17;state.manager=c.addressof(host.context)
        host.on_write=lambda:self.opened(state,host)
        try:
            self.assertEqual(self.mode_action(state,host,op=0),1);self.assertFalse(state.open)
            self.assertEqual(self.mode_action(state,host,op=0),1);self.assertTrue(state.open)
            self.assertEqual(host.read_count,0);self.assertFalse(state.transport);host.verify(self)
        finally:state.manager=None
    def test_mode_action_native_code_request_metadata_privacy_and_reply_order(self):
        state=self.create();host=ModeHost();state.client=17;state.server_active=True;state.code_app=True;events=[]
        sid='切替"😀';self.api.meltype_tsf_service_set_sid(c.addressof(state),sid.encode())
        notice=CORE_NOTICE(lambda owner,op,value:events.append((op,value)))
        result,request=self.fixture_mode(state,host,'{"active":true,"code":true,"english":true,"consumed":true}',value=0,notice=notice)
        self.assertEqual(result,1);self.assertEqual((request['op'],request['sid'],request['japanese']),('toggle',sid,False))
        self.assertEqual(request['before'],'print("日本語😀")');self.assertEqual(request['process'].lower(),'python')
        self.assertEqual(events,[(4,3),(2,1),(3,0)]);self.assertTrue(state.code_english);host.verify(self)
    def test_mode_action_native_focus_fallback_releases_document_and_context(self):
        state=self.create();host=ModeHost();state.client=17;state.server_active=True;state.code_app=True;state.manager=c.addressof(host.thread)
        try:
            result,request=self.fixture_mode(state,host,'{"active":true,"consumed":true}',op=0,context=None)
            self.assertEqual(result,1);self.assertEqual(request['op'],'toggle');self.assertFalse(request['japanese'])
            self.assertGreater(host.read_count,0);host.verify(self)
        finally:state.manager=None
    def test_mode_action_native_query_skips_noncode_inactive_and_secret_text(self):
        state=self.create();host=ModeHost();state.client=17
        for code,active in [(False,False),(True,False),(False,True)]:
            state.code_app=code;state.server_active=active;self.assertEqual(self.mode_action(state,host,op=2),0)
        self.assertEqual(host.value_reads,[])
        state.code_app=state.server_active=True;host.values[0].values=[31]
        self.assertEqual(self.mode_action(state,host,op=2),0);self.assertEqual(host.read_count,0);self.assertFalse(state.transport);host.verify(self)
    def test_mode_action_native_query_applies_reply_without_consumed_requirement(self):
        state=self.create();host=ModeHost();state.client=17;state.code_app=state.server_active=True;state.code_english=True
        result,request=self.fixture_mode(state,host,'{"active":false,"code":true,"english":true,"consumed":false}',op=2,value=0)
        self.assertEqual(result,1);self.assertEqual(request['op'],'mode');self.assertEqual(request['before'],'print("日本語😀")')
        self.assertFalse(state.code_app);self.assertFalse(state.code_english);self.assertTrue(state.server_active);host.verify(self)
    def test_mode_action_native_invalid_reply_does_not_change_display_state(self):
        state=self.create();host=ModeHost();state.client=17;state.code_app=state.server_active=True;events=[]
        result,_=self.fixture_mode(state,host,'invalid JSON',notice=CORE_NOTICE(lambda owner,op,value:events.append((op,value))))
        self.assertEqual(result,0);self.assertTrue(state.code_app);self.assertFalse(state.code_english)
        self.assertEqual(events,[(4,3),(1,0)]);host.verify(self)
    def test_mode_action_native_all_response_flags_preserve_strict_booleans(self):
        state=self.create();host=ModeHost();state.client=17;state.server_active=True
        for flags in range(16):
            value={key:bool(flags&(1<<bit)) for bit,key in enumerate(['active','code','english','consumed'])}
            result,_=self.fixture_mode(state,host,json.dumps(value))
            self.assertEqual(result,int(value['active'] and value['consumed']))
            self.assertEqual(state.code_app,value['active'] and value['code'])
            self.assertEqual(state.code_english,value['active'] and value['code'] and value['english'])
        result,_=self.fixture_mode(state,host,'{"active":1,"code":true,"english":true,"consumed":true}')
        self.assertEqual(result,0);self.assertFalse(state.code_app);self.assertFalse(state.code_english);host.verify(self)
    def test_mode_action_native_compatible_reply_parser_accepts_surrogate_escape(self):
        state=self.create();host=ModeHost();state.client=17;state.server_active=True;events=[]
        result,_=self.fixture_mode(state,host,'{"active":true,"consumed":true,"label":"\\uD800"}',notice=CORE_NOTICE(lambda owner,op,value:events.append((op,value))))
        self.assertEqual(result,1);self.assertNotIn((1,0),events);host.verify(self)
    def test_mode_action_native_delayed_read_uses_tracked_line_and_disables_late_work(self):
        state=self.create();host=ModeHost();state.client=17;state.server_active=True;host.lock_mode='async'
        try:
            result,request=self.fixture_mode(state,host,'{"active":true,"consumed":true}')
            self.assertEqual(result,1);self.assertNotIn('before',request);self.assertEqual(host.read_count,0)
            self.assertIsNotNone(host.pending);self.assertEqual(invoke(host.pending,3,HR,[U32],7),HR(0x80004004).value)
        finally:
            if host.pending:release(host.pending);host.pending=None
            host.verify(self)
    def test_mode_action_native_invalid_arguments_preserve_state(self):
        state=self.create();host=ModeHost();address=c.addressof(state);before=c.string_at(address,c.sizeof(State))
        for data,lifetime,worker,op in [(None,c.byref(host.context),self.worker,0),(address,None,self.worker,0),
            (address,c.byref(host.context),None,0),(address,c.byref(host.context),self.worker,3)]:
            self.assertEqual(self.api.meltype_tsf_service_mode_action(data,lifetime,worker,c.byref(host.context),7,op,0,None,CORE_NOTICE()),0)
        self.assertEqual(c.string_at(address,c.sizeof(State)),before);host.verify(self)
    def keyboard(self,shift=False,control=False):
        saved=(c.c_ubyte*256)();self.assertTrue(user.GetKeyboardState(saved));self.addCleanup(user.SetKeyboardState,saved)
        self.set_keyboard(shift,control)
    def set_keyboard(self,shift=False,control=False):
        value=(c.c_ubyte*256)()
        if shift:value[0x10]=value[0xa0]=0x80
        if control:value[0x11]=value[0xa2]=0x80
        self.assertTrue(user.SetKeyboardState(value))
    def key_state(self,state):
        if not state.keys:state.keys=self.api.meltype_tsf_key_state_create()
        self.assertTrue(state.keys)
    def key_secret(self,state,host,key=65,test=True,notice=None,lifetime=None):
        return self.api.meltype_tsf_service_key_secret(c.addressof(state),lifetime or c.byref(host.context),c.byref(host.context),key,int(test),123,notice or CORE_NOTICE(lambda *_:None))
    def would_key(self,state,host,key=65,test=True,commit=None,notice=None):
        ch=c.c_uint16(999)
        result=self.api.meltype_tsf_service_would_eat(c.addressof(state),c.byref(host.context),(c.byref(host.sink) if hasattr(host,'sink') else None),self.worker,c.byref(host.context),key,0,int(test),123,
            commit or KEY_COMMIT(lambda *_:0),notice or CORE_NOTICE(lambda *_:None),c.byref(ch))
        return result,ch.value
    def key_event(self,state,host,kind,key=65,commit=None,handler=None,lifetime=None):
        return self.api.meltype_tsf_service_key_event(c.addressof(state),lifetime or c.byref(host.context),(c.byref(host.sink) if hasattr(host,'sink') else None),self.worker,kind,c.byref(host.context),key,0,123,
            commit or KEY_COMMIT(lambda *_:0),handler or KEY_HANDLE(lambda *_:None),CORE_NOTICE())
    def test_key_secret_native_test_result_and_private_metadata_are_reused_once(self):
        state=self.create();host=ModeHost();state.client=17;self.key_state(state);host.values[0].values=[61]
        try:
            self.assertEqual(self.key_secret(state,host),0);self.assertTrue(state.private_field);self.assertEqual(len(host.locks),1)
            host.values[0].values=[31]
            self.assertEqual(self.key_secret(state,host,test=False),0);self.assertTrue(state.private_field);self.assertEqual(len(host.locks),1)
            self.assertEqual(self.key_secret(state,host,test=False),1);self.assertFalse(state.private_field);self.assertEqual(len(host.locks),2)
        finally:self.api.meltype_tsf_key_cache_clear(state.keys);host.verify(self)
    def test_key_secret_native_refused_read_stays_unconfirmed(self):
        state=self.create();host=ModeHost();state.client=17;self.key_state(state);host.lock_mode='reject'
        try:
            self.assertEqual(self.key_secret(state,host),-1);self.assertEqual(host.value_reads,[]);self.assertFalse(state.secret_checked)
        finally:self.api.meltype_tsf_key_cache_clear(state.keys);host.verify(self)
    def test_key_secret_native_delayed_sync_read_is_unknown_and_cannot_run_later(self):
        state=self.create();host=ModeHost();state.client=17;self.key_state(state);host.lock_mode='async'
        try:
            self.assertEqual(self.key_secret(state,host),-1);self.assertIsNotNone(host.pending)
            self.assertEqual(invoke(host.pending,3,HR,[U32],7),HR(0x80004004).value);self.assertEqual(host.value_reads,[])
        finally:
            self.api.meltype_tsf_key_cache_clear(state.keys)
            if host.pending:release(host.pending);host.pending=None
            host.verify(self)
    def test_key_secret_native_owner_survives_private_write_after_notice_release(self):
        state=self.create();address=c.addressof(state);host=ModeHost();state.client=17;self.key_state(state);host.values[0].values=[61];events=[]
        dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):events.append(('destroy',state.private_field));self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        @self.checked(CORE_NOTICE)
        def notice(owner,op,value):
            self.assertEqual((owner,op,value),(123,4,3));events.append('scope');release(root);self.assertEqual(self.live(),self.before+1)
        try:
            self.assertEqual(self.key_secret(state,host,test=False,notice=notice,lifetime=root),0)
            self.assertEqual(events,['scope',('destroy',True)]);host.verify(self)
        finally:
            if not events:release(root)
    def test_would_native_disabled_and_modifier_reset_only_per_key_secret_flags(self):
        self.keyboard();state=self.create();host=ModeHost();self.key_state(state)
        for disabled,key in [(True,65),(False,0x10)]:
            state.disabled=disabled;state.secret_checked=state.secret_composing=True
            self.assertEqual(self.would_key(state,host,key),(0,0));self.assertFalse(state.secret_checked);self.assertFalse(state.secret_composing)
        self.assertEqual(host.value_reads,[]);self.assertFalse(state.transport);host.verify(self)
    def test_would_native_pass_reason_deduplicates_and_success_clears_it(self):
        self.keyboard();state=self.create();host=ModeHost();state.client=17;self.key_state(state);state.open=False;events=[]
        notice=CORE_NOTICE(lambda owner,op,value:events.append((op,value)))
        try:
            self.assertEqual(self.would_key(state,host,notice=notice),(0,0));self.assertEqual(self.would_key(state,host,notice=notice),(0,0))
            self.assertEqual(events,[(6,1)]);self.assertEqual(c.wstring_at(state.pass_reason),'英数')
            state.open=True;state.server_active=True;self.set_keyboard(shift=True)
            self.assertEqual(self.would_key(state,host,0x20,notice=notice),(1,0x20));self.assertFalse(state.pass_reason);self.assertTrue(state.secret_checked)
        finally:self.api.meltype_tsf_key_cache_clear(state.keys);host.verify(self)
    def test_would_native_password_blocks_and_private_input_sets_checked_metadata(self):
        self.keyboard(shift=True);state=self.create();host=ModeHost();state.client=17;state.server_active=True;self.key_state(state)
        try:
            host.values[0].values=[31];self.assertEqual(self.would_key(state,host,0x20),(0,0x20));self.assertFalse(state.secret_checked)
            self.assertEqual(c.wstring_at(state.pass_reason),'パスワードの入力欄');host.values[0].values=[61]
            self.assertEqual(self.would_key(state,host,0x20),(1,0x20));self.assertTrue(state.private_field);self.assertTrue(state.secret_checked)
            self.assertEqual(host.read_count,0)
        finally:self.api.meltype_tsf_key_cache_clear(state.keys);host.verify(self)
    def test_would_native_unicode_packet_requests_sync_then_async_keep_caret(self):
        self.keyboard();state=self.create();host=DeferredHost();self.shutdown_composition(state,host);self.key_state(state);events=[]
        commit=KEY_COMMIT(lambda owner,mode,keep:events.append((owner,mode,keep)) or 0)
        try:
            self.assertEqual(self.would_key(state,host,0xe7,commit=commit),(0,0));self.assertEqual(events,[(123,0,1),(123,1,1)])
        finally:self.cleanup_shutdown_composition(state,host)
    def test_would_native_stale_test_actual_pair_counts_once_then_retires_old_input(self):
        self.keyboard();state=self.create();old=DeferredHost();host=ModeHost();self.shutdown_composition(state,old);self.key_state(state);state.server_active=True;events=[]
        commit=KEY_COMMIT(lambda owner,mode,keep:events.append((mode,keep)) or 0)
        try:
            self.assertEqual(self.would_key(state,host,65,commit=commit)[0],0);self.assertEqual(self.stale_tries(state),1)
            self.assertEqual(self.would_key(state,host,65,test=False,commit=commit)[0],0);self.assertEqual(self.stale_tries(state),1)
            self.assertEqual(self.would_key(state,host,66,commit=commit)[0],1)
            self.assertFalse(state.composition);self.assertTrue(state.abandoned);self.assertEqual(self.stale_tries(state),0)
            self.assertEqual(events,[(0,1),(1,1),(0,1),(0,1)])
        finally:
            self.api.meltype_tsf_key_cache_clear(state.keys);self.cleanup_native_abandoned(state,old);host.verify(self)
    def test_key_event_native_four_entries_toggle_and_release_consumed_keys(self):
        self.keyboard();state=self.create();host=ModeHost();self.key_state(state);state.client=17;state.manager=c.addressof(host.context)
        host.on_write=lambda:self.opened(state,host);events=[]
        try:
            handler=KEY_HANDLE(lambda *_:events.append('handle'))
            self.assertEqual(self.key_event(state,host,0,0x19,handler=handler),1);self.assertTrue(state.open)
            self.assertEqual(self.key_event(state,host,1,0x19,handler=handler),1);self.assertFalse(state.open)
            self.assertEqual(self.key_event(state,host,2,0x19),1);self.assertEqual(self.key_event(state,host,3,0x19),1)
            self.assertEqual(self.key_event(state,host,2,0x19),0);self.assertEqual(events,[]);self.assertFalse(state.transport);host.verify(self)
        finally:state.manager=None
    def test_key_event_native_passed_key_marks_caret_and_records_metadata(self):
        self.keyboard();state=self.create();host=ModeHost();self.key_state(state);state.open=False;state.direct_word=True
        self.assertEqual(self.key_event(state,host,1,0x25),0);self.assertTrue(state.caret_moved);self.assertTrue(state.direct_word)
        passed=(c.c_int*2)();self.assertEqual(self.api.meltype_tsf_key_passed(state.keys,0,0,passed),1);self.assertEqual(tuple(passed),(0x25,0));host.verify(self)
    def test_key_event_native_handle_can_release_last_caller_reference(self):
        self.keyboard(shift=True);state=self.create();address=c.addressof(state);host=ModeHost();self.key_state(state);state.client=17;state.server_active=True;events=[]
        dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):events.append('destroy');self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        @self.checked(KEY_HANDLE)
        def handle(owner,context,key,ch):
            self.assertEqual((owner,context,key,ch),(123,c.addressof(host.context),0x20,0x20));events.append('handle');release(root)
            self.assertEqual(self.live(),self.before+1)
        try:
            self.assertEqual(self.key_event(state,host,1,0x20,handler=handle,lifetime=root),1);self.assertEqual(events,['handle','destroy']);host.verify(self)
        finally:
            if not events:release(root)
    def test_key_secret_native_null_context_is_unknown_and_clears_private_result(self):
        state=self.create();host=ModeHost();self.key_state(state);state.private_field=True
        self.assertEqual(self.api.meltype_tsf_service_key_secret(c.addressof(state),c.byref(host.context),None,65,0,None,CORE_NOTICE()),-1)
        self.assertFalse(state.private_field);self.assertEqual(host.value_reads,[]);host.verify(self)
    def test_would_native_resumes_host_committed_input_with_composition_sink(self):
        self.keyboard();state=self.create();host=DeferredHost();host.mode='sync';state.client=17;state.atom_input=11;self.key_state(state)
        state.resume=self.capture_resume(host)
        try:
            self.assertEqual(self.would_key(state,host,65),(1,0));self.assertFalse(state.resume);self.assertTrue(state.composition)
            self.assertEqual(host.attributes,[(0,3,11)]);self.assertEqual(host.document,units('old'));self.assertFalse(state.transport)
        finally:self.cleanup_recovered(state,host)
    def test_key_input_native_invalid_arguments_preserve_state_and_zero_character(self):
        state=self.create();host=ModeHost();address=c.addressof(state);life=c.byref(host.context);sink=(c.byref(host.sink) if hasattr(host,'sink') else None);before=c.string_at(address,c.sizeof(State));out=c.c_uint16()
        for data,lifetime,worker in [(None,life,self.worker),(address,None,self.worker),(address,life,None)]:
            out.value=999
            self.assertEqual(self.api.meltype_tsf_service_would_eat(data,lifetime,sink,worker,c.byref(host.context),65,0,1,None,KEY_COMMIT(lambda *_:0),CORE_NOTICE(),c.byref(out)),0)
            self.assertEqual(out.value,0)
            self.assertEqual(self.api.meltype_tsf_service_key_event(data,lifetime,sink,worker,0,c.byref(host.context),65,0,None,KEY_COMMIT(lambda *_:0),KEY_HANDLE(lambda *_:None),CORE_NOTICE()),0)
        self.assertEqual(self.api.meltype_tsf_service_key_secret(None,life,c.byref(host.context),65,1,None,CORE_NOTICE()),1)
        self.assertEqual(c.string_at(address,c.sizeof(State)),before);host.verify(self)
    def service_commit(self,state,host,mode=0,keep=True,worker=None,apply=None,notice=None,context=None,lifetime=None):
        return self.api.meltype_tsf_service_commit(c.addressof(state),lifetime or c.byref(host.context),worker or self.worker,mode,
            context or c.byref(host.context),7,int(keep),123,apply or COMMIT_APPLY(lambda *_:1),notice or TOGGLE())
    def service_select(self,state,host,index=2,prediction=False,worker=None,apply=None,lifetime=None):
        return self.api.meltype_tsf_service_select_candidate(c.addressof(state),lifetime or c.byref(host.context),worker or self.worker,index,int(prediction),123,apply or COMMIT_APPLY(lambda *_:1),TOGGLE())
    def test_commit_native_sync_metadata_state_and_cleanup(self):
        state=self.create();host=DeferredHost();host.mode='sync';self.shutdown_composition(state,host);self.stale_state(state)
        state.edit_context=c.addressof(host.context);host.context_ref(1);events=[]
        sid='commit"😀';self.api.meltype_tsf_service_set_sid(c.addressof(state),sid.encode())
        @self.checked(COMMIT_APPLY)
        def apply(owner,cookie,context,bytes,count):
            events.append((owner,cookie,context,state.own_edit,state.keep_caret,c.string_at(bytes,count)));return 1
        try:
            result,request=self.fixture_recovery(state,'{}',lambda worker:self.service_commit(state,host,worker=worker,apply=apply))
            self.assertEqual(result,0);self.assertEqual(request,{'op':'commit','sid':sid,'moved':True})
            self.assertEqual(events,[(123,7,c.addressof(host.context),True,True,b'{}')])
            self.assertFalse(state.own_edit);self.assertFalse(state.keep_caret);self.assertFalse(state.composition);self.assertEqual(host.ended,1);self.assertEqual(self.stale_tries(state),0)
        finally:self.cleanup_recovered(state,host)
    def test_commit_native_existing_cookie_preserves_own_mark_and_omits_moved(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host);state.edit_context=c.addressof(host.context);host.context_ref(1)
        try:
            result,request=self.fixture_recovery(state,'{}',lambda worker:self.service_commit(state,host,mode=2,keep=False,worker=worker))
            self.assertEqual(result,0);self.assertEqual(request,{'op':'commit','sid':''});self.assertTrue(state.own_edit);self.assertFalse(state.keep_caret);self.assertEqual(host.requests,[])
        finally:self.cleanup_recovered(state,host)
    def test_commit_native_other_field_does_not_mark_own_edit(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host)
        try:
            result,_=self.fixture_recovery(state,'{}',lambda worker:self.service_commit(state,host,mode=2,worker=worker))
            self.assertEqual(result,0);self.assertFalse(state.own_edit);self.assertEqual(host.ended,1)
        finally:self.cleanup_recovered(state,host)
    def test_commit_native_invalid_reply_not_applied_but_composition_ends(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host);events=[]
        try:
            result,_=self.fixture_recovery(state,'invalid JSON',lambda worker:self.service_commit(state,host,mode=2,worker=worker,
                apply=COMMIT_APPLY(lambda *_:events.append('apply') or 1),notice=TOGGLE(lambda *_:events.append('invalid'))))
            self.assertEqual(result,0);self.assertEqual(events,['invalid']);self.assertEqual(host.ended,1);self.assertFalse(state.keep_caret)
        finally:self.cleanup_recovered(state,host)
    def test_commit_native_compatible_surrogate_reply_reaches_value_adapter(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host);events=[]
        @self.checked(COMMIT_APPLY)
        def apply(owner,cookie,context,bytes,count):events.append(c.string_at(bytes,count));return 1
        try:
            reply='{"value":"\\uD800"}'
            result,_=self.fixture_recovery(state,reply,lambda worker:self.service_commit(state,host,mode=2,worker=worker,apply=apply))
            self.assertEqual(result,0);self.assertEqual(events,[reply.encode()])
        finally:self.cleanup_recovered(state,host)
    def test_commit_native_incorrectly_deferred_sync_is_invalidated(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host);state.own_edit=True
        try:
            self.assertEqual(self.service_commit(state,host),HR(0x80004004).value);self.assertFalse(state.own_edit)
            pending,host.pending=host.pending,[]
            for session in pending:
                try:self.assertEqual(invoke(session,3,HR,[U32],7),HR(0x80004004).value)
                finally:release(session)
            self.assertEqual(host.ended,0);self.assertTrue(state.composition)
        finally:self.cleanup_recovered(state,host)
    def test_commit_native_refused_sync_clears_own_without_edits(self):
        state=self.create();host=DeferredHost();host.mode='reject';self.shutdown_composition(state,host);state.own_edit=True
        try:
            self.assertLess(self.service_commit(state,host),0);self.assertFalse(state.own_edit);self.assertEqual(host.ended,0)
        finally:self.cleanup_recovered(state,host)
    def test_commit_native_async_owns_adapter_and_worker_until_execution(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host);events=[]
        @self.checked(COMMIT_APPLY)
        def apply(owner,cookie,context,bytes,count):events.append(state.keep_caret);return 1
        def action(worker):
            path=c.create_string_buffer(worker)
            result=self.service_commit(state,host,mode=1,worker=path,apply=apply);self.assertGreater(result,0)
            path.value=b'destroyed';self.assertEqual(events,[]);host.flush();return result
        try:
            _,request=self.fixture_recovery(state,'{}',action)
            self.assertEqual(request['op'],'commit');self.assertEqual(events,[True]);self.assertFalse(state.composition);self.assertEqual(host.ended,1)
        finally:self.cleanup_recovered(state,host)
    def test_commit_native_stale_async_never_ends_new_input(self):
        state=self.create();host=DeferredHost();other=DeferredHost();self.shutdown_composition(state,host)
        try:
            self.assertGreater(self.service_commit(state,host,mode=1),0);self.clear_owned_pair(state);self.shutdown_composition(state,other)
            host.flush();self.assertEqual(other.ended,0);self.assertTrue(state.composition)
        finally:self.cleanup_recovered(state,other);host.verify(self)
    def test_commit_native_reply_reentry_preserves_new_composition(self):
        state=self.create();host=DeferredHost();other=DeferredHost();self.shutdown_composition(state,host)
        @self.checked(COMMIT_APPLY)
        def apply(*_):self.clear_owned_pair(state);self.shutdown_composition(state,other);return 1
        try:
            self.fixture_recovery(state,'{}',lambda worker:self.service_commit(state,host,mode=2,worker=worker,apply=apply))
            self.assertEqual(state.composition_context,c.addressof(other.context));self.assertEqual(other.ended,0);self.assertFalse(state.keep_caret)
        finally:self.cleanup_recovered(state,other);host.verify(self)
    def test_select_native_async_metadata_and_state_without_ending(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host);events=[]
        @self.checked(COMMIT_APPLY)
        def apply(*_):events.append((state.own_edit,state.keep_caret));return 1
        def action(worker):
            result=self.service_select(state,host,index=-1,prediction=True,worker=worker,apply=apply);self.assertEqual(events,[]);host.flush();return result
        try:
            result,request=self.fixture_recovery(state,'{}',action)
            self.assertGreater(result,0);self.assertEqual(request,{'op':'select','sid':'','index':-1,'prediction':True})
            self.assertEqual(events,[(True,False)]);self.assertTrue(state.composition);self.assertEqual(host.ended,0)
        finally:self.cleanup_recovered(state,host)
    def test_select_native_stale_and_cancelled_sessions_do_not_apply(self):
        for cancel in [False,True]:
            state=self.create();host=DeferredHost();self.shutdown_composition(state,host);events=[]
            apply=COMMIT_APPLY(lambda *_:events.append(1) or 1)
            try:
                self.assertGreater(self.service_select(state,host,apply=apply),0)
                if cancel:host.cancel()
                else:self.clear_owned_pair(state);host.flush()
                self.assertEqual(events,[]);self.assertFalse(state.own_edit)
            finally:self.cleanup_recovered(state,host)
    def test_commit_native_sync_last_caller_release_survives_end_and_own_cleanup(self):
        state=self.create();host=DeferredHost();host.mode='sync';self.shutdown_composition(state,host);address=c.addressof(state);events=[]
        dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):
            events.append(('destroy',state.composition,state.keep_caret,state.own_edit));self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        @self.checked(COMMIT_APPLY)
        def apply(*_):
            events.append('apply');release(root);self.assertEqual(self.live(),self.before+1);return 1
        self.fixture_recovery(state,'{}',lambda worker:self.service_commit(state,host,worker=worker,apply=apply,lifetime=root))
        self.assertEqual(events,['apply',('destroy',None,False,False)]);host.verify(self)
    def test_commit_native_async_last_caller_release_retained_until_cancel(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host);address=c.addressof(state);events=[]
        dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):
            self.clear_owned_pair(state);events.append('destroy');self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        self.assertGreater(self.service_commit(state,host,mode=1,lifetime=root),0);release(root)
        self.assertEqual(events,[]);self.assertEqual(self.live(),self.before+1);host.cancel()
        self.assertEqual(events,['destroy']);host.verify(self)
    def test_select_native_last_caller_release_retained_through_execution(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host);address=c.addressof(state);events=[]
        dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):
            events.append(('destroy',state.own_edit));self.clear_owned_pair(state);self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        @self.checked(COMMIT_APPLY)
        def apply(*_):events.append('apply');self.assertEqual(self.live(),self.before+1);return 1
        def action(worker):
            result=self.service_select(state,host,worker=worker,apply=apply,lifetime=root);release(root)
            self.assertEqual(events,[]);host.flush();return result
        self.fixture_recovery(state,'{}',action)
        self.assertEqual(events,['apply',('destroy',True)]);host.verify(self)
    def test_commit_native_invalid_arguments_preserve_state(self):
        state=self.create();host=DeferredHost();address=c.addressof(state);before=c.string_at(address,c.sizeof(State));apply=COMMIT_APPLY(lambda *_:1)
        for data,life,worker,callback in [(None,c.byref(host.context),self.worker,apply),(address,None,self.worker,apply),(address,c.byref(host.context),None,apply),(address,c.byref(host.context),self.worker,COMMIT_APPLY())]:
            self.assertEqual(self.api.meltype_tsf_service_commit(data,life,worker,0,c.byref(host.context),7,1,None,callback,TOGGLE()),HR(0x80070057).value)
            self.assertEqual(self.api.meltype_tsf_service_select_candidate(data,life,worker,0,0,None,callback,TOGGLE()),HR(0x80070057).value)
        self.assertEqual(self.service_commit(state,host,mode=3),HR(0x80070057).value);self.assertEqual(c.string_at(address,c.sizeof(State)),before);host.verify(self)
    def handle_service_key(self,state,host,key=65,ch=97,worker=None,apply=None,notice=None,sender=None,lifetime=None,context=None):
        # Always replace SendInput in private tests, including unexpected failures.
        send=sender or SENDER(lambda *_:2)
        return self.api.meltype_tsf_service_handle_key(c.addressof(state),lifetime or c.byref(host.context),
            c.byref(host.sink) if hasattr(host,'sink') else None,worker or self.worker,context or c.byref(host.context),key,ch,123,
            apply or COMMIT_APPLY(lambda *_:1),notice or CORE_NOTICE(),send)
    def key_work_state(self):
        self.keyboard();state=self.create();self.key_state(state);state.client=17;state.secret_checked=True
        host=DeferredHost();host.mode='sync';return state,host
    def test_key_work_native_metadata_state_request_and_apply(self):
        state,host=self.key_work_state();state.caret_moved=True;state.private_field=True;events=[]
        self.api.meltype_tsf_service_set_sid(c.addressof(state),'key"😀'.encode())
        @self.checked(COMMIT_APPLY)
        def apply(owner,cookie,context,bytes,count):
            events.append((owner,cookie,context,state.own_edit,state.server_active,state.direct_detection,state.direct_word,state.consumed));return 1
        try:
            result,request=self.fixture_recovery(state,'{"active":true,"directDetection":true,"directEnglishWord":true,"consumed":true,"fullWidthSpace":true}',
                lambda worker:self.handle_service_key(state,host,worker=worker,apply=apply))
            self.assertEqual(result,0);self.assertEqual(request['sid'],'key"😀');self.assertEqual(request['process'].lower(),Path(sys.executable).stem.lower())
            self.assertEqual({k:request[k] for k in ['op','vk','ch','mods','code','direct','canDelete','moved','private','before','after']},
                {'op':'key','vk':65,'ch':97,'mods':0,'code':True,'direct':False,'canDelete':True,'moved':True,'private':True,'before':'old','after':''})
            self.assertEqual(events,[(123,7,c.addressof(host.context),True,True,True,True,True)])
            self.assertTrue(state.full_width_space)
            self.assertFalse(state.own_edit);self.assertFalse(state.caret_moved);self.assertFalse(state.key_failed)
        finally:self.cleanup_recovered(state,host)
    def test_key_work_native_normalizes_dependent_server_flags(self):
        state,host=self.key_work_state()
        try:
            state.full_width_space=True
            self.fixture_recovery(state,'{"active":false,"directDetection":true,"directEnglishWord":true,"consumed":true,"fullWidthSpace":true}',lambda worker:self.handle_service_key(state,host,worker=worker))
            self.assertFalse(state.server_active);self.assertFalse(state.direct_detection);self.assertFalse(state.direct_word)
            self.assertFalse(state.full_width_space)
        finally:self.cleanup_recovered(state,host)
    def test_key_work_native_composing_request_omits_surrounding_text(self):
        state,host=self.key_work_state();self.shutdown_composition(state,host)
        try:
            _,request=self.fixture_recovery(state,'{"active":true,"consumed":true}',lambda worker:self.handle_service_key(state,host,worker=worker))
            self.assertTrue(request['composing']);self.assertNotIn('before',request);self.assertNotIn('after',request)
        finally:self.cleanup_recovered(state,host)
    def test_key_work_native_bad_reply_inserts_fallback_without_reinjection(self):
        state,host=self.key_work_state();events=[];sent=[]
        try:
            result,_=self.fixture_recovery(state,'invalid',lambda worker:self.handle_service_key(state,host,worker=worker,
                notice=CORE_NOTICE(lambda owner,op,value:events.append(op)),sender=SENDER(lambda *_:sent.append(1) or 2)))
            self.assertEqual(result,0);self.assertEqual(host.document,units('olda'));self.assertTrue(state.key_failed);self.assertFalse(state.server_active)
            self.assertEqual(events,[4,3]);self.assertEqual(sent,[]);self.assertFalse(state.composition);self.assertEqual(host.ended,1)
        finally:self.cleanup_recovered(state,host)
    def test_key_work_native_failed_fallback_write_is_reinjected_even_if_composition_ended(self):
        state,host=self.key_work_state();host.fail_write=True;sent=[]
        @self.checked(SENDER)
        def send(owner,inputs,count):
            self.assertEqual((owner,count),(123,2));sent.append((inputs[0].payload.keyboard.key,inputs[1].payload.keyboard.flags));return count
        try:
            self.fixture_recovery(state,'invalid',lambda worker:self.handle_service_key(state,host,worker=worker,sender=send))
            self.assertEqual(sent,[(65,2)]);self.assertEqual(host.document,units('old'));self.assertEqual(host.ended,1);self.assertFalse(state.composition)
        finally:self.cleanup_recovered(state,host)
    def test_key_work_native_unconsumed_english_inserts_direct_character(self):
        state,host=self.key_work_state();state.open=False;sent=[]
        try:
            self.fixture_recovery(state,'{"active":true,"consumed":false}',lambda worker:self.handle_service_key(state,host,worker=worker,sender=SENDER(lambda *_:sent.append(1) or 2)))
            self.assertEqual(host.document,units('olda'));self.assertEqual(sent,[]);self.assertFalse(state.key_failed);self.assertFalse(state.consumed)
        finally:self.cleanup_recovered(state,host)
    def test_key_work_native_switch_opens_after_edit_and_does_not_insert_directly(self):
        state,host=self.key_work_state();state.open=False;events=[];manager=CompartmentHost();state.manager=c.addressof(manager.context)
        def opened():events.append(('open',state.own_edit));state.open=True
        manager.on_write=opened
        @self.checked(COMMIT_APPLY)
        def apply(*_):events.append((state.open,state.own_edit));return 1
        try:
            self.fixture_recovery(state,'{"active":true,"consumed":true,"switchToJapanese":true}',lambda worker:self.handle_service_key(state,host,worker=worker,apply=apply))
            self.assertEqual(events,[(False,True),('open',False)]);self.assertTrue(state.open);self.assertFalse(state.own_edit);self.assertEqual(host.document,units('old'))
            self.assertEqual(manager.writes_mode,[(bytes(OPEN),17,1)])
        finally:state.manager=None;self.cleanup_recovered(state,host);manager.verify(self)
    def test_key_work_native_apply_failure_sets_outcome_and_reinjects(self):
        state,host=self.key_work_state();sent=[]
        try:
            self.fixture_recovery(state,'{"active":true,"consumed":true}',lambda worker:self.handle_service_key(state,host,worker=worker,
                apply=COMMIT_APPLY(lambda *_:0),sender=SENDER(lambda *_:sent.append(1) or 2)))
            self.assertTrue(state.key_failed);self.assertFalse(state.consumed);self.assertEqual(sent,[1]);self.assertFalse(state.own_edit)
        finally:self.cleanup_recovered(state,host)
    def test_key_work_native_compatible_unpaired_reply_reaches_apply(self):
        state,host=self.key_work_state();events=[]
        @self.checked(COMMIT_APPLY)
        def apply(owner,cookie,context,bytes,count):events.append(c.string_at(bytes,count));return 1
        try:
            reply='{"active":true,"consumed":true,"extra":"\\uD800"}'
            self.fixture_recovery(state,reply,lambda worker:self.handle_service_key(state,host,worker=worker,apply=apply))
            self.assertEqual(events,[reply.encode()]);self.assertFalse(state.key_failed)
        finally:self.cleanup_recovered(state,host)
    def test_key_work_native_secret_field_never_requests_or_inserts(self):
        state,host=self.key_work_state();state.secret_composing=True;sent=[];events=[]
        try:
            self.assertEqual(self.handle_service_key(state,host,notice=CORE_NOTICE(lambda owner,op,value:events.append(op)),sender=SENDER(lambda *_:sent.append(1) or 2)),0)
            self.assertEqual(host.document,units('old'));self.assertFalse(state.transport);self.assertFalse(state.consumed);self.assertEqual(sent,[1]);self.assertEqual(events,[0])
        finally:self.cleanup_recovered(state,host)
    def test_key_work_native_incorrectly_deferred_session_cannot_use_stack(self):
        state,host=self.key_work_state();host.mode='async';sent=[]
        try:
            self.assertEqual(self.handle_service_key(state,host,sender=SENDER(lambda *_:sent.append(1) or 2)),HR(0x80004004).value)
            self.assertEqual(sent,[1]);self.assertTrue(state.key_failed);self.assertFalse(state.own_edit)
            pending,host.pending=host.pending,[]
            for session in pending:
                try:self.assertEqual(invoke(session,3,HR,[U32],7),HR(0x80004004).value)
                finally:release(session)
            self.assertFalse(state.transport);self.assertEqual(host.document,units('old'))
        finally:self.cleanup_recovered(state,host)
    def test_key_work_native_send_failure_reports_original_error(self):
        state,host=self.key_work_state();state.secret_composing=True;events=[]
        @self.checked(SENDER)
        def send(*_):c.windll.kernel32.SetLastError(5);return 0
        try:
            self.handle_service_key(state,host,sender=send,notice=CORE_NOTICE(lambda owner,op,value:events.append((op,value))))
            self.assertIn((9,5),events)
        finally:self.cleanup_recovered(state,host)
    def test_key_work_native_last_caller_release_pins_post_edit_cleanup(self):
        state,host=self.key_work_state();address=c.addressof(state);events=[];dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):events.append(('destroy',state.own_edit,state.key_failed));self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        @self.checked(COMMIT_APPLY)
        def apply(*_):events.append('apply');release(root);self.assertEqual(self.live(),self.before+1);return 1
        self.fixture_recovery(state,'{"active":true,"consumed":true}',lambda worker:self.handle_service_key(state,host,worker=worker,apply=apply,lifetime=root))
        self.assertEqual(events,['apply',('destroy',False,False)]);host.verify(self)
    def test_key_work_native_failed_edit_async_commit_owns_adapter_after_return(self):
        state,host=self.key_work_state();host.mode='async';self.shutdown_composition(state,host);address=c.addressof(state);events=[]
        dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):
            events.append(('destroy',state.composition,state.keep_caret));self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        @self.checked(COMMIT_APPLY)
        def apply(*_):events.append('apply');return 1
        @self.checked(CORE_NOTICE)
        def notice(owner,op,value):
            self.assertEqual(owner,123);events.append(op);self.assertEqual(self.live(),self.before+1)
        def action(worker):
            path=c.create_string_buffer(worker)
            result=self.handle_service_key(state,host,worker=path,apply=apply,notice=notice,lifetime=root)
            self.assertEqual(result,HR(0x80004004).value);self.assertEqual(len(host.pending),2)
            path.value=b'destroyed';release(root)
            session=host.pending.pop(0)
            try:self.assertEqual(invoke(session,3,HR,[U32],7),HR(0x80004004).value)
            finally:release(session)
            host.flush();return result
        _,request=self.fixture_recovery(state,'invalid',action)
        self.assertEqual(request['op'],'commit');self.assertEqual(events,[2,4,('destroy',None,False)]);host.verify(self)
    def test_key_work_native_invalid_arguments_preserve_state(self):
        state,host=self.key_work_state();address=c.addressof(state);before=c.string_at(address,c.sizeof(State));apply=COMMIT_APPLY(lambda *_:1);sender=SENDER(lambda *_:2)
        for data,life,worker,callback in [(None,c.byref(host.context),self.worker,apply),(address,None,self.worker,apply),(address,c.byref(host.context),None,apply),(address,c.byref(host.context),self.worker,COMMIT_APPLY())]:
            self.assertEqual(self.api.meltype_tsf_service_handle_key(data,life,c.byref(host.sink),worker,c.byref(host.context),65,97,None,callback,CORE_NOTICE(),sender),HR(0x80070057).value)
        self.assertEqual(self.handle_service_key(state,host,key=256),HR(0x80070057).value);self.assertEqual(c.string_at(address,c.sizeof(State)),before);host.verify(self)
    def service_apply(self,state,host,reply,worker=None,notice=None,lifetime=None):
        raw=reply.encode() if isinstance(reply,str) else json.dumps(reply,ensure_ascii=False).encode()
        return self.api.meltype_tsf_service_apply_reply(c.addressof(state),lifetime or c.byref(host.context),c.byref(host.sink),worker or self.worker,c.byref(host.context),7,raw,len(raw),123,notice or TOGGLE())
    def test_apply_native_commits_then_preedit_and_attributes(self):
        state=self.create();host=DeferredHost();host.selection=(0,3);state.atom_input=11;state.atom_converted=22;state.atom_target=33
        try:
            self.assertEqual(self.service_apply(state,host,{'active':True,'commits':[{'text':'確定'}],'view':{'text':'日本😀','converting':True,'clauses':['日','本😀'],'selectedClause':1}}),1)
            self.assertEqual(host.document,units('確定日本😀'));self.assertTrue(state.last_converting);self.assertTrue(state.composition);self.assertEqual(host.ended,1)
            self.assertEqual(host.attributes,[(2,3,22),(3,6,33)])
        finally:self.cleanup_recovered(state,host)
    def test_apply_native_active_empty_view_erases_and_ends(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host);self.stale_state(state);state.last_converting=True
        try:
            self.assertEqual(self.service_apply(state,host,{'active':True,'view':None}),1);self.assertEqual(host.document,[])
            self.assertFalse(state.composition);self.assertFalse(state.last_converting);self.assertEqual(host.ended,1);self.assertEqual(self.stale_tries(state),0)
        finally:self.cleanup_recovered(state,host)
    def test_apply_native_inactive_empty_view_preserves_text(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host)
        try:
            self.assertEqual(self.service_apply(state,host,{'active':False}),1);self.assertEqual(host.document,units('old'));self.assertFalse(state.composition);self.assertEqual(host.ended,1)
        finally:self.cleanup_recovered(state,host)
    def test_apply_native_commit_batch_anchors_after_preceding_edit_with_keep_caret(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host);host.selection=(0,0);state.keep_caret=True
        try:
            self.assertEqual(self.service_apply(state,host,{'commits':[{'text':'日本'},{'text':'😀'}]}),1)
            self.assertEqual(host.document,units('日本😀'));self.assertEqual(host.selection,(0,0));self.assertTrue(state.keep_caret);self.assertFalse(state.composition)
        finally:self.cleanup_recovered(state,host)
    def test_apply_native_guarded_failure_resets_core_then_marks_outcome(self):
        state=self.create();host=DeferredHost();state.open=False;state.last_converting=True
        self.api.meltype_tsf_service_set_sid(c.addressof(state),'reset"😀'.encode())
        try:
            result,request=self.fixture_recovery(state,'{}',lambda worker:self.service_apply(state,host,{'commits':[{'text':'wrong','deleteBefore':3,'expect':'bad'},{'text':'later'}],'view':{'text':'discard'}},worker=worker))
            self.assertEqual(result,0);self.assertEqual(request,{'op':'reset','sid':'reset"😀','direct':True})
            self.assertTrue(state.key_failed);self.assertFalse(state.consumed);self.assertTrue(state.last_converting);self.assertEqual(host.document,units('old'));self.assertFalse(state.composition)
        finally:self.cleanup_recovered(state,host)
    def test_apply_native_preserves_selection_guard_through_value_projection(self):
        state=self.create();host=DeferredHost();host.selection=(0,3)
        try:
            result,request=self.fixture_recovery(state,'{}',lambda worker:self.service_apply(state,host,{'active':True,'selectionExpect':'different','commits':[{'text':'BAD'}],'view':{'text':'BAD'}},worker=worker))
            self.assertEqual(result,0);self.assertEqual(request['op'],'reset')
            self.assertEqual(host.document,units('old'));self.assertFalse(state.composition)
            self.assertTrue(state.key_failed);self.assertFalse(state.consumed)
        finally:self.cleanup_recovered(state,host)
    def test_apply_native_matching_selection_starts_reconversion(self):
        state=self.create();host=DeferredHost();host.document=units('前😀old後');host.selection=(3,6)
        try:
            self.assertEqual(self.service_apply(state,host,{'active':True,'selectionExpect':'old','view':{'text':'再変換','converting':True}}),1)
            self.assertEqual(host.document,units('前😀再変換後'));self.assertTrue(state.composition)
        finally:self.cleanup_recovered(state,host)
    def test_apply_native_guarded_replacement_checks_expected_text(self):
        state=self.create();host=DeferredHost()
        try:
            self.assertEqual(self.service_apply(state,host,{'commits':[{'text':'日本😀','deleteBefore':3,'expect':'old'}]}),1)
            self.assertEqual(host.document,units('日本😀'));self.assertFalse(state.key_failed);self.assertFalse(state.composition)
        finally:self.cleanup_recovered(state,host)
    def test_apply_native_wrong_typed_values_follow_shell_defaults(self):
        state=self.create();host=DeferredHost();host.selection=(0,3)
        try:
            self.assertEqual(self.service_apply(state,host,{'active':'true','commits':[{'text':8,'deleteBefore':'3','expect':False}],'view':False}),1)
            self.assertEqual(host.document,[]);self.assertFalse(state.last_converting);self.assertFalse(state.composition)
        finally:self.cleanup_recovered(state,host)
    def test_apply_native_compatible_surrogates_are_replaced_at_value_projection(self):
        state=self.create();host=DeferredHost();host.selection=(0,3)
        try:
            self.assertEqual(self.service_apply(state,host,'{"commits":[{"text":"\\ud800\\ud83d\\ude00"}]}'),1)
            self.assertEqual(host.document,units('�😀'));self.assertFalse(state.key_failed)
        finally:self.cleanup_recovered(state,host)
    def test_apply_native_bad_json_preserves_document_state(self):
        state=self.create();host=DeferredHost();before=c.string_at(c.addressof(state),c.sizeof(State))
        self.assertEqual(self.service_apply(state,host,'invalid'),-1);self.assertEqual(c.string_at(c.addressof(state),c.sizeof(State)),before);self.assertEqual(host.document,units('old'));host.verify(self)
    def test_apply_native_failed_plain_write_does_not_trigger_guard_reset(self):
        state=self.create();host=DeferredHost();host.fail_write=True
        try:
            self.assertEqual(self.service_apply(state,host,{'commits':[{'text':'failure'}]}),1)
            self.assertEqual(host.document,units('old'));self.assertFalse(state.key_failed);self.assertFalse(state.transport);self.assertFalse(state.composition)
        finally:self.cleanup_recovered(state,host)
    def test_apply_native_last_caller_release_retained_through_end_and_hide(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host);address=c.addressof(state);events=[];dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):events.append(('destroy',state.composition,state.last_converting));self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        def ended():events.append('end');release(root);self.assertEqual(self.live(),self.before+1)
        host.on_end=ended
        self.assertEqual(self.service_apply(state,host,{'active':False},lifetime=root),1)
        self.assertEqual(events,['end',('destroy',None,False)]);host.verify(self)
    def test_apply_native_reset_failure_notice_pins_outcome_updates(self):
        state=self.create();host=DeferredHost();address=c.addressof(state);events=[];dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):events.append(('destroy',state.key_failed,state.consumed));self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        @self.checked(TOGGLE)
        def notice(owner):self.assertEqual(owner,123);events.append('invalid');release(root);self.assertEqual(self.live(),self.before+1)
        result,request=self.fixture_recovery(state,'invalid',lambda worker:self.service_apply(state,host,{'commits':[{'text':'wrong','deleteBefore':3,'expect':'bad'}]},worker=worker,notice=notice,lifetime=root))
        self.assertEqual(result,0);self.assertEqual(request['op'],'reset');self.assertEqual(events,['invalid',('destroy',True,False)]);host.verify(self)
    def test_apply_native_invalid_arguments_preserve_state(self):
        state=self.create();host=DeferredHost();address=c.addressof(state);before=c.string_at(address,c.sizeof(State));life=c.byref(host.context);context=c.byref(host.context)
        for data,root,worker,ctx,raw in [(None,life,self.worker,context,b'{}'),(address,None,self.worker,context,b'{}'),(address,life,None,context,b'{}'),(address,life,self.worker,None,b'{}'),(address,life,self.worker,context,None)]:
            self.assertEqual(self.api.meltype_tsf_service_apply_reply(data,root,c.byref(host.sink),worker,ctx,7,raw,2,None,TOGGLE()),-1)
        self.assertEqual(c.string_at(address,c.sizeof(State)),before);host.verify(self)
    def native_shutdown(self,state,host,worker=None,notice=None,lifetime=None):
        return self.api.meltype_tsf_service_shutdown(c.addressof(state),lifetime or c.byref(host.context),
            c.byref(host.sink) if hasattr(host,'sink') else None,worker or self.worker,123,notice or CORE_NOTICE())
    def test_shutdown_native_exchange_applies_commit_then_discards_close_reply(self):
        state=self.create();host=DeferredHost();host.mode='sync';self.shutdown_composition(state,host);self.stale_state(state)
        self.api.meltype_tsf_service_set_sid(c.addressof(state),'終了"😀'.encode())
        try:
            result,request=self.fixture_recovery(state,'{"active":true,"commits":[{"text":"確定😀"}]}',lambda worker:self.native_shutdown(state,host,worker=worker))
            self.assertEqual(result,0);self.assertEqual(request,{'op':'close','sid':'終了"😀'})
            self.assertEqual(host.document,units('確定😀'));self.assertEqual(host.ended,1);self.assertFalse(state.composition);self.assertFalse(state.transport)
            self.assertEqual((state.shutdown_phase,state.deactivated,state.client,self.stale_tries(state)),(2,True,0,0))
        finally:self.cleanup_recovered(state,host)
    def test_shutdown_native_close_without_composition_never_inserts_reply(self):
        state=self.create();host=DeferredHost();state.client=17
        result,request=self.fixture_recovery(state,'{"commits":[{"text":"duplicate"}]}',lambda worker:self.native_shutdown(state,host,worker=worker))
        self.assertEqual(result,0);self.assertEqual(request,{'op':'close','sid':''});self.assertEqual(host.document,units('old'));self.assertEqual(host.requests,[]);host.verify(self)
    def test_shutdown_native_disabled_skips_exchange_and_repeated_call_is_inert(self):
        state=self.create();host=DeferredHost();state.disabled=True;state.client=17
        self.assertEqual(self.native_shutdown(state,host),0);self.assertFalse(state.transport);self.assertTrue(state.deactivated);self.assertEqual(state.shutdown_phase,2)
        before=c.string_at(c.addressof(state),c.sizeof(State));self.assertEqual(self.native_shutdown(state,host),0);self.assertEqual(c.string_at(c.addressof(state),c.sizeof(State)),before);host.verify(self)
    def test_shutdown_native_bad_commit_and_close_replies_report_and_preserve_text(self):
        state=self.create();host=DeferredHost();host.mode='sync';self.shutdown_composition(state,host);events=[]
        notice=CORE_NOTICE(lambda owner,op,value:events.append((owner,op,value)))
        try:
            result,request=self.fixture_recovery(state,'invalid',lambda worker:self.native_shutdown(state,host,worker=worker,notice=notice))
            self.assertEqual(result,0);self.assertEqual(request['op'],'close');self.assertEqual(events,[(123,1,0),(123,1,0)])
            self.assertEqual(host.document,units('old'));self.assertEqual(host.ended,1);self.assertEqual(state.shutdown_phase,2)
        finally:self.cleanup_recovered(state,host)
    def test_shutdown_native_deferred_sync_never_uses_stack_after_return(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host)
        try:
            result,request=self.fixture_recovery(state,'{}',lambda worker:self.native_shutdown(state,host,worker=worker))
            self.assertEqual(result,0);self.assertEqual(request['op'],'close');self.assertFalse(state.composition);self.assertEqual(state.shutdown_phase,2)
            self.assertEqual(host.ended,0);self.assertEqual(host.document,units('old'));self.assertEqual(len(host.pending),1)
            pending,host.pending=host.pending,[]
            for session in pending:
                try:self.assertEqual(invoke(session,3,HR,[U32],7),HR(0x80004004).value)
                finally:release(session)
        finally:self.cleanup_recovered(state,host)
    def test_shutdown_native_refused_edit_still_closes_and_releases_slots(self):
        state=self.create();host=DeferredHost();host.mode='reject';self.shutdown_composition(state,host)
        try:
            result,request=self.fixture_recovery(state,'{}',lambda worker:self.native_shutdown(state,host,worker=worker))
            self.assertEqual(result,0);self.assertEqual(request['op'],'close');self.assertFalse(state.composition);self.assertFalse(state.composition_context)
            self.assertEqual(host.document,units('old'));self.assertEqual(host.ended,0);self.assertFalse(state.transport)
        finally:self.cleanup_recovered(state,host)
    def test_shutdown_native_nested_stop_during_invalid_reply_is_inert(self):
        state=self.create();host=DeferredHost();host.mode='sync';self.shutdown_composition(state,host);events=[]
        @self.checked(CORE_NOTICE)
        def notice(owner,op,value):
            events.append(op);self.assertEqual(self.native_shutdown(state,host),0);self.assertEqual(state.shutdown_phase,1)
        try:
            self.fixture_recovery(state,'invalid',lambda worker:self.native_shutdown(state,host,worker=worker,notice=notice))
            self.assertEqual(events,[1,1]);self.assertEqual(state.shutdown_phase,2);self.assertEqual(host.ended,1)
        finally:self.cleanup_recovered(state,host)
    def test_shutdown_native_last_caller_release_during_commit_pins_all_cleanup(self):
        state=self.create();host=DeferredHost();host.mode='sync';self.shutdown_composition(state,host);address=c.addressof(state);events=[];dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):
            events.append(('destroy',state.shutdown_phase,state.deactivated,state.client,state.composition,state.transport));self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        def ended():events.append('end');release(root);self.assertEqual(self.live(),self.before+1)
        host.on_end=ended
        result,request=self.fixture_recovery(state,'{}',lambda worker:self.native_shutdown(state,host,worker=worker,lifetime=root))
        self.assertEqual(result,0);self.assertEqual(request['op'],'close');self.assertEqual(events,['end',('destroy',2,True,0,None,None)]);host.verify(self)
    def test_shutdown_native_superseding_activation_during_commit_stops_old_teardown(self):
        state=self.create();host=DeferredHost();host.mode='sync';self.shutdown_composition(state,host)
        def activated():state.focus_epoch+=1;state.shutdown_phase=0;state.client=99
        host.on_end=activated
        try:
            result,request=self.fixture_recovery(state,'{}',lambda worker:self.native_shutdown(state,host,worker=worker))
            self.assertEqual(result,0);self.assertEqual(request['op'],'commit');self.assertEqual((state.shutdown_phase,state.client,state.deactivated),(0,99,False));self.assertTrue(state.transport)
        finally:self.cleanup_recovered(state,host)
    def test_shutdown_native_invalid_arguments_preserve_state(self):
        state=self.create();host=DeferredHost();address=c.addressof(state);before=c.string_at(address,c.sizeof(State));life=c.byref(host.context)
        for data,root,worker in [(None,life,self.worker),(address,None,self.worker),(address,life,None)]:
            self.assertEqual(self.api.meltype_tsf_service_shutdown(data,root,c.byref(host.sink),worker,None,CORE_NOTICE()),HR(0x80070057).value)
        self.assertEqual(c.string_at(address,c.sizeof(State)),before);host.verify(self)
    def dispatch_key(self,state,host,kind=0,key=65,worker=None,mode_notice=None,key_notice=None,sender=None,lifetime=None):
        return self.api.meltype_tsf_service_dispatch_key(c.addressof(state),lifetime or c.byref(host.context),
            c.byref(host.sink) if hasattr(host,'sink') else None,worker or self.worker,kind,c.byref(host.context),key,0,123,
            mode_notice or CORE_NOTICE(),key_notice or CORE_NOTICE(),sender or SENDER(lambda *_:2))
    def dispatch_host(self):
        host=ModeHost()
        def edit(_,client,session,flags,result):
            self.assertEqual(client,17);self.assertIn(flags,[3,7]);host.locks.append(flags)
            result[0]=invoke(session,3,HR,[U32],7);return 0
        host.bind(host.context_table,3,HR,[U32,P,U32,c.POINTER(HR)],edit)
        return host
    def test_dispatch_native_all_four_entries_reach_owned_key_edit_and_application(self):
        self.keyboard();state=self.create();self.key_state(state);state.client=17;state.server_active=True;host=self.dispatch_host();sent=[]
        def action(worker):
            self.assertEqual(self.dispatch_key(state,host,0,worker=worker),1)
            self.assertEqual(self.dispatch_key(state,host,1,worker=worker,sender=SENDER(lambda *_:sent.append(1) or 2)),1)
            self.assertEqual(self.dispatch_key(state,host,2,worker=worker),1);self.assertEqual(self.dispatch_key(state,host,3,worker=worker),1)
            self.assertEqual(self.dispatch_key(state,host,2,worker=worker),0);return 1
        try:
            _,request=self.fixture_recovery(state,'{"active":true,"consumed":true}',action)
            self.assertEqual((request['op'],request['vk'],request['ch']),('key',65,97));self.assertTrue(state.consumed);self.assertFalse(state.key_failed);self.assertFalse(state.own_edit);self.assertEqual(sent,[])
        finally:self.api.meltype_tsf_key_cache_clear(state.keys);host.verify(self)
    def test_dispatch_native_english_mode_passes_without_transport_or_sender(self):
        self.keyboard();state=self.create();self.key_state(state);state.open=False;host=self.dispatch_host();events=[];sent=[]
        self.assertEqual(self.dispatch_key(state,host,mode_notice=CORE_NOTICE(lambda owner,op,value:events.append((owner,op,value))),sender=SENDER(lambda *_:sent.append(1) or 2)),0)
        self.assertEqual(events,[(123,6,1)]);self.assertEqual(sent,[]);self.assertFalse(state.transport);self.assertEqual(host.locks,[]);host.verify(self)
    def test_dispatch_native_prior_field_commit_applies_without_shell_callback(self):
        self.keyboard();state=self.create();self.key_state(state);old=DeferredHost();old.mode='sync';host=self.dispatch_host();self.shutdown_composition(state,old);state.server_active=True
        try:
            result,request=self.fixture_recovery(state,'{"active":true,"commits":[{"text":"確定😀"}]}',lambda worker:self.dispatch_key(state,host,worker=worker))
            self.assertEqual(result,1);self.assertEqual(request['op'],'commit');self.assertEqual(old.document,units('確定😀'));self.assertEqual(old.ended,1);self.assertFalse(state.composition)
        finally:self.api.meltype_tsf_key_cache_clear(state.keys);self.cleanup_recovered(state,old);host.verify(self)
    def test_dispatch_native_packet_async_commit_owns_worker_and_applies_after_return(self):
        self.keyboard();state=self.create();self.key_state(state);host=DeferredHost();self.shutdown_composition(state,host)
        def action(worker):
            path=c.create_string_buffer(worker);result=self.dispatch_key(state,host,key=0xe7,worker=path)
            self.assertEqual(result,0);self.assertEqual(len(host.pending),2);path.value=b'destroyed'
            session=host.pending.pop(0)
            try:self.assertEqual(invoke(session,3,HR,[U32],7),HR(0x80004004).value)
            finally:release(session)
            host.flush();return result
        try:
            _,request=self.fixture_recovery(state,'{"commits":[{"text":"native😀"}]}',action)
            self.assertEqual(request['op'],'commit');self.assertEqual(host.document,units('native😀'));self.assertEqual(host.ended,1);self.assertFalse(state.composition);self.assertFalse(state.keep_caret)
        finally:self.cleanup_recovered(state,host)
    def test_dispatch_native_packet_async_invalid_reply_notice_retains_real_owner(self):
        self.keyboard();state=self.create();self.key_state(state);host=DeferredHost();self.shutdown_composition(state,host);address=c.addressof(state);events=[];dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):events.append(('destroy',state.composition));self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        @self.checked(CORE_NOTICE)
        def notice(owner,op,value):self.assertEqual((owner,op,value),(123,4,0));events.append('invalid');self.assertEqual(self.live(),self.before+1)
        def action(worker):
            self.assertEqual(self.dispatch_key(state,host,key=0xe7,worker=worker,key_notice=notice,lifetime=root),0);release(root)
            session=host.pending.pop(0)
            try:self.assertEqual(invoke(session,3,HR,[U32],7),HR(0x80004004).value)
            finally:release(session)
            host.flush()
        self.fixture_recovery(state,'invalid',action)
        self.assertEqual(events,['invalid',('destroy',None)]);self.assertEqual(host.document,units('old'));host.verify(self)
    def test_dispatch_native_scope_notice_last_caller_release_pins_complete_key(self):
        self.keyboard();state=self.create();self.key_state(state);state.client=17;state.server_active=True;host=self.dispatch_host();address=c.addressof(state);events=[];dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):events.append(('destroy',state.consumed,state.own_edit));self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        @self.checked(CORE_NOTICE)
        def notice(owner,op,value):
            self.assertEqual(owner,123)
            if op==4:events.append('scope');release(root);self.assertEqual(self.live(),self.before+1)
        result,request=self.fixture_recovery(state,'{"active":true,"consumed":true}',lambda worker:self.dispatch_key(state,host,1,worker=worker,mode_notice=notice,lifetime=root))
        self.assertEqual(result,1);self.assertEqual(request['op'],'key');self.assertEqual(events,['scope',('destroy',True,False)]);host.verify(self)
    def test_dispatch_native_invalid_arguments_preserve_state(self):
        state=self.create();self.key_state(state);host=self.dispatch_host();address=c.addressof(state);before=c.string_at(address,c.sizeof(State));life=c.byref(host.context);sender=SENDER(lambda *_:2)
        for data,root,worker in [(None,life,self.worker),(address,None,self.worker),(address,life,None)]:
            self.assertEqual(self.api.meltype_tsf_service_dispatch_key(data,root,None,worker,0,c.byref(host.context),65,0,None,CORE_NOTICE(),CORE_NOTICE(),sender),0)
        self.assertEqual(self.dispatch_key(state,host,kind=4),0);self.assertEqual(c.string_at(address,c.sizeof(State)),before);host.verify(self)
    def notify_focus(self,state,host,event=-1,worker=None,notice=None,lifetime=None):
        return self.api.meltype_tsf_service_focus_native(c.addressof(state),lifetime or c.byref(host.context),c.byref(host.sink),None,worker or self.worker,None,event,123,notice or CORE_NOTICE())
    def notify_open(self,state,host,worker=None,notice=None,lifetime=None):
        return self.api.meltype_tsf_service_opened_native(c.addressof(state),lifetime or c.byref(host.context),c.byref(host.sink),worker or self.worker,c.byref(OPEN),123,notice or CORE_NOTICE())
    def test_notifications_document_focus_native_commit_preserves_caret(self):
        state=self.create();host=DeferredHost();host.mode='sync';self.shutdown_composition(state,host);host.selection=(0,0);state.disabled=True
        try:
            result,request=self.fixture_recovery(state,'{"commits":[{"text":"日本😀"}]}',lambda worker:self.notify_focus(state,host,worker=worker))
            self.assertEqual(result,0);self.assertTrue(request['moved']);self.assertEqual(host.document,units('日本😀'));self.assertEqual(host.selection,(0,0));self.assertFalse(state.composition)
        finally:self.cleanup_recovered(state,host)
    def test_notifications_open_close_native_commit_moves_caret(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host);manager=OpenHost();state.manager=c.addressof(manager.context);manager.integers[bytes(OPEN)]=0;events=[];host.selection=(0,0)
        def action(worker):
            result=self.notify_open(state,host,worker=worker,notice=CORE_NOTICE(lambda owner,op,value:events.append((owner,op,value))))
            self.assertEqual(events,[(123,11,0)]);self.assertTrue(host.pending);host.flush();return result
        try:
            _,request=self.fixture_recovery(state,'{"commits":[{"text":"確定😀"}]}',action)
            self.assertNotIn('moved',request);self.assertEqual(host.document,units('確定😀'));self.assertEqual(host.selection,(4,4));self.assertFalse(state.open);self.assertFalse(state.composition)
        finally:state.manager=None;self.cleanup_recovered(state,host);manager.verify(self)
    def test_notifications_async_commit_keeps_real_notice_after_open_returns(self):
        state=self.create();host=DeferredHost();self.shutdown_composition(state,host);manager=OpenHost();state.manager=c.addressof(manager.context);manager.integers[bytes(OPEN)]=0;events=[]
        @self.checked(CORE_NOTICE)
        def notice(owner,op,value):self.assertEqual(owner,123);events.append(op)
        def action(worker):
            path=c.create_string_buffer(worker);result=self.notify_open(state,host,worker=path,notice=notice);path.value=b'destroyed';host.flush();return result
        try:
            self.fixture_recovery(state,'invalid',action);self.assertEqual(events,[11,1]);self.assertEqual(host.document,units('old'));self.assertEqual(host.ended,1)
        finally:state.manager=None;self.cleanup_recovered(state,host);manager.verify(self)
    def test_notifications_edited_own_mark_consumed_without_query(self):
        state=self.create();host=EventHost();state.own_edit=True
        self.assertEqual(self.api.meltype_tsf_service_edited_native(c.addressof(state),c.byref(host.context),c.byref(host.sink),self.worker,c.byref(host.context),c.byref(host.record),7,None,CORE_NOTICE()),1)
        self.assertFalse(state.own_edit);self.assertEqual(host.record_reads,0);self.assertFalse(state.transport);host.verify(self)
    def test_notifications_focus_invalid_reply_last_caller_release_pins_cleanup(self):
        state=self.create();host=DeferredHost();host.mode='sync';self.shutdown_composition(state,host);state.disabled=True;address=c.addressof(state);events=[];dispatch=DISPATCH(lambda *_:0)
        @self.checked(DESTROY)
        def destroyed(owner):events.append(('destroy',state.composition,state.caret_moved));self.api.meltype_tsf_service_state_destroy(owner)
        root=self.api.meltype_tsf_service_com_create(address,dispatch,destroyed);self.assertTrue(root);self.owned.remove(address)
        @self.checked(CORE_NOTICE)
        def notice(owner,op,value):self.assertEqual((owner,op),(123,1));events.append('invalid');release(root);self.assertEqual(self.live(),self.before+1)
        self.fixture_recovery(state,'invalid',lambda worker:self.notify_focus(state,host,worker=worker,notice=notice,lifetime=root))
        self.assertEqual(events,['invalid',('destroy',None,True)]);host.verify(self)
    def test_notifications_edited_native_query_uses_owned_exchange_and_updates_mode(self):
        state=self.create();state.client=17;state.code_app=True;state.server_active=True;self.key_state(state);host=ModeHost();record=EventHost();events=[]
        try:
            result,request=self.fixture_recovery(state,'{"active":true,"code":true,"english":true}',lambda worker:self.api.meltype_tsf_service_edited_native(c.addressof(state),c.byref(host.context),None,worker,c.byref(host.context),c.byref(record.record),7,123,CORE_NOTICE(lambda owner,op,value:events.append((owner,op,value)))))
            self.assertEqual(result,1);self.assertEqual(request['op'],'mode');self.assertTrue(state.code_app);self.assertTrue(state.code_english);self.assertTrue(state.caret_moved)
            self.assertIn((123,2,1),events)
        finally:host.verify(self);record.verify(self)
    def test_notifications_invalid_worker_preserves_state(self):
        state=self.create();host=DeferredHost();before=c.string_at(c.addressof(state),c.sizeof(State));life=c.byref(host.context);sink=c.byref(host.sink);address=c.addressof(state)
        self.assertEqual(self.api.meltype_tsf_service_focus_native(address,life,sink,None,None,None,0,None,CORE_NOTICE()),HR(0x80070057).value)
        self.assertEqual(self.api.meltype_tsf_service_opened_native(address,life,sink,None,c.byref(OPEN),None,CORE_NOTICE()),HR(0x80070057).value)
        self.assertEqual(self.api.meltype_tsf_service_edited_native(address,life,sink,None,life,None,7,None,CORE_NOTICE()),0)
        self.assertEqual(c.string_at(address,c.sizeof(State)),before);host.verify(self)
    def activate_owned_ui(self,state,host,flags=0,worker=None,notice=None,ui_notice=None,lifetime=None):
        sinks=(P*5)(*[c.addressof(host.sink)]*5)
        @self.checked(CATEGORY)
        def category(owner):self.assertEqual(owner,123);host.count('category_refs',1);return c.addressof(host.category)
        notice=notice or CORE_NOTICE();ui_notice=ui_notice or CORE_NOTICE();self.callbacks.extend([category,notice,ui_notice])
        return self.api.meltype_tsf_service_activate_native(c.addressof(state),lifetime or c.byref(host.context),worker or self.worker,c.byref(host.context),17,flags,sinks,123,notice,ui_notice,category)
    def test_owned_ui_activation_bar_reads_native_state_after_return(self):
        state=self.create();host=StartupHost()
        try:
            self.assertEqual(self.activate_owned_ui(state,host),0);bar=state.langbar;self.assertTrue(bar)
            self.assertEqual(self.bar_text(bar),'あ');state.code_app=True;state.code_english=True;self.assertEqual(self.bar_text(bar),'A');state.code_app=False;self.assertEqual(self.bar_text(bar),'A')
            state.code_english=False;self.assertEqual(self.bar_text(bar),'あ')
        finally:self.finish_activation(state,host)
    def test_owned_ui_activation_bar_toggle_calls_native_mode_after_return(self):
        state=self.create();host=StartupHost();host.null_focus=True
        try:
            self.assertEqual(self.activate_owned_ui(state,host),0);before=len(host.writes_mode);self.click_bar(state.langbar)
            self.assertEqual(host.writes_mode[before:],[(bytes(OPEN),17,0)])
        finally:self.finish_activation(state,host)
    def test_owned_ui_activation_secure_mode_and_reactivation_replace_adapters(self):
        state=self.create();host=StartupHost()
        try:
            self.assertEqual(self.activate_owned_ui(state,host,flags=2),0);self.assertFalse(state.langbar);self.assertTrue(state.candidates);self.assertFalse(state.transport)
            self.finish_activation(state,host)
            self.assertEqual(self.activate_owned_ui(state,host),0);self.assertTrue(state.langbar);self.assertEqual(self.bar_text(state.langbar),'あ')
        finally:self.finish_activation(state,host)
    def test_owned_ui_retained_bar_is_detached_after_state_destruction(self):
        state=self.create();host=StartupHost();address=c.addressof(state)
        self.assertEqual(self.activate_owned_ui(state,host),0);bar=state.langbar;invoke(bar,1,U32,[])
        try:
            self.finish_activation(state,host);self.owned.remove(address);self.api.meltype_tsf_service_state_destroy(address)
            self.click_bar(bar);self.assertEqual(self.bar_text(bar),'あ');self.assertEqual(self.live(),self.before)
        finally:release(bar)
    def test_owned_ui_activation_native_candidate_click_sends_selection(self):
        state=self.create();host=StartupHost();document=DeferredHost();document.mode='sync'
        def action(worker):
            path=c.create_string_buffer(worker);self.assertEqual(self.activate_owned_ui(state,host,worker=path),0);path.value=b'destroyed'
            self.shutdown_composition(state,document);self.assertEqual(self.show_candidates(state),1);self.click_candidate(self.candidate_windows()[0]);return 1
        try:
            result,request=self.fixture_recovery(state,'{"active":true}',action)
            self.assertEqual(result,1);self.assertEqual(request['op'],'select');self.assertEqual(request['index'],1);self.assertTrue(request['prediction']);self.assertFalse(state.composition)
        finally:self.clear_owned_pair(state);document.verify(self);self.finish_activation(state,host)
    def test_owned_ui_activation_invalid_arguments_preserve_state(self):
        state=self.create();host=StartupHost();address=c.addressof(state);before=c.string_at(address,c.sizeof(State));sinks=(P*5)(*[c.addressof(host.sink)]*5)
        for data,life,worker in [(None,c.byref(host.context),self.worker),(address,None,self.worker),(address,c.byref(host.context),None)]:
            self.assertEqual(self.api.meltype_tsf_service_activate_native(data,life,worker,c.byref(host.context),17,0,sinks,None,CORE_NOTICE(),CORE_NOTICE(),CATEGORY()),HR(0x80070057).value)
        sinks[4]=None
        self.assertEqual(self.api.meltype_tsf_service_activate_native(address,c.byref(host.context),self.worker,c.byref(host.context),17,0,sinks,None,CORE_NOTICE(),CORE_NOTICE(),CATEGORY()),HR(0x80070057).value)
        self.assertEqual(c.string_at(address,c.sizeof(State)),before);host.verify(self)
    def routed_root(self,state):
        dispatch=DISPATCH(lambda *_:0);destroy=DESTROY(lambda *_:None);self.callbacks.extend([dispatch,destroy])
        root=self.api.meltype_tsf_service_com_create(c.addressof(state),dispatch,destroy);self.assertTrue(root);self.addCleanup(release,root);return root
    def route(self,state,root,call,worker=None,notice=None):
        return self.api.meltype_tsf_service_route(c.addressof(state),root,worker or self.worker,c.byref(call),123,notice or CORE_NOTICE(),CORE_NOTICE(),CORE_NOTICE(),CORE_NOTICE())
    def test_route_unknown_and_null_key_output_preserve_state(self):
        state=self.create();root=self.routed_root(state);before=c.string_at(c.addressof(state),c.sizeof(State))
        self.assertEqual(self.route(state,root,Call(999)),HR(0x80004001).value)
        for op in [9,10,11,12]:self.assertEqual(self.route(state,root,Call(op)),HR(0x80070057).value)
        self.assertEqual(c.string_at(c.addressof(state),c.sizeof(State)),before)
    def test_route_key_up_clamps_pointer_sized_keys_and_writes_bool(self):
        state=self.create();self.key_state(state);root=self.routed_root(state);out=HR(7)
        for op in [10,12]:
            self.assertEqual(self.route(state,root,Call(op,None,c.addressof(out),None,c.c_size_t(-1).value,-1)),0);self.assertEqual(out.value,0)
    def test_route_focus_and_edit_notifications_use_native_paths(self):
        state=self.create();self.key_state(state);root=self.routed_root(state);host=EventHost();state.direct_word=True
        self.assertEqual(self.route(state,root,Call(8,None,None,None,0,0)),0)
        state.own_edit=True
        self.assertEqual(self.route(state,root,Call(18,c.addressof(host.context),c.addressof(host.record),None,7,0)),0)
        self.assertFalse(state.own_edit);self.assertEqual(host.record_reads,0);host.verify(self)
    def test_route_shutdown_disabled_and_repeated_call(self):
        state=self.create();state.disabled=True;root=self.routed_root(state)
        self.assertEqual(self.route(state,root,Call(2)),0);self.assertEqual(state.shutdown_phase,2);self.assertTrue(state.deactivated)
        self.assertEqual(self.route(state,root,Call(2)),0);self.assertFalse(state.transport)
    def test_route_activate_uses_five_native_sink_interfaces(self):
        state=self.create();root=self.routed_root(state);host=StartupHost()
        try:
            # Secure mode needs no real category/IME registration; atom creation
            # may fail normally. Only the private manager receives subscriptions.
            self.assertEqual(self.route(state,root,Call(1,c.addressof(host.context),None,None,17,2)),0)
            self.assertEqual(state.client,17);self.assertTrue(state.disabled);self.assertTrue(state.candidates);self.assertFalse(state.langbar)
            self.assertEqual(self.route(state,root,Call(2)),0);host.clear_bars();host.verify(self)
        finally:
            if state.shutdown_phase!=2:self.route(state,root,Call(2))
    def test_route_invalid_arguments_preserve_state(self):
        state=self.create();root=self.routed_root(state);call=Call(999);before=c.string_at(c.addressof(state),c.sizeof(State));addr=c.addressof(state)
        for data,life,worker,value in [(None,root,self.worker,c.byref(call)),(addr,None,self.worker,c.byref(call)),(addr,root,None,c.byref(call)),(addr,root,self.worker,None)]:
            self.assertEqual(self.api.meltype_tsf_service_route(data,life,worker,value,None,CORE_NOTICE(),CORE_NOTICE(),CORE_NOTICE(),CORE_NOTICE()),HR(0x80070057).value)
        self.assertEqual(c.string_at(addr,c.sizeof(State)),before)
    def create_native_object(self,iid=None,worker=None,lock=None,notice=None):
        out=P(999);iid=iid or IDS[2];lock=lock or OBJECT_LOCK();notice=notice or CORE_NOTICE()
        self.callbacks.extend([lock,notice])
        status=self.api.meltype_tsf_service_object_create(worker or self.worker,c.byref(iid),c.byref(out),123,CORE_NOTICE(),CORE_NOTICE(),notice,CORE_NOTICE(),lock)
        return status,out.value
    def test_object_native_owns_state_and_balances_shell_pin_on_final_release(self):
        events=[]
        @self.checked(OBJECT_LOCK)
        def lock(value):events.append((value,self.live()))
        status,root=self.create_native_object(lock=lock)
        self.assertEqual(status,0);self.assertTrue(root);self.assertEqual(events,[(1,self.before+1)])
        self.assertEqual(invoke(root,1,U32,[]),2);self.assertEqual(release(root),1);self.assertEqual(self.live(),self.before+1)
        self.assertEqual(release(root),0);self.assertEqual(events,[(1,self.before+1),(0,self.before)]);self.assertEqual(self.live(),self.before)
    def test_object_native_all_interfaces_share_identity_and_no_extra_owner_refs(self):
        status,root=self.create_native_object();self.assertEqual(status,0)
        try:
            for iid in IDS:
                out=P();self.assertEqual(invoke(root,0,HR,[P,P],c.byref(iid),c.byref(out)),0)
                identity=P();self.assertEqual(invoke(out,0,HR,[P,P],c.byref(IDS[0]),c.byref(identity)),0)
                self.assertEqual(identity.value,root);release(identity);release(out)
        finally:self.assertEqual(release(root),0)
    def test_object_native_unsupported_iid_releases_state_and_pin(self):
        events=[];iid=Guid.parse('00000000-0000-0000-0000-000000000001')
        status,root=self.create_native_object(iid=iid,lock=OBJECT_LOCK(lambda value:events.append(value)))
        self.assertEqual(status,HR(0x80004002).value);self.assertFalse(root);self.assertEqual(events,[1,0]);self.assertEqual(self.live(),self.before)
    def test_object_native_secure_public_activation_and_shutdown_balance_state(self):
        events=[];host=StartupHost();status,root=self.create_native_object(notice=CORE_NOTICE(lambda owner,op,value:events.append((owner,op,value))))
        self.assertEqual(status,0)
        try:
            self.assertEqual(invoke(root,5,HR,[P,U32,U32],c.byref(host.context),17,2),0)
            self.assertEqual(events,[(123,0,1)]);self.assertGreater(host.context_refs,1)
            self.assertEqual(invoke(root,4,HR,[]),0);self.assertEqual(invoke(root,4,HR,[]),0);host.clear_bars();host.verify(self)
        finally:release(root)
        self.assertEqual(self.live(),self.before)
    def test_object_native_copies_worker_and_invalid_arguments_clear_out(self):
        path=c.create_string_buffer(self.worker);status,root=self.create_native_object(worker=path);self.assertEqual(status,0);path.value=b'destroyed'
        try:self.assertEqual(invoke(root,4,HR,[]),0)
        finally:release(root)
        out=P(123);lock=OBJECT_LOCK()
        for worker,iid in [(None,c.byref(IDS[0])),(self.worker,None)]:
            self.assertEqual(self.api.meltype_tsf_service_object_create(worker,iid,c.byref(out),None,CORE_NOTICE(),CORE_NOTICE(),CORE_NOTICE(),CORE_NOTICE(),lock),HR(0x80070057).value);self.assertFalse(out.value)
        self.assertEqual(self.api.meltype_tsf_service_object_create(self.worker,c.byref(IDS[0]),None,None,CORE_NOTICE(),CORE_NOTICE(),CORE_NOTICE(),CORE_NOTICE(),lock),HR(0x80070057).value)
        self.assertEqual(self.live(),self.before)
if __name__=='__main__':unittest.main(argv=[sys.argv[0]],verbosity=2,failfast=True)

