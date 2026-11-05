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
    for release in releases:
        if release.get('draft') or release.get('prerelease'):
            continue
        tag = release['tag_name']
        for asset in release.get('assets', []):
            name = asset['name']
            suffix = '_' + arch + '.deb' if deb else '.' + {'amd64': 'x86_64', 'arm64': 'aarch64'}[arch] + '.rpm'
            published = 'Meltype-flash-Linux-' + {'amd64': 'x64', 'arm64': 'arm64'}[arch] + ('.deb' if deb else '.rpm')
            if name != published and (not name.startswith('meltype-flash') or not name.endswith(suffix)):
                continue
            digest = asset.get('digest') or ''
            if not re.fullmatch(r'sha256:[0-9a-fA-F]{64}', digest):
                continue
            version = re.search(r'(\d+)\.(\d+)\.(\d+)', tag)
            current = re.search(r'(\d+)\.(\d+)\.(\d+)', sys.argv[2])
            if not version or not current:
                raise ValueError('バージョンを確認できません')
            if tuple(map(int, version.groups())) <= tuple(map(int, current.groups())):
                print('現在のバージョンは最新です。')
                return
            if sys.argv[1] != 'apply':
                print(f'Linux版 {tag} を利用できます。「ダウンロード・更新」で導入できます。')
                return
            url = asset['browser_download_url']
            if not url.startswith('https://github.com/appipinopi/Meltype-Flash/releases/download/'):
                raise ValueError('配布元が一致しません')
            directory = Path(tempfile.mkdtemp(prefix='meltype-update-'))
            package = directory / ('meltype-flash.deb' if deb else 'meltype-flash.rpm')
            try:
                with fetch(url) as response, package.open('wb') as output:
                    shutil.copyfileobj(response, output)
                with package.open('rb') as stream:
                    actual = hashlib.file_digest(stream, 'sha256').hexdigest()
                if actual.lower() != digest[7:].lower():
                    raise ValueError('配布ファイルのSHA-256が一致しません')
                subprocess.run(['xdg-open', str(package)], check=True, timeout=30)
            except Exception:
                shutil.rmtree(directory)
                raise
            print(f'{tag} のインストール画面を開きました。画面で更新を完了してください。再ログイン後に反映されます。')
            return
    raise ValueError('この環境用の検証可能な公開パッケージがありません')


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
