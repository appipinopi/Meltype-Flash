#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Linux設定画面用。検証した配布パッケージをシステムのGUIに渡す。"""
import hashlib
import json
import platform
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.request


def fetch(url):
    if not url.startswith('https://'):
        raise ValueError('HTTPS URL required')
    return urllib.request.urlopen(urllib.request.Request(url, headers={
        'User-Agent': 'Meltype-Flash-Updater', 'Accept': 'application/vnd.github+json'
    }), timeout=30)


def install_package(package, deb):
    manager = shutil.which('apt-get' if deb else 'dnf')
    authority = shutil.which('pkexec')
    if not manager or not authority:
        raise ValueError('更新にはPolicyKitとapt-get / dnfが必要です。配布パッケージをアプリセンターで開いてください。')
    result = subprocess.run([authority, manager, 'install', '-y', str(package)],
                            capture_output=True, text=True, timeout=600)
    if result.returncode in (126, 127):
        raise ValueError('更新の認証がキャンセルされたか、許可されませんでした。')
    if result.returncode:
        raise ValueError('パッケージ更新に失敗しました: ' + (result.stderr or result.stdout)[-4000:])


def main():
    arch = {'x86_64': 'amd64', 'aarch64': 'arm64'}.get(platform.machine())
    if arch is None:
        raise ValueError('未対応のCPUです')
    # Package databases, rather than availability of an incidental CLI, select format.
    deb = Path('/var/lib/dpkg/status').exists()
    if not deb and not shutil.which('rpm'):
        raise ValueError('deb / rpm対応の環境で実行してください')
    with fetch('https://api.github.com/repos/appipinopi/Meltype-Flash/releases?per_page=20') as response:
        releases = json.load(response)
    current = re.search(r'(\d+)\.(\d+)\.(\d+)', sys.argv[2])
    if not current:
        raise ValueError('バージョンを確認できません')
    current_version = tuple(map(int, current.groups()))
    # GitHub のリリース一覧は版の順に並ぶとは限らない (1.0.9 が 1.0.10 より前に来ることがある)。
    # 一覧の先頭ではなく、一番新しい版を選ぶ。
    best = None
    seen = False
    for release in releases:
        if release.get('draft') or release.get('prerelease'):
            continue
        tag = release['tag_name']
        version = re.search(r'(\d+)\.(\d+)\.(\d+)', tag)
        if not version:
            continue
        version_number = tuple(map(int, version.groups()))
        for asset in release.get('assets', []):
            name = asset['name']
            suffix = '_' + arch + '.deb' if deb else '.' + {'amd64': 'x86_64', 'arm64': 'aarch64'}[arch] + '.rpm'
            published = 'Meltype-flash-Linux-' + {'amd64': 'x64', 'arm64': 'arm64'}[arch] + ('.deb' if deb else '.rpm')
            if name != published and (not name.startswith('meltype-flash') or not name.endswith(suffix)):
                continue
            digest = asset.get('digest') or ''
            if not re.fullmatch(r'sha256:[0-9a-fA-F]{64}', digest):
                continue
            url = asset['browser_download_url']
            if not url.startswith('https://github.com/appipinopi/Meltype-Flash/releases/download/'):
                raise ValueError('配布元が一致しません')
            seen = True
            if version_number <= current_version:
                continue
            if best is None or version_number > best[0]:
                best = (version_number, tag, url, digest)
            break
    if best is None:
        if seen:
            print('現在のバージョンは最新です。')
            return
        raise ValueError('この環境用の検証可能な公開パッケージがありません')
    _version_number, tag, url, digest = best
    if sys.argv[1] != 'apply':
        print(f'Linux版 {tag} を利用できます。「ダウンロード・更新」で導入できます。')
        return
    directory = Path(tempfile.mkdtemp(prefix='meltype-update-'))
    package = directory / ('meltype-flash.deb' if deb else 'meltype-flash.rpm')
    try:
        with fetch(url) as response, package.open('wb') as output:
            shutil.copyfileobj(response, output)
        with package.open('rb') as stream:
            actual = hashlib.file_digest(stream, 'sha256').hexdigest()
        if actual.lower() != digest[7:].lower():
            raise ValueError('配布ファイルのSHA-256が一致しません')
        install_package(package, deb)
    finally:
        shutil.rmtree(directory)
    print(f'{tag} へ更新しました。設定画面を閉じて開き直してください。入力メソッドは再ログイン後に反映されます。')


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
