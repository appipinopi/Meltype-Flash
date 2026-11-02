#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Package the Rust IBus bundle as a ZIP and a distinct Debian package."""
import argparse
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import zipfile


def package(bundle, output, version, arch):
    if arch not in ('amd64', 'arm64') or not re.fullmatch(r'\d+\.\d+\.\d+(?:-pre\.\d+)?', version):
        raise ValueError('Invalid architecture or version')
    bundle, output = Path(bundle), Path(output)
    for name in ('meltype-ibus', 'meltype-flash', 'meltype-rs', 'icon.png', 'LICENSE', 'THIRD-PARTY-NOTICES.md', 'install.sh'):
        if not (bundle / name).is_file():
            raise ValueError('Missing bundle member: ' + name)
    machine = {'amd64': 62, 'arm64': 183}[arch]
    for name in ('meltype-ibus', 'meltype-flash', 'meltype-rs'):
        header = (bundle / name).read_bytes()[:20]
        if header[:6] != b'\x7fELF\x02\x01' or int.from_bytes(header[18:20], 'little') != machine:
            raise ValueError('Wrong ELF architecture: ' + name)
    output.mkdir(parents=True, exist_ok=True)
    label = 'x64' if arch == 'amd64' else 'arm64'
    archive = output / f'Meltype-flash-Linux-{label}.zip'
    with zipfile.ZipFile(archive, 'x', zipfile.ZIP_DEFLATED) as zipped:
        for path in sorted(bundle.rglob('*')):
            if path.is_file():
                zipped.write(path, 'Meltype-flash-linux/' + path.relative_to(bundle).as_posix())
    with tempfile.TemporaryDirectory(prefix='meltype-flash-deb-') as temporary:
        root = Path(temporary)
        app = root / 'usr/libexec/meltype-flash/ibus'
        shutil.copytree(bundle, app)
        for name in ('meltype-ibus', 'meltype-flash', 'meltype-rs', 'install.sh'):
            (app / name).chmod(0o755)
        if (app / 'mozc/meltype_mozc_helper').exists():
            (app / 'mozc/meltype_mozc_helper').chmod(0o755)
        # Generate component paths for the final location, not the staging tree.
        component = subprocess.check_output([str(bundle.resolve() / 'meltype-ibus'), '--print-component',
            '--component-executable', '/usr/libexec/meltype-flash/ibus/meltype-ibus'], text=True)
        components = root / 'usr/share/ibus/component'
        components.mkdir(parents=True)
        (components / 'meltype-flash.xml').write_text(component, encoding='utf-8')
        desktop = root / 'usr/share/applications'
        desktop.mkdir(parents=True)
        (desktop / 'meltype-flash.desktop').write_text(
            '[Desktop Entry]\nType=Application\nName=Meltype-flash 設定\n'
            'Exec=/usr/libexec/meltype-flash/ibus/meltype-flash\n'
            'Icon=/usr/libexec/meltype-flash/ibus/icon.png\nCategories=Settings;Utility;\nTerminal=false\n', encoding='utf-8')
        # GNOMEはIBus名からこの固定名を探す（upstream #489）。
        (desktop / 'ibus-setup-meltype-flash.desktop').write_text(
            (desktop / 'meltype-flash.desktop').read_text(encoding='utf-8')
            + 'NoDisplay=true\nX-Meltype-Flash-IBus=true\n', encoding='utf-8')
        control = root / 'DEBIAN'
        control.mkdir()
        deb_version = version.replace('-pre.', '~pre.')
        (control / 'control').write_text(
            f'Package: meltype-flash\nVersion: {deb_version}\nArchitecture: {arch}\n'
            'Maintainer: appipinopi <appipinopi@users.noreply.github.com>\n'
            'Section: utils\nPriority: optional\n'
            'Depends: ibus, libc6 (>= 2.39), libgcc-s1, libxkbcommon-x11-0\n'
            'Recommends: fonts-noto-cjk\n'
            'Homepage: https://github.com/appipinopi/Meltype-Flash\n'
            'Description: Meltype-flash Rust Japanese input method\n'
            ' Rust IBus engine and settings. Sign out and select Meltype-flash after installation.\n', encoding='utf-8')
        # No process termination, input-source changes, or deletion of personal data.
        for name in ('postinst', 'postrm'):
            script = control / name
            script.write_text('#!/bin/sh\nset -e\nif command -v ibus >/dev/null 2>&1; then\n'
                              '  ibus write-cache --system >/dev/null 2>&1 || true\nfi\nexit 0\n', encoding='utf-8')
            script.chmod(0o755)
        subprocess.run(['dpkg-deb', '--root-owner-group', '-Zxz', '--build', str(root),
                        str(output.resolve() / f'meltype-flash_{version}_{arch}.deb')], check=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('bundle', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--version', required=True)
    parser.add_argument('--arch', required=True, choices=['amd64', 'arm64'])
    args = parser.parse_args()
    package(args.bundle, args.output, args.version, args.arch)
