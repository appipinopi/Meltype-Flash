#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""確認後に呼ぶAI専用セットアップ。システム環境や設定JSONは変更しない。"""
import hashlib
import json
from pathlib import Path
import platform
import re
import shutil
import sys
import subprocess
import tarfile
import tempfile
import urllib.request

TAG = 'b12125'
MODEL_REV = 'ddf6e44b2e05ab7ea9a3e31559c5e7948761365c'
MODEL_SHA = '4de930c06bef8c263aa1aa40684af206db4ce1b96375b3b8ed0ea508e0b14f6c'
MODEL_URL = f'https://huggingface.co/Miwa-Keita/zenz-v3.1-small-gguf/resolve/{MODEL_REV}/ggml-model-Q5_K_M.gguf'


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def download(url, path, expected):
    if not re.fullmatch('[0-9a-fA-F]{64}', expected):
        raise ValueError('配布ファイルのSHA-256がありません')
    with urllib.request.urlopen(urllib.request.Request(url, headers={'User-Agent': 'Meltype-AI-Setup'}), timeout=60) as response, path.open('wb') as output:
        shutil.copyfileobj(response, output)
    if digest(path).lower() != expected.lower():
        raise ValueError('配布ファイルのSHA-256が一致しません')


def installed(root):
    receipt = json.loads((root / 'receipt.json').read_text())
    for name, expected in receipt['files'].items():
        path = (root / name).resolve()
        if not path.is_relative_to(root.resolve()) or digest(path) != expected:
            raise ValueError('既存のAI環境の検証に失敗しました')
    for key in ('server', 'model'):
        if receipt[key] not in receipt['files'] or not (root / receipt[key]).resolve().is_relative_to(root.resolve()):
            raise ValueError('AI環境の記録が不完全です')
    return {key: str(root / receipt[key]) for key in ('server', 'model')}


def setup(data_root):
    if platform.system() != 'Linux':
        raise ValueError('Linux専用のセットアップです')
    arch = {'x86_64': 'x64', 'aarch64': 'arm64'}.get(platform.machine())
    if arch is None:
        raise ValueError('このCPUは未対応です')
    parent = Path(data_root).resolve() / 'ai'
    parent.mkdir(parents=True, exist_ok=True)
    target = parent / f'zenz-v3.1-{TAG}-{arch}'
    if target.exists():
        return installed(target)
    with urllib.request.urlopen(urllib.request.Request(f'https://api.github.com/repos/ggml-org/llama.cpp/releases/tags/{TAG}', headers={'User-Agent': 'Meltype-AI-Setup'}), timeout=30) as response:
        release = json.load(response)
    name = f'llama-{TAG}-bin-ubuntu-{arch}.tar.gz'
    asset = next(a for a in release['assets'] if a['name'] == name)
    url = f'https://github.com/ggml-org/llama.cpp/releases/download/{TAG}/{name}'
    if asset['browser_download_url'] != url:
        raise ValueError('AI実行環境の配布元が一致しません')
    with tempfile.TemporaryDirectory(prefix='.setup-', dir=parent) as temporary:
        stage = Path(temporary)
        archive = stage / 'runtime.tar.gz'
        download(url, archive, (asset.get('digest') or '').removeprefix('sha256:'))
        runtime = stage / 'runtime'
        runtime.mkdir()
        with tarfile.open(archive) as bundle:
            if sum(member.size for member in bundle.getmembers()) > 1024**3:
                raise ValueError('実行環境の展開サイズが大きすぎます')
            bundle.extractall(runtime, filter='data')
        archive.unlink()
        server = next(runtime.rglob('llama-server'))
        server.chmod(server.stat().st_mode | 0o100)
        subprocess.run([str(server), '--version'], check=True, capture_output=True, timeout=15)
        download(MODEL_URL, stage / 'model.gguf', MODEL_SHA)
        files = {path.relative_to(stage).as_posix(): digest(path) for path in stage.rglob('*') if path.is_file()}
        receipt = {'server': server.relative_to(stage).as_posix(), 'model': 'model.gguf', 'files': files}
        (stage / 'receipt.json').write_text(json.dumps(receipt))
        stage.rename(target)
    return installed(target)


if __name__ == '__main__':
    try:
        print(json.dumps(setup(sys.argv[1])))
    except Exception as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
