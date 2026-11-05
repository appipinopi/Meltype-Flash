#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Rust IBusの配布フォルダーからZIP・deb・rpmを作る。"""
import argparse
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import tempfile
import zipfile


TARGET = '/usr/libexec/meltype-flash/ibus'
EXECUTABLES = ('meltype-ibus', 'meltype-flash', 'meltype-rs', 'install.sh')
# GUIがdlopenするライブラリはELFの自動依存検出に現れない。
DEB_DEPENDS = ('ibus, zenity, python3, xdg-utils, libc6 (>= 2.39), libgcc-s1, libstdc++6, libibus-1.0-5, '
               'libglib2.0-0t64 | libglib2.0-0, libdbus-1-3, '
               'libx11-6, libx11-xcb1, libxcursor1, libxi6, libxkbcommon0, '
               'libxkbcommon-x11-0, libegl1, libgl1, libwayland-client0, libwayland-egl1')
RPM_DLOPEN = ('libdbus-1.so.3', 'libX11.so.6', 'libX11-xcb.so.1', 'libXcursor.so.1',
              'libXi.so.6', 'libxkbcommon.so.0', 'libxkbcommon-x11.so.0',
              'libEGL.so.1', 'libGL.so.1', 'libwayland-client.so.0', 'libwayland-egl.so.1')


def stage_payload(bundle, root):
    app = root / TARGET.lstrip('/')
    shutil.copytree(bundle, app)
    for path in root.rglob('*'):
        path.chmod(0o755 if path.is_dir() else 0o644)
    for name in EXECUTABLES:
        (app / name).chmod(0o755)
    if (app / 'mozc/meltype_mozc_helper').exists():
        (app / 'mozc/meltype_mozc_helper').chmod(0o755)
    # ステージ先ではなく、導入後の絶対パスを埋め込む。
    component = subprocess.check_output([str(bundle.resolve() / 'meltype-ibus'), '--print-component',
        '--component-executable', TARGET + '/meltype-ibus'], text=True)
    components = root / 'usr/share/ibus/component'
    components.mkdir(parents=True)
    (components / 'meltype-flash.xml').write_text(component, encoding='utf-8')
    desktop = root / 'usr/share/applications'
    desktop.mkdir(parents=True)
    (desktop / 'meltype-flash.desktop').write_text(
        '[Desktop Entry]\nType=Application\nName=Meltype-FLASH 設定\n'
        f'Exec={TARGET}/meltype-flash\nIcon=meltype-flash\nStartupWMClass=meltype-flash\n'
        'Categories=Settings;Utility;\nTerminal=false\n', encoding='utf-8')
    icons = root / 'usr/share/icons/hicolor/256x256/apps'
    icons.mkdir(parents=True, exist_ok=True)
    shutil.copy2(app / 'icon.png', icons / 'meltype-flash.png')
    metainfo = root / 'usr/share/metainfo'
    metainfo.mkdir(parents=True, exist_ok=True)
    (metainfo / 'meltype-flash.metainfo.xml').write_text(
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<component type="desktop-application">\n'
        '<id>io.github.appipinopi.MeltypeFlash</id>\n'
        '<pkgname>meltype-flash</pkgname>\n'
        '<metadata_license>CC0-1.0</metadata_license>\n'
        '<project_license>GPL-3.0-or-later</project_license>\n'
        '<name>Meltype-FLASH</name>\n'
        '<summary>Japanese input method and settings</summary>\n'
        '<description><p>Japanese input with automatic language detection and Mozc conversion.</p></description>\n'
        '<launchable type="desktop-id">meltype-flash.desktop</launchable>\n'
        '<url type="homepage">https://github.com/appipinopi/Meltype-Flash</url>\n'
        '<provides><binary>meltype-flash</binary></provides>\n'
        '</component>\n', encoding='utf-8')
    # GNOMEはIBus名からこの固定名を探す（upstream #489）。
    (desktop / 'ibus-setup-meltype-flash.desktop').write_text(
        (desktop / 'meltype-flash.desktop').read_text(encoding='utf-8')
        + 'NoDisplay=true\nX-Meltype-Flash-IBus=true\n', encoding='utf-8')


def make_deb(root, destination, version, arch):
    control = root / 'DEBIAN'
    control.mkdir()
    deb_version = version.replace('-pre.', '~pre.')
    installed_kib = sum((path.stat().st_size + 1023) // 1024 for path in root.rglob('*') if path.is_file())
    (control / 'control').write_text(
        f'Package: meltype-flash\nVersion: {deb_version}\nArchitecture: {arch}\n'
        'Maintainer: appipinopi <appipinopi@users.noreply.github.com>\n'
        f'Section: utils\nPriority: optional\nInstalled-Size: {installed_kib}\n'
        f'Depends: {DEB_DEPENDS}\nRecommends: fonts-noto-cjk, xdg-desktop-portal\n'
        'Homepage: https://github.com/appipinopi/Meltype-Flash\n'
        'Description: Meltype-FLASH Rust Japanese input method\n'
        ' Rust IBus engine and settings. Sign out and select Meltype-FLASH after installation.\n', encoding='utf-8')
    # プロセス・入力ソース・個人データを変更せず、システムのキャッシュだけを更新する。
    for name in ('postinst', 'postrm'):
        script = control / name
        script.write_text('#!/bin/sh\nset -e\nif command -v ibus >/dev/null 2>&1; then\n'
                          '  ibus write-cache --system >/dev/null 2>&1 || true\nfi\nexit 0\n', encoding='utf-8')
        script.chmod(0o755)
    subprocess.run(['dpkg-deb', '--root-owner-group', '-Zxz', '--build', str(root), str(destination)], check=True)


def make_rpm(root, destination, version, arch, stage):
    rpm_arch = {'amd64': 'x86_64', 'arm64': 'aarch64'}[arch]
    rpm_version = version.replace('-pre.', '~pre.')
    files = []
    for path in sorted(root.rglob('*')):
        relative = '/' + path.relative_to(root).as_posix()
        if path.is_dir():
            # 共有ディレクトリーはIBusやデスクトップ側のパッケージが所有する。
            if relative.startswith('/usr/libexec/meltype-flash'):
                files.append('%dir ' + relative)
        else:
            files.append(('%license ' if path.name == 'LICENSE' else '') + relative)
    required = ', '.join(name + '()(64bit)' for name in RPM_DLOPEN)
    spec = stage / 'meltype-flash.spec'
    spec.write_text(
        '%global debug_package %{nil}\n%global _build_id_links none\n'
        '# 公開済みのELFをstrip・書き換えせず、ELF依存の自動抽出は残す。\n'
        '%global __os_install_post %{nil}\n'
        f'Name: meltype-flash\nVersion: {rpm_version}\nRelease: 1\n'
        'Summary: Meltype-FLASH Rust Japanese input method\nLicense: GPL-3.0-or-later\n'
        'URL: https://github.com/appipinopi/Meltype-Flash\n'
        f'Requires: ibus, zenity, python3, xdg-utils, glibc >= 2.39, {required}\nRecommends: xdg-desktop-portal\n'
        '\n%description\nRust IBus engine and settings.\n'
        'Sign out and select Meltype-FLASH after installation.\n'
        '\n%install\nmkdir -p %{buildroot}\n'
        f'cp -a {shlex.quote(str(root))}/. %{{buildroot}}/\n'
        # FedoraのIBusパッケージはcomponentファイルのトリガーでキャッシュを更新する。
        '\n%files\n%defattr(-,root,root,-)\n' + '\n'.join(files) + '\n', encoding='utf-8')
    rpm_output = stage / 'rpms'
    subprocess.run(['rpmbuild', '-bb', '--target', rpm_arch,
        '--define', '_rpmformat 4',
        '--define', '_topdir ' + str(stage / 'rpmbuild'),
        '--define', '_rpmdir ' + str(rpm_output), str(spec)], check=True)
    produced = list(rpm_output.rglob('*.rpm'))
    if len(produced) != 1 or produced[0].name != destination.name:
        raise ValueError('Unexpected RPM output: ' + repr(produced))
    shutil.copyfile(produced[0], destination)


def package(bundle, output, version, arch, formats=('zip', 'deb')):
    if arch not in ('amd64', 'arm64') or not re.fullmatch(r'\d+\.\d+\.\d+(?:-pre\.\d+)?', version):
        raise ValueError('Invalid architecture or version')
    bundle, output = Path(bundle), Path(output)
    formats = tuple(formats)
    if not formats or len(set(formats)) != len(formats) or set(formats) - {'zip', 'deb', 'rpm'}:
        raise ValueError('Invalid package formats')
    for name in ('meltype-ibus', 'meltype-flash', 'meltype-rs', 'icon.png', 'LICENSE', 'THIRD-PARTY-NOTICES.md', 'install.sh'):
        if not (bundle / name).is_file():
            raise ValueError('Missing bundle member: ' + name)
    machine = {'amd64': 62, 'arm64': 183}[arch]
    for name in ('meltype-ibus', 'meltype-flash', 'meltype-rs'):
        header = (bundle / name).read_bytes()[:20]
        if header[:6] != b'\x7fELF\x02\x01' or int.from_bytes(header[18:20], 'little') != machine:
            raise ValueError('Wrong ELF architecture: ' + name)
    label = 'x64' if arch == 'amd64' else 'arm64'
    filenames = {'zip': f'Meltype-flash-Linux-{label}.zip',
                 'deb': f'meltype-flash_{version}_{arch}.deb',
                 'rpm': f'meltype-flash-{version.replace("-pre.", "~pre.")}-1.{"x86_64" if arch == "amd64" else "aarch64"}.rpm'}
    for kind in formats:
        if (output / filenames[kind]).exists():
            raise FileExistsError(output / filenames[kind])
    with tempfile.TemporaryDirectory(prefix='meltype-flash-package-') as temporary:
        stage = Path(temporary)
        artifacts = stage / 'artifacts'
        artifacts.mkdir()
        if 'zip' in formats:
            with zipfile.ZipFile(artifacts / filenames['zip'], 'x', zipfile.ZIP_DEFLATED) as zipped:
                for path in sorted(bundle.rglob('*')):
                    if path.is_file():
                        zipped.write(path, 'Meltype-flash-linux/' + path.relative_to(bundle).as_posix())
        if set(formats) & {'deb', 'rpm'}:
            payload = stage / 'payload'
            payload.mkdir(mode=0o755)
            stage_payload(bundle, payload)
            if 'rpm' in formats:
                make_rpm(payload, artifacts / filenames['rpm'], version, arch, stage)
            if 'deb' in formats:
                make_deb(payload, artifacts / filenames['deb'], version, arch)
        # 全形式の生成後に出力する。失敗した部分生成物を完成品として残さない。
        output.mkdir(parents=True, exist_ok=True)
        for kind in formats:
            shutil.move(str(artifacts / filenames[kind]), str(output / filenames[kind]))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('bundle', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--version', required=True)
    parser.add_argument('--arch', required=True, choices=['amd64', 'arm64'])
    parser.add_argument('--formats', nargs='+', choices=['zip', 'deb', 'rpm'], default=['zip', 'deb'])
    args = parser.parse_args()
    package(args.bundle, args.output, args.version, args.arch, args.formats)
