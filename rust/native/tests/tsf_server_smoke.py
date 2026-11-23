# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Standalone Rust DLL exports, resource icon, locks, identity and private logs."""
import ctypes as c
import os
from pathlib import Path
import sys
import tempfile
import time
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parent))
from tsf_com_smoke import Guid,HR,P,U32,invoke,release,UNKNOWN
from tsf_com_smoke import RUST_MODULES,PROVIDER
from tsf_edit_session_smoke import ACTION,DESTROY,REQUEST,ASYNC
from tsf_startup_smoke import StartupHost
CLSID=Guid.parse('c540baf6-88b1-5b30-b524-2c8702b034fc')
FACTORY=Guid.parse('00000001-0000-0000-c000-000000000046')
PROCESSOR=Guid.parse('6e4e2102-f9cd-433d-b496-303ce03a6507')
class ServerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.path=Path(sys.argv[1]).resolve();cls.dll=c.WinDLL(str(cls.path));cls.native=c.CDLL(str(cls.path))
        RUST_MODULES.add(cls.path)
        cls.get=cls.dll.DllGetClassObject;cls.get.argtypes=[P,P,c.POINTER(P)];cls.get.restype=HR
        cls.dll.DllCanUnloadNow.restype=HR
        cls.native.meltype_tsf_service_state_live_count.restype=U32
        # Resolve registration exports without invoking them on the host OS.
        assert cls.dll.DllRegisterServer and cls.dll.DllUnregisterServer
        cls.directory=tempfile.TemporaryDirectory(prefix='meltype-rust-server-')
        cls.previous={key:os.environ.get(key) for key in ['LOCALAPPDATA','MELTYPE_TIP_LOG']}
        os.environ['LOCALAPPDATA']=cls.directory.name;os.environ['MELTYPE_TIP_LOG']='1'
        (Path(cls.directory.name)/'Meltype-FLASH').mkdir()
    @classmethod
    def tearDownClass(cls):
        for key,value in cls.previous.items():
            if value is None:os.environ.pop(key,None)
            else:os.environ[key]=value
        cls.directory.cleanup()
    def setUp(self):self.assertEqual(self.dll.DllCanUnloadNow(),0);self.assertEqual(self.native.meltype_tsf_service_state_live_count(),0)
    def tearDown(self):self.setUp()
    def factory(self):
        out=P();self.assertEqual(self.get(c.byref(CLSID),c.byref(FACTORY),c.byref(out)),0);return out.value
    def create(self,factory):
        out=P();self.assertEqual(invoke(factory,3,HR,[P,P,P],None,c.byref(PROCESSOR),c.byref(out)),0);return out.value
    def test_factory_service_identity_lock_and_final_cleanup(self):
        factory=self.factory();root=None
        try:
            self.assertEqual(self.dll.DllCanUnloadNow(),1)
            self.assertEqual(invoke(factory,4,HR,[HR],1),0)
            root=self.create(factory);self.assertEqual(self.native.meltype_tsf_service_state_live_count(),1)
            identity=P();self.assertEqual(invoke(root,0,HR,[P,P],c.byref(UNKNOWN),c.byref(identity)),0)
            self.assertEqual(identity.value,root);release(identity)
            self.assertEqual(invoke(factory,4,HR,[HR],0),0)
        finally:
            if root:release(root)
            release(factory)
    def test_candidate_classes_retire_before_unload_and_can_register_again(self):
        self.native.meltype_tsf_candidate_create.argtypes=[P,P];self.native.meltype_tsf_candidate_create.restype=P
        self.native.meltype_tsf_candidate_destroy.argtypes=[P]
        user=c.WinDLL('user32');user.GetClassInfoExW.argtypes=[P,c.c_wchar_p,P]
        def registered():
            buffer=c.create_string_buffer(80 if c.sizeof(P)==8 else 48);c.cast(buffer,c.POINTER(U32))[0]=len(buffer)
            return bool(user.GetClassInfoExW(self.dll._handle,'Meltype-flashCandidateWindow',buffer))
        for _ in range(2):
            token=self.native.meltype_tsf_candidate_create(None,None);self.assertTrue(token)
            try:self.assertTrue(registered());self.assertEqual(self.dll.DllCanUnloadNow(),1)
            finally:self.native.meltype_tsf_candidate_destroy(token)
            self.assertEqual(self.dll.DllCanUnloadNow(),0);self.assertFalse(registered())
    def test_provider_children_and_cloned_enumerator_pin_dll_after_service_release(self):
        factory=self.factory();out=P()
        self.assertEqual(invoke(factory,3,HR,[P,P,P],None,c.byref(PROVIDER),c.byref(out)),0)
        provider=out.value;release(factory);enumerator=P();clone=P();info=P();fetched=U32()
        self.assertEqual(invoke(provider,3,HR,[P],c.byref(enumerator)),0)
        self.assertEqual(invoke(enumerator,3,HR,[P],c.byref(clone)),0)
        self.assertEqual(invoke(enumerator,4,HR,[U32,P,P],1,c.byref(info),c.byref(fetched)),0)
        release(provider);release(enumerator)
        self.assertEqual(self.native.meltype_tsf_service_state_live_count(),0)
        self.assertEqual(self.dll.DllCanUnloadNow(),1)
        release(clone);self.assertEqual(self.dll.DllCanUnloadNow(),1)
        release(info);self.assertEqual(self.dll.DllCanUnloadNow(),0)
    def test_detached_language_bar_pins_dll_until_final_release(self):
        self.native.meltype_tsf_create_langbar.argtypes=[P,P,P];self.native.meltype_tsf_create_langbar.restype=P
        self.native.meltype_tsf_langbar_detach.argtypes=[P]
        bar=self.native.meltype_tsf_create_langbar(None,None,None);self.assertTrue(bar)
        try:
            self.native.meltype_tsf_langbar_detach(bar)
            self.assertEqual(self.dll.DllCanUnloadNow(),1)
        finally:release(bar)
    def test_deferred_edit_session_pins_dll_through_destroy_callback(self):
        pending=[];destroyed=[]
        @REQUEST
        def request(this,client,session,flags,result):
            invoke(session,1,U32,[]);pending.append(session);result[0]=ASYNC;return 0
        @ACTION
        def action(context,cookie):return 0
        @DESTROY
        def destroy(context):destroyed.append(self.dll.DllCanUnloadNow())
        vtable=(P*4)(None,None,None,c.cast(request,P));context=(P*1)(c.cast(vtable,P))
        api=self.native.meltype_tsf_request_edit_session
        api.argtypes=[P,U32,U32,P,ACTION,DESTROY];api.restype=HR
        self.assertEqual(api(context,17,6,None,action,destroy),ASYNC)
        self.assertEqual(self.dll.DllCanUnloadNow(),1)
        self.assertEqual(invoke(pending[0],3,HR,[U32],77),0)
        release(pending.pop());self.assertEqual(destroyed,[1])
        self.assertEqual(self.dll.DllCanUnloadNow(),0)
    def test_factory_invalid_guid_and_aggregation_clear_output(self):
        bad=Guid.parse('00000000-0000-0000-0000-000000000001');out=P(123)
        self.assertEqual(self.get(c.byref(bad),c.byref(FACTORY),c.byref(out)),HR(0x80040111).value);self.assertFalse(out.value)
        self.assertEqual(self.get(c.byref(CLSID),c.byref(bad),c.byref(out)),HR(0x80004002).value);self.assertFalse(out.value)
        factory=self.factory()
        try:
            self.assertEqual(invoke(factory,3,HR,[P,P,P],123,c.byref(PROCESSOR),c.byref(out)),HR(0x80040110).value);self.assertFalse(out.value)
        finally:release(factory)
    def test_icon_resource_101_is_preserved(self):
        user=c.WinDLL('user32');user.LoadImageW.argtypes=[P,P,U32,HR,HR,U32];user.LoadImageW.restype=P;user.DestroyIcon.argtypes=[P]
        icon=user.LoadImageW(self.dll._handle,101,1,32,32,0)
        self.assertTrue(icon);self.assertTrue(user.DestroyIcon(icon))
    def test_secure_activation_writes_only_private_log_and_releases_host(self):
        host=StartupHost();factory=self.factory();root=self.create(factory);release(factory)
        try:
            self.assertEqual(invoke(root,5,HR,[P,U32,U32],c.byref(host.context),17,2),0)
            self.assertEqual(invoke(root,4,HR,[]),0);host.clear_bars();host.verify(self)
            path=Path(self.directory.name)/'Meltype-flash/tip.log';text=path.read_text(encoding='utf-8')
            self.assertIn('Activate (',text);self.assertIn('無効: セキュリティ保護された画面',text)
            self.assertRegex(text,r'\d{2}:\d{2}:\d{2}\.\d{3} \[')
        finally:release(root)
    def test_version_resource_matches_flash_1_0_1_dll_version(self):
        version=c.WinDLL('version');version.GetFileVersionInfoSizeW.argtypes=[c.c_wchar_p,c.POINTER(U32)];version.GetFileVersionInfoSizeW.restype=U32
        version.GetFileVersionInfoW.argtypes=[c.c_wchar_p,U32,U32,P];version.VerQueryValueW.argtypes=[P,c.c_wchar_p,c.POINTER(P),c.POINTER(U32)]
        unused=U32();size=version.GetFileVersionInfoSizeW(str(self.path),c.byref(unused));self.assertGreater(size,0)
        buffer=c.create_string_buffer(size);self.assertTrue(version.GetFileVersionInfoW(str(self.path),0,size,buffer))
        info=P();count=U32();self.assertTrue(version.VerQueryValueW(buffer,'\\',c.byref(info),c.byref(count)));self.assertGreaterEqual(count.value,52)
        fields=c.cast(info,c.POINTER(U32));self.assertEqual(fields[0],0xfeef04bd);self.assertEqual((fields[2],fields[3]),(0x10000,0x10000))
    def test_transport_supervisor_prevents_unload_until_os_thread_exit(self):
        api=self.native
        api.meltype_tip_transport_create.argtypes=[c.c_char_p];api.meltype_tip_transport_create.restype=P
        api.meltype_tip_transport_destroy.argtypes=[P]
        for _ in range(10):
            # Creating a transport starts its supervisor, without launching any
            # worker or writing a profile. It waits on its live request channel.
            # The lazy constructor validates an existing absolute path. No
            # request is sent, so this test DLL is never executed as a worker.
            handle=api.meltype_tip_transport_create(str(self.path).encode('utf-8'))
            self.assertTrue(handle)
            try:self.assertEqual(self.dll.DllCanUnloadNow(),1)
            finally:api.meltype_tip_transport_destroy(handle)
            deadline=time.monotonic()+5
            while self.dll.DllCanUnloadNow()!=0 and time.monotonic()<deadline:time.sleep(.001)
            self.assertEqual(self.dll.DllCanUnloadNow(),0)
if __name__=='__main__':unittest.main(argv=[sys.argv[0]],verbosity=2,failfast=True)
