# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Call actual TSF COM vtables without registering/activating an input method.

Usage: python tsf_com_smoke.py path/to/meltype_native.dll [path/to/MeltypeTip.dll]
The optional second path verifies the shipped Rust COM server as well.
"""
import ctypes as c
import os
from pathlib import Path
import sys
import uuid
import threading

if os.name != "nt":
    raise SystemExit("This test requires Windows")

P = c.c_void_p
U32 = c.c_uint32
HR = c.c_int32
INVALID = -2147024809
NO_INTERFACE = -2147467262
NOT_IMPLEMENTED = -2147467263

class Guid(c.Structure):
    _fields_ = [("a", U32), ("b", c.c_uint16), ("c", c.c_uint16), ("d", c.c_ubyte * 8)]

    @classmethod
    def parse(cls, value):
        return cls.from_buffer_copy(uuid.UUID(value).bytes_le)

class Color(c.Structure):
    _fields_ = [("type", HR), ("value", U32)]

class Attribute(c.Structure):
    _fields_ = [("text", Color), ("background", Color), ("line", HR), ("bold", HR), ("line_color", Color), ("kind", HR)]

UNKNOWN = Guid.parse("00000000-0000-0000-c000-000000000046")
INFO = Guid.parse("70528852-2f26-4aea-8c96-215150578932")
ENUM = Guid.parse("7cef04d7-cb75-4e80-a7ab-5f5bc7d332de")
PROVIDER = Guid.parse("fee47777-163c-4769-996a-6e9c50ad8f54")
FACTORY = Guid.parse("00000001-0000-0000-c000-000000000046")
SERVICE = Guid.parse("c540baf6-88b1-5b30-b524-2c8702b034fc")
RUST_MODULES = set()
SPECS = [
    ("51987acc-d57a-5c81-ad25-5c1e8d83f258", "Meltype-flash: 入力中", 2, 0, 0),
    ("52a75cec-c221-5443-bb39-eb8d777c688f", "Meltype-flash: 変換した文節", 1, 0, 2),
    ("711d8fcb-cfbd-56d2-9a8e-672ee6ed74c1", "Meltype-flash: 選んでいる文節", 1, 1, 1),
]

def invoke(pointer, slot, result, args, *values):
    table = c.cast(pointer, c.POINTER(c.POINTER(P))).contents
    return c.WINFUNCTYPE(result, P, *args)(table[slot])(pointer, *values)

def release(pointer):
    return invoke(pointer, 2, U32, [])

def assert_rust_vtable(pointer):
    # A C++ implementation with the same behavior must not satisfy this port test.
    table = c.cast(pointer, c.POINTER(c.POINTER(P))).contents
    kernel = c.WinDLL("kernel32", use_last_error=True)
    kernel.GetModuleHandleExW.argtypes = [U32, P, c.POINTER(P)]
    kernel.GetModuleFileNameW.argtypes = [P, c.c_wchar_p, U32]
    module = P()
    assert kernel.GetModuleHandleExW(6, table[3], c.byref(module))
    path = c.create_unicode_buffer(32768)
    assert kernel.GetModuleFileNameW(module, path, len(path))
    actual=Path(path.value).resolve()
    assert actual in RUST_MODULES if RUST_MODULES else actual.name.lower()=="meltype_native.dll", path.value

def verify_info(pointer, index):
    assert_rust_vtable(pointer)
    expected, label, line, bold, kind = SPECS[index]
    identity = P()
    assert invoke(pointer, 0, HR, [c.POINTER(Guid), c.POINTER(P)], c.byref(UNKNOWN), c.byref(identity)) == 0
    assert identity.value == pointer
    release(identity)
    assert invoke(pointer, 0, HR, [c.POINTER(Guid), c.POINTER(P)], c.byref(INFO), c.byref(identity)) == 0
    release(identity)
    identity.value = 123
    assert invoke(pointer, 0, HR, [c.POINTER(Guid), c.POINTER(P)], c.byref(ENUM), c.byref(identity)) == NO_INTERFACE
    assert identity.value is None
    assert invoke(pointer, 0, HR, [c.POINTER(Guid), c.POINTER(P)], c.byref(INFO), None) == INVALID
    value = Guid()
    assert invoke(pointer, 3, HR, [c.POINTER(Guid)], c.byref(value)) == 0
    assert bytes(value) == uuid.UUID(expected).bytes_le
    assert invoke(pointer, 3, HR, [c.POINTER(Guid)], None) == INVALID
    text = P()
    assert invoke(pointer, 4, HR, [c.POINTER(P)], c.byref(text)) == 0
    allocator = c.WinDLL("oleaut32")
    allocator.SysStringLen.argtypes = [P]
    allocator.SysStringLen.restype = U32
    allocator.SysFreeString.argtypes = [P]
    try:
        assert c.wstring_at(text, allocator.SysStringLen(text)) == label
    finally:
        allocator.SysFreeString(text)
    assert invoke(pointer, 4, HR, [c.POINTER(P)], None) == INVALID
    attribute = Attribute()
    assert invoke(pointer, 5, HR, [c.POINTER(Attribute)], c.byref(attribute)) == 0
    assert (attribute.line, attribute.bold, attribute.kind) == (line, bold, kind)
    assert (attribute.text.type, attribute.background.type, attribute.line_color.type) == (0, 0, 0)
    assert invoke(pointer, 5, HR, [c.POINTER(Attribute)], None) == INVALID
    assert invoke(pointer, 6, HR, [c.POINTER(Attribute)], c.byref(attribute)) == NOT_IMPLEMENTED
    assert invoke(pointer, 7, HR, []) == 0

def verify_enum(pointer):
    assert_rust_vtable(pointer)
    identity = P()
    assert invoke(pointer, 0, HR, [c.POINTER(Guid), c.POINTER(P)], c.byref(ENUM), c.byref(identity)) == 0
    assert identity.value == pointer
    release(identity)
    values = (P * 4)()
    fetched = U32(77)
    assert invoke(pointer, 4, HR, [U32, c.POINTER(P), c.POINTER(U32)], 2, values, None) == INVALID
    assert invoke(pointer, 4, HR, [U32, c.POINTER(P), c.POINTER(U32)], 4, values, c.byref(fetched)) == 1
    assert fetched.value == 3
    for index in range(3):
        verify_info(values[index], index)
        assert release(values[index]) == 0
    assert invoke(pointer, 5, HR, []) == 0
    assert invoke(pointer, 6, HR, [U32], 1) == 0
    clone = P()
    assert invoke(pointer, 3, HR, [c.POINTER(P)], c.byref(clone)) == 0
    assert invoke(clone, 4, HR, [U32, c.POINTER(P), c.POINTER(U32)], 1, values, None) == 0
    verify_info(values[0], 1)
    release(values[0])
    release(clone)
    # Clone advances independently; skipping exactly to the end is S_OK.
    assert invoke(pointer, 6, HR, [U32], 2) == 0
    assert invoke(pointer, 6, HR, [U32], 0xffffffff) == 1
    assert invoke(pointer, 4, HR, [U32, c.POINTER(P), c.POINTER(U32)], 1, values, c.byref(fetched)) == 1
    assert fetched.value == 0
    assert release(pointer) == 0

def verify_langbar(native):
    class Point(c.Structure):
        _fields_ = [("x", HR), ("y", HR)]
    class ItemInfo(c.Structure):
        _fields_ = [("service", Guid), ("item", Guid), ("style", U32), ("sort", U32), ("description", c.c_wchar * 32)]
    read_type = c.CFUNCTYPE(HR, P)
    toggle_type = c.CFUNCTYPE(None, P)
    native.meltype_tsf_create_langbar.argtypes = [P, read_type, toggle_type]
    native.meltype_tsf_create_langbar.restype = P
    native.meltype_tsf_langbar_update.argtypes = [P]
    native.meltype_tsf_langbar_detach.argtypes = [P]
    mode = [True]
    reads = []
    updates = []
    pointer = None
    @read_type
    def read(context):
        reads.append(context)
        return int(mode[0])
    @toggle_type
    def toggle(context):
        assert context == 123
        mode[0] = not mode[0]
        native.meltype_tsf_langbar_update(pointer)
    pointer = native.meltype_tsf_create_langbar(123, read, toggle)
    assert pointer
    assert_rust_vtable(pointer)
    info = ItemInfo()
    assert invoke(pointer, 3, HR, [c.POINTER(ItemInfo)], c.byref(info)) == 0
    assert bytes(info.service) == uuid.UUID("c540baf6-88b1-5b30-b524-2c8702b034fc").bytes_le
    assert bytes(info.item) == uuid.UUID("2c77a81e-41cc-4178-a3a7-5f8a987568e6").bytes_le
    assert (info.description, info.style, info.sort) == ("Meltype-flash の入力モード", 65538, 0)
    assert invoke(pointer, 3, HR, [c.POINTER(ItemInfo)], None) == INVALID
    value = U32(123)
    assert invoke(pointer, 4, HR, [c.POINTER(U32)], c.byref(value)) == 0 and value.value == 0
    assert invoke(pointer, 5, HR, [HR], 1) == NOT_IMPLEMENTED
    allocator = c.WinDLL("oleaut32")
    allocator.SysFreeString.argtypes = [P]
    def get_text(slot=11):
        text = P()
        assert invoke(pointer, slot, HR, [c.POINTER(P)], c.byref(text)) == 0
        try:
            return c.wstring_at(text)
        finally:
            allocator.SysFreeString(text)
    assert get_text() == "あ"
    assert get_text(6) == "Meltype-flash: 日本語 (クリックで英数)"
    source = P()
    source_id = Guid.parse("4ea48a35-60ae-446f-8fd6-e6a8d82459f7")
    assert invoke(pointer, 0, HR, [c.POINTER(Guid), c.POINTER(P)], c.byref(source_id), c.byref(source)) == 0
    assert source.value != pointer
    assert_rust_vtable(source)
    identity = P()
    assert invoke(source, 0, HR, [c.POINTER(Guid), c.POINTER(P)], c.byref(UNKNOWN), c.byref(identity)) == 0
    assert identity.value == pointer
    release(identity)
    sink_id = Guid.parse("57dbe1a0-de25-11d2-afdd-00105a2799b5")
    refs = [1]
    unadvise_on_update = [False]
    class Sink(c.Structure):
        _fields_ = [("table", c.POINTER(P))]
    @c.WINFUNCTYPE(HR, P, c.POINTER(Guid), c.POINTER(P))
    def query(_this, iid, out):
        out[0] = None
        if bytes(iid.contents) not in [bytes(sink_id), bytes(UNKNOWN)]:
            return NO_INTERFACE
        refs[0] += 1
        out[0] = c.addressof(sink)
        return 0
    @c.WINFUNCTYPE(U32, P)
    def add(_this):
        refs[0] += 1
        return refs[0]
    @c.WINFUNCTYPE(U32, P)
    def drop(_this):
        refs[0] -= 1
        return refs[0]
    @c.WINFUNCTYPE(HR, P, U32)
    def update(_this, flags):
        updates.append(flags)
        if unadvise_on_update[0]:
            assert invoke(source, 4, HR, [U32], cookie.value) == 0
        return 0
    table = (P * 4)(*[c.cast(fn, P).value for fn in [query, add, drop, update]])
    sink = Sink(table)
    cookie = U32()
    advise_args = [c.POINTER(Guid), P, c.POINTER(U32)]
    assert invoke(source, 3, HR, advise_args, c.byref(sink_id), c.byref(sink), c.byref(cookie)) == 0
    assert cookie.value == 0x4d454c54 and refs[0] == 2
    other_cookie = U32()
    assert invoke(source, 3, HR, advise_args, c.byref(sink_id), c.byref(sink), c.byref(other_cookie)) == HR(0x80040201).value
    assert other_cookie.value == 0xffffffff
    assert invoke(source, 4, HR, [U32], 123) == HR(0x80040200).value
    assert invoke(pointer, 7, HR, [HR, Point, P], 0, Point(), None) == 0
    assert get_text() == "あ", "Unknown clicks must not toggle the input mode"
    assert invoke(pointer, 7, HR, [HR, Point, P], 2, Point(), None) == 0
    assert get_text() == "A"
    assert get_text(6) == "Meltype-flash: 英数 (クリックで日本語)"
    assert updates == [7] and refs[0] == 2
    # Foreign-thread calls cannot invoke callbacks into an STA text service.
    before = len(reads)
    result = []
    worker = threading.Thread(target=lambda: result.append(get_text()))
    worker.start(); worker.join(timeout=5)
    assert not worker.is_alive() and result == ["あ"] and len(reads) == before
    # Notification callbacks may unadvise themselves without deadlocking.
    unadvise_on_update[0] = True
    native.meltype_tsf_langbar_update(pointer)
    assert refs[0] == 1 and updates == [7, 7]
    assert invoke(source, 4, HR, [U32], cookie.value) == HR(0x80040200).value
    user = c.WinDLL("user32")
    user.DestroyIcon.argtypes = [P]
    user.GetGuiResources.argtypes = [P, U32]
    class IconInfo(c.Structure):
        _fields_ = [("is_icon", HR), ("x", U32), ("y", U32), ("mask", P), ("color", P)]
    class BitmapHeader(c.Structure):
        _fields_ = [("size", U32), ("width", HR), ("height", HR), ("planes", c.c_uint16), ("bits", c.c_uint16), ("compression", U32), ("image_size", U32), ("xppm", HR), ("yppm", HR), ("used", U32), ("important", U32)]
    user.GetIconInfo.argtypes = [P, c.POINTER(IconInfo)]
    user.GetDC.argtypes = [P]
    user.GetDC.restype = P
    user.ReleaseDC.argtypes = [P, P]
    gdi = c.WinDLL("gdi32")
    gdi.GetDIBits.argtypes = [P, P, U32, U32, P, c.POINTER(BitmapHeader), U32]
    gdi.DeleteObject.argtypes = [P]
    def icon_pixels(icon):
        info = IconInfo()
        assert user.GetIconInfo(icon, c.byref(info))
        dc = user.GetDC(None)
        try:
            header = BitmapHeader()
            header.size = c.sizeof(header)
            assert gdi.GetDIBits(dc, info.color, 0, 0, None, c.byref(header), 0)
            width, height = header.width, abs(header.height)
            header.height = -height
            header.planes, header.bits, header.compression = 1, 32, 0
            pixels = (c.c_ubyte * (width * height * 4))()
            assert gdi.GetDIBits(dc, info.color, 0, height, pixels, c.byref(header), 0) == height
            data = bytes(pixels)
            assert any(data[3::4]), "mode icon must contain visible glyph pixels"
            assert any(a == 0 for a in data[3::4]), "mode icon background must be transparent"
            assert all(b <= a and g <= a and r <= a for b, g, r, a in zip(data[0::4], data[1::4], data[2::4], data[3::4])), "icon must use premultiplied alpha"
            return width, height, data
        finally:
            user.ReleaseDC(None, dc)
            gdi.DeleteObject(info.color)
            gdi.DeleteObject(info.mask)
    kernel = c.WinDLL("kernel32")
    kernel.GetCurrentProcess.restype = P
    def draw_icon(inspect=False):
        icon = P()
        assert invoke(pointer, 10, HR, [c.POINTER(P)], c.byref(icon)) == 0 and icon.value
        try:
            return icon_pixels(icon) if inspect else None
        finally:
            assert user.DestroyIcon(icon)
    latin_icon = draw_icon(inspect=True)
    for _ in range(5): draw_icon()
    resources = user.GetGuiResources(kernel.GetCurrentProcess(), 0)
    for _ in range(100): draw_icon()
    assert user.GetGuiResources(kernel.GetCurrentProcess(), 0) <= resources + 2
    native.meltype_tsf_langbar_detach(pointer)
    before = len(reads)
    assert get_text() == "あ"
    assert invoke(pointer, 7, HR, [HR, Point, P], 0, Point(), None) == 0
    assert not mode[0] and len(reads) == before
    japanese_icon = draw_icon(inspect=True)
    assert latin_icon[:2] == japanese_icon[:2] and latin_icon[2] != japanese_icon[2]
    assert release(source) == 1
    assert release(pointer) == 0
    assert refs[0] == 1
    print("TSF language bar: Rust vtables, COM identity, mode click/reentrant notifications, sink ownership/unadvise, STA callbacks, icon resource cleanup and detach passed")
    return japanese_icon, latin_icon

def verify_factory(native):
    create_type = c.CFUNCTYPE(HR, c.POINTER(Guid), c.POINTER(P))
    lock_type = c.CFUNCTYPE(None, HR)
    pins = [0]
    calls = []
    @create_type
    def create(iid, out):
        calls.append(bytes(iid.contents))
        out[0] = None
        return NO_INTERFACE
    @lock_type
    def lock(value):
        pins[0] += 1 if value else -1
    native.meltype_tsf_get_class_object.argtypes = [c.POINTER(Guid), c.POINTER(Guid), c.POINTER(P), create_type, lock_type]
    native.meltype_tsf_get_class_object.restype = HR
    def get(clsid=SERVICE, iid=FACTORY):
        out=P(123)
        result=native.meltype_tsf_get_class_object(c.byref(clsid),c.byref(iid),c.byref(out),create,lock)
        return result,out
    result,out=get(clsid=UNKNOWN)
    assert result == HR(0x80040111).value and not out.value and pins==[0]
    result,out=get(iid=PROVIDER)
    assert result == NO_INTERFACE and not out.value and pins==[0]
    result,out=get()
    assert result==0 and pins==[1]
    pointer=out.value
    assert_rust_vtable(pointer)
    identity=P()
    assert invoke(pointer,0,HR,[c.POINTER(Guid),c.POINTER(P)],c.byref(UNKNOWN),c.byref(identity))==0
    assert identity.value==pointer
    assert release(identity)==1 and pins==[1]
    instance=P(123)
    args=[P,c.POINTER(Guid),c.POINTER(P)]
    assert invoke(pointer,3,HR,args,123,c.byref(UNKNOWN),c.byref(instance))==HR(0x80040110).value
    assert not instance.value and calls==[]
    assert invoke(pointer,3,HR,args,None,None,c.byref(instance))==INVALID
    assert invoke(pointer,3,HR,args,None,c.byref(UNKNOWN),None)==INVALID
    assert invoke(pointer,3,HR,args,None,c.byref(PROVIDER),c.byref(instance))==NO_INTERFACE
    assert not instance.value and calls==[bytes(PROVIDER)]
    assert invoke(pointer,4,HR,[HR],1)==0 and pins==[2]
    assert invoke(pointer,4,HR,[HR],0)==0 and pins==[1]
    assert release(pointer)==0 and pins==[0]
    print("TSF factory: Rust vtable, class/interface rejection, COM identity/refcounts, no aggregation, creation failure and balanced DLL pins passed")

def main():
    RUST_MODULES.update(Path(p).resolve() for p in sys.argv[1:])
    native = c.CDLL(str(Path(sys.argv[1]).resolve()))
    # Invalid inputs fail before touching any real registration backend.
    native.meltype_tsf_register_server.argtypes=[c.POINTER(c.c_uint16),U32]
    native.meltype_tsf_register_server.restype=HR
    assert native.meltype_tsf_register_server(None,0)==INVALID
    invalid=(c.c_uint16*2)(65,0)
    assert native.meltype_tsf_register_server(invalid,2)==INVALID
    assert native.meltype_tsf_register_server(invalid,32768)==INVALID
    native.meltype_tsf_create_display_attribute.argtypes = [c.POINTER(Guid)]
    native.meltype_tsf_create_display_attribute.restype = P
    native.meltype_tsf_create_display_attribute_enum.restype = P
    assert not native.meltype_tsf_create_display_attribute(None)
    assert not native.meltype_tsf_create_display_attribute(c.byref(UNKNOWN))
    for index, spec in enumerate(SPECS):
        pointer = native.meltype_tsf_create_display_attribute(c.byref(Guid.parse(spec[0])))
        assert pointer
        verify_info(pointer, index)
        assert release(pointer) == 0
    verify_enum(native.meltype_tsf_create_display_attribute_enum())
    verify_langbar(native)
    verify_factory(native)
    if len(sys.argv) > 2:
        tip = c.WinDLL(str(Path(sys.argv[2]).resolve()))
        tip.DllGetClassObject.argtypes = [c.POINTER(Guid), c.POINTER(Guid), c.POINTER(P)]
        tip.DllGetClassObject.restype = HR
        assert tip.DllCanUnloadNow()==0
        pin_factory=P()
        assert tip.DllGetClassObject(c.byref(SERVICE),c.byref(FACTORY),c.byref(pin_factory))==0
        assert_rust_vtable(pin_factory)
        assert invoke(pin_factory,4,HR,[HR],1)==0
        assert release(pin_factory)==0
        assert tip.DllCanUnloadNow()==1, "LockServer must survive the factory's last Release"
        unlock_factory=P()
        assert tip.DllGetClassObject(c.byref(SERVICE),c.byref(FACTORY),c.byref(unlock_factory))==0
        assert invoke(unlock_factory,4,HR,[HR],0)==0
        release(unlock_factory)
        assert tip.DllCanUnloadNow()==0
        # Rejected class/interface requests never pin the shell.
        rejected=P(123)
        assert tip.DllGetClassObject(c.byref(UNKNOWN),c.byref(FACTORY),c.byref(rejected))==HR(0x80040111).value
        assert not rejected.value and tip.DllCanUnloadNow()==0
        assert tip.DllGetClassObject(c.byref(SERVICE),c.byref(PROVIDER),c.byref(rejected))==NO_INTERFACE
        assert not rejected.value and tip.DllCanUnloadNow()==0
        factory = P()
        assert tip.DllGetClassObject(c.byref(Guid.parse("c540baf6-88b1-5b30-b524-2c8702b034fc")), c.byref(Guid.parse("00000001-0000-0000-c000-000000000046")), c.byref(factory)) == 0
        assert_rust_vtable(factory)
        assert tip.DllCanUnloadNow() == 1
        assert invoke(factory,4,HR,[HR],1)==0
        provider = P()
        assert invoke(factory, 3, HR, [P, c.POINTER(Guid), c.POINTER(P)], None, c.byref(PROVIDER), c.byref(provider)) == 0
        # A failed request cannot leak a C++ service reference.
        rejected=P(123)
        assert invoke(factory,3,HR,[P,c.POINTER(Guid),c.POINTER(P)],None,c.byref(FACTORY),c.byref(rejected))==NO_INTERFACE
        assert rejected.value is None
        assert invoke(factory,4,HR,[HR],0)==0
        release(factory)
        enumerator = P()
        assert invoke(provider, 3, HR, [c.POINTER(P)], c.byref(enumerator)) == 0
        verify_enum(enumerator.value)
        pointer = P()
        assert invoke(provider, 4, HR, [c.POINTER(Guid), c.POINTER(P)], c.byref(Guid.parse(SPECS[2][0])), c.byref(pointer)) == 0
        release(provider)
        # The standalone DLL must stay resident for the independently held child.
        assert tip.DllCanUnloadNow() == 1
        verify_info(pointer.value, 2)
        release(pointer)
        assert tip.DllCanUnloadNow() == 0
    print("TSF COM: three upstream display styles, identities, BSTR ownership, reference counts, partial enumeration, Clone/Reset/Skip and provider delegation passed")

if __name__ == "__main__":
    main()
