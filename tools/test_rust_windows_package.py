# SPDX-License-Identifier: GPL-3.0-or-later
"""Fail-closed package checks using synthetic PE metadata, not runnable programs."""
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import tempfile
import unittest
import uuid

spec = importlib.util.spec_from_file_location('package_audit', Path(__file__).with_name('check-rust-windows-package.py'))
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


def image(machine, imports=(), exports=()):
    data = bytearray(4096)
    data[:2] = b'MZ'
    def put(fmt, offset, *values):
        struct.pack_into('<'+fmt, data, offset, *values)
    put('I', 0x3c, 0x80)
    data[0x80:0x84] = b'PE\0\0'
    put('HH', 0x84, machine, 1)
    put('H', 0x94, 0xf0)
    put('H', 0x98, 0x20b)
    put('I', 0x104, 16)
    put('IIII', 0x190, 3584, 0x1000, 3584, 512)
    cursor = 0x600
    def string(text):
        nonlocal cursor
        value = cursor
        raw = text.encode()+b'\0'
        data[cursor:cursor+len(raw)] = raw
        cursor += len(raw)
        return value+0xe00
    if imports:
        put('II', 0x110, 0x1200, (len(imports)+1)*20)
        for i, name in enumerate(imports):
            put('I', 0x400+i*20+12, string(name))
    if exports:
        put('II', 0x108, 0x1000, 40)
        put('I', 0x218, len(exports))
        put('I', 0x220, 0x1080)
        for i, name in enumerate(exports):
            put('I', 0x280+i*4, string(name))
    return bytes(data) + b"".join(uuid.UUID(value).bytes_le for value in ["C540BAF6-88B1-5B30-B524-2C8702B034FC", "6D01DE9C-8AB3-5CFC-A5C6-10893CDF8762"])


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.exports = ['DllGetClassObject', 'DllCanUnloadNow', 'DllRegisterServer', 'DllUnregisterServer']
        for name, arch in audit.REQUIRED.items():
            p = self.root/name
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_bytes(image(arch, ['kernel32.dll'], self.exports if p.suffix=='.dll' else ()))
        for name in ['LICENSE', 'THIRD-PARTY-NOTICES.md', 'README-Windows.md', 'Manage-Windows.ps1', 'Start-Portable.ps1', 'Update-Windows.ps1']:
            (self.root/name).write_text('fixture', encoding='utf-8')

    def test_old_shared_identity_dll_is_rejected(self):
        path = self.root/'tip/x64/MeltypeTip.dll'
        path.write_bytes(path.read_bytes().replace(uuid.UUID('C540BAF6-88B1-5B30-B524-2C8702B034FC').bytes_le, b'X'*16))
        result = audit.inspect(self.root)
        self.assertFalse(result['passed'])
        self.assertTrue(any('missing Meltype-FLASH' in error for error in result['errors']))

    def test_complete_layout(self):
        self.assertTrue(audit.inspect(self.root)['passed'])

    def test_lifecycle_scripts_are_required(self):
        for name in ['Manage-Windows.ps1', 'Start-Portable.ps1', 'Update-Windows.ps1']:
            saved = (self.root/name).read_bytes()
            (self.root/name).unlink()
            self.assertTrue(any(name in error for error in audit.inspect(self.root)['errors']))
            (self.root/name).write_bytes(saved)

    def test_worker_is_required_next_to_tip(self):
        (self.root/'tip/x86/meltype-tip-worker.exe').unlink()
        self.assertFalse(audit.inspect(self.root)['passed'])

    def test_architecture_mismatch(self):
        (self.root/'tip/x86/meltype-tip-worker.exe').write_bytes(image(0x8664))
        self.assertTrue(any('wrong machine' in e for e in audit.inspect(self.root)['errors']))

    def test_runtime_in_another_architecture_is_not_a_fallback(self):
        (self.root/'tip/x86/meltype-tip-worker.exe').write_bytes(image(0x14c, ['vcruntime140.dll']))
        (self.root/'tip/x64/vcruntime140.dll').write_bytes(image(0x8664))
        self.assertFalse(audit.inspect(self.root)['passed'])
        (self.root/'tip/x86/vcruntime140.dll').write_bytes(image(0x14c, ['api-ms-win-crt-runtime-l1-1-0.dll']))
        self.assertTrue(audit.inspect(self.root)['passed'])

    def test_missing_export_and_malformed_pe(self):
        (self.root/'tip/x64/MeltypeTip.dll').write_bytes(image(0x8664))
        (self.root/'meltype-flash.exe').write_bytes(b'MZ')
        self.assertFalse(audit.inspect(self.root)['passed'])

    def test_manifest_detects_tampering_and_path_escape(self):
        digest = hashlib.sha256((self.root/'LICENSE').read_bytes()).hexdigest()
        manifest = self.root/'SHA256SUMS.json'
        manifest.write_text(json.dumps({'LICENSE': digest}), encoding='utf-8')
        self.assertTrue(audit.inspect(self.root)['passed'])
        (self.root/'LICENSE').write_text('modified', encoding='utf-8')
        self.assertFalse(audit.inspect(self.root)['passed'])
        manifest.write_text(json.dumps({'../outside': digest}), encoding='utf-8')
        self.assertFalse(audit.inspect(self.root)['passed'])


if __name__ == '__main__':
    unittest.main()
