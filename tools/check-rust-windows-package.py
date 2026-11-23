#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Inspect an unpacked Rust Windows preview without installing or registering it."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import uuid

REQUIRED = {
    'meltype-flash.exe': 0x8664,
    'meltype-rs.exe': 0x8664,
    'tip/x64/MeltypeTip.dll': 0x8664,
    'tip/x64/meltype-tip-worker.exe': 0x8664,
    'tip/x86/MeltypeTip.dll': 0x14c,
    'tip/x86/meltype-tip-worker.exe': 0x14c,
}
# Windows 10/11 components, not arbitrary DLLs installed on the build machine.
SYSTEM_DLLS = set(('advapi32 bcrypt bcryptprimitives cfgmgr32 combase comctl32 comdlg32 crypt32 '
    'd2d1 d3d11 dcomp dinput8 dwrite dwmapi dxgi gdi32 hid imm32 iphlpapi '
    'kernel32 kernelbase mpr msimg32 msvcrt mswsock ncrypt netapi32 normaliz '
    'ntdll ole32 oleacc oleaut32 opengl32 powrprof propsys psapi rpcrt4 '
    'rstrtmgr secur32 setupapi shell32 shcore shlwapi sspicli ucrtbase '
    'uiautomationcore user32 userenv usp10 uxtheme version winhttp wininet '
    'winmm winspool wintrust ws2_32 wtsapi32').split())


def pe(path):
    data = path.read_bytes()
    def unpack(fmt, offset):
        return struct.unpack_from('<' + fmt, data, offset)
    if data[:2] != b'MZ':
        raise ValueError('not a PE image')
    header, = unpack('I', 0x3c)
    if data[header:header+4] != b'PE\0\0':
        raise ValueError('invalid PE signature')
    machine, sections = unpack('HH', header+4)
    optional_size, = unpack('H', header+20)
    optional = header+24
    magic, = unpack('H', optional)
    if magic not in (0x10b, 0x20b):
        raise ValueError('unknown optional header')
    directories = optional + (112 if magic == 0x20b else 96)
    count, = unpack('I', directories-4)
    section_table = optional+optional_size
    def rva(value):
        for i in range(sections):
            size, address, raw_size, raw = unpack('IIII', section_table+i*40+8)
            if address <= value < address+max(size, raw_size):
                offset = raw+value-address
                if value-address >= raw_size or offset >= len(data):
                    break
                return offset
        raise ValueError(f'unmapped RVA {value:x}')
    def string(value):
        offset = rva(value)
        return data[offset:data.index(b'\0', offset)].decode('ascii')
    def directory(index):
        if index >= count or directories+(index+1)*8 > optional+optional_size:
            return 0, 0
        return unpack('II', directories+index*8)
    imports = []
    for index, stride, name_offset in [(1, 20, 12), (13, 32, 4)]:
        address, size = directory(index)
        if not address:
            continue
        offset = rva(address)
        for entry in range(0, size, stride):
            descriptor = data[offset+entry:offset+entry+stride]
            if len(descriptor) != stride:
                raise ValueError('truncated import descriptor')
            if not any(descriptor):
                break
            if index == 13 and not struct.unpack_from('<I', descriptor)[0] & 1:
                raise ValueError('unsupported VA-based delay import')
            name, = struct.unpack_from('<I', descriptor, name_offset)
            imports.append(string(name).lower())
        else:
            raise ValueError('unterminated import directory')
    exports = []
    address, _ = directory(0)
    if address:
        offset = rva(address)
        names, = unpack('I', offset+24)
        table, = unpack('I', offset+32)
        for i in range(names):
            name, = unpack('I', rva(table)+i*4)
            exports.append(string(name))
    return dict(machine=machine, imports=sorted(set(imports)), exports=exports)


def inspect(root):
    errors, binaries = [], {}
    for name in [*REQUIRED, 'LICENSE', 'THIRD-PARTY-NOTICES.md', 'README-Windows.md', 'Manage-Windows.ps1', 'Start-Portable.ps1', 'Update-Windows.ps1']:
        if not (root/name).is_file():
            errors.append('missing required file: ' + name)
    queue = [(name, arch) for name, arch in REQUIRED.items() if (root/name).is_file()]
    while queue:
        name, arch = queue.pop(0)
        if name in binaries:
            continue
        path = root/name
        try:
            info = pe(path)
        except (ValueError, struct.error, IndexError) as error:
            errors.append(f'{name}: {error}')
            continue
        info['sha256'] = hashlib.sha256(path.read_bytes()).hexdigest()
        info['bytes'] = path.stat().st_size
        binaries[name] = info
        if info['machine'] != arch:
            errors.append(f'{name}: wrong machine {info["machine"]:x}; expected {arch:x}')
        if path.name == 'MeltypeTip.dll':
            payload = path.read_bytes()
            flash_ids = ['C540BAF6-88B1-5B30-B524-2C8702B034FC', '6D01DE9C-8AB3-5CFC-A5C6-10893CDF8762']
            info['flash_identity'] = all(uuid.UUID(value).bytes_le in payload for value in flash_ids)
            if not info['flash_identity']:
                errors.append(f'{name}: missing Meltype-FLASH service/profile identity; rebuild the Rust DLL')
            for export in ['DllGetClassObject', 'DllCanUnloadNow', 'DllRegisterServer', 'DllUnregisterServer']:
                if export not in info['exports']:
                    errors.append(f'{name}: missing TSF export {export}')
        info['resolved_imports'] = {}
        local = {p.name.lower(): p for p in path.parent.iterdir() if p.is_file()}
        for dependency in info['imports']:
            # The TIP DLL and worker must resolve their own adjacent DLLs;
            # another architecture's folder is never used as a fallback.
            if dependency in local:
                target = local[dependency].relative_to(root).as_posix()
                info['resolved_imports'][dependency] = target
                queue.append((target, arch))
            elif dependency.startswith(('api-ms-win-', 'ext-ms-win-')) or dependency.removesuffix('.dll') in SYSTEM_DLLS:
                info['resolved_imports'][dependency] = 'Windows 10/11'
            else:
                errors.append(f'{name}: missing non-system dependency {dependency}')
    manifest_path = root/'SHA256SUMS.json'
    if manifest_path.is_file():
        try:
            manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
            if not isinstance(manifest, dict):
                raise ValueError('manifest must be an object')
            for name, expected in manifest.items():
                path = (root/name).resolve()
                if not path.is_relative_to(root.resolve()) or not path.is_file():
                    errors.append('invalid or missing manifest file: ' + name)
                elif hashlib.sha256(path.read_bytes()).hexdigest() != expected:
                    errors.append('manifest hash mismatch: ' + name)
        except (ValueError, TypeError) as error:
            errors.append('invalid SHA256SUMS.json: ' + str(error))
    return dict(passed=not errors, errors=errors, binaries=binaries,
        system_data='FST indexes and dictionary payload are embedded by rust/core/build.rs; no separate system TXT/FST runtime files required.',
        settings='User config and SQLite database are created in the Rust data directory; no personal data is shipped.',
        optional_mozc={arch: (root/f'tip/{arch}/mozc/meltype_mozc_helper.exe').is_file() for arch in ['x64', 'x86']},
        limitations=['Static PE/import/export and file inspection only; no IME registration or user-document input.',
                     'This does not validate installation, updates, clean-machine operation or all dynamically loaded dependencies.',
                     'The optional Mozc helper/model is not required for the System converter; Japanese Windows language components are needed.',
                     'Unbounded setting and OS/learning-state enumeration is excluded from acceptance.'])


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('package', type=Path)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    report = inspect(args.package.resolve())
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
    print(json.dumps(dict(passed=report['passed'], errors=report['errors'], binaries=len(report['binaries'])), ensure_ascii=False))
    raise SystemExit(not report['passed'])
