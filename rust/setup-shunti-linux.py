#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""確認後にのみshunti用実行環境・辞書・モデルを個人領域へ取得する。"""
import hashlib
import json
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.request

REPO = 'appipinopi/Meltype-Flash'
ASSETS = {
    'kkc_lex.bin': ('kkc_lex_v5.bin', '209a151603339a533810a5692037a02dc89725cd4a461b7ecc730e7ade207c75'),
    'kkc_model.bin': ('kkc_model_s8pv5.bin', '8f802b7c1659a56d00c5566dd03ea1c280fda58eb139cbf55393575e8cc84a41'),
}

def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

def download(url, path, checksum):
    if not re.fullmatch('[a-fA-F0-9]{64}', checksum):
        raise ValueError('SHA-256がありません')
    with urllib.request.urlopen(urllib.request.Request(url, headers={'User-Agent': 'Meltype-Shunti-Setup'}), timeout=60) as response, path.open('wb') as output:
        shutil.copyfileobj(response, output)
    if digest(path).lower() != checksum.lower():
        raise ValueError('SHA-256が一致しません')

def installed(root):
    files = json.loads((root/'receipt.json').read_text())['files']
    if not {'shunti-helper', 'libshunti.so', *ASSETS} <= files.keys():
        raise ValueError('shunti環境が不完全です')
    for name, expected in files.items():
        path = (root/name).resolve()
        if not path.is_relative_to(root.resolve()) or digest(path) != expected:
            raise ValueError('shunti環境が変更されています')
    return {'engine': 'shunti', 'helper': str(root/'shunti-helper')}

def setup(data_root, local_package=None, model=None):
    if platform.system() != 'Linux' or platform.machine() != 'x86_64':
        raise ValueError('shunti追加パッケージはLinux x64専用です')
    parent = Path(data_root).resolve()/'ai'
    parent.mkdir(parents=True, exist_ok=True)
    directory = model['directory'] if model else 'shunti-14801bff-s8'
    if not re.fullmatch('[a-zA-Z0-9][a-zA-Z0-9_.-]*', directory):
        raise ValueError('不正なモデルIDです')
    target = parent/directory
    if target.exists():
        result = installed(target)
        if model and any(digest(target/f['name']) != f['sha256'] for f in model['files']):
            raise ValueError('選択したモデルと一致しません')
        return result
    with tempfile.TemporaryDirectory(prefix='.setup-shunti-', dir=parent) as temporary:
        stage = Path(temporary)
        archive = stage/'runtime.tar.gz'
        if local_package is not None:
            shutil.copyfile(local_package, archive)
        else:
            with urllib.request.urlopen(urllib.request.Request(f'https://api.github.com/repos/{REPO}/releases/tags/flash-v1.0.6', headers={'User-Agent': 'Meltype-Shunti-Setup'}), timeout=30) as response:
                release = json.load(response)
            name = 'Meltype-flash-Shunti-Linux-x64.tar.gz'
            asset = next(a for a in release['assets'] if a['name'] == name)
            url = f'https://github.com/{REPO}/releases/download/flash-v1.0.6/{name}'
            if asset['browser_download_url'] != url:
                raise ValueError('実行環境の配布元が一致しません')
            download(url, archive, (asset.get('digest') or '').removeprefix('sha256:'))
        with tarfile.open(archive) as bundle:
            members = bundle.getmembers()
            if sum(m.size for m in members) > 128*1024**2 or any(not m.isfile() or '/' in m.name or m.name.startswith('.') for m in members):
                raise ValueError('不正な実行環境パッケージです')
            bundle.extractall(stage, filter='data')
        archive.unlink()
        receipt = json.loads((stage/'runtime-receipt.json').read_text())
        for name, checksum in receipt['files'].items():
            if Path(name).name != name or digest(stage/name) != checksum:
                raise ValueError('実行環境の検証に失敗しました')
        specs = model['files'] if model else [{'name': name, 'url': f'https://github.com/shuntilettuce/Japanese-IME-for-HarmonyOS-next/releases/download/shuntelligence-v7/{source}', 'sha256': checksum} for name, (source, checksum) in ASSETS.items()]
        for spec in specs:
            if spec['name'] not in ASSETS:
                raise ValueError('不正なファイル名です')
            download(spec['url'], stage/spec['name'], spec['sha256'])
        (stage/'shunti-helper').chmod(0o755)
        probe = subprocess.run([str(stage/'shunti-helper')], input='C\t\tきょうはいいてんきです\n', text=True, capture_output=True, timeout=30)
        if probe.returncode or not probe.stdout.startswith('READY\n') or '\x1f' not in probe.stdout:
            raise ValueError('shuntiの変換試験に失敗しました: ' + probe.stderr[-1000:])
        files = {p.name: digest(p) for p in stage.iterdir() if p.is_file()}
        (stage/'receipt.json').write_text(json.dumps({'files': files}))
        stage.rename(target)
    return installed(target)

if __name__ == '__main__':
    try:
        spec = json.loads(Path(sys.argv[2]).read_text(encoding='utf-8')) if len(sys.argv) > 2 else None
        print(json.dumps(setup(sys.argv[1], model=spec)))
    except Exception as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
