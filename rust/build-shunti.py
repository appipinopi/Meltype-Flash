#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""固定した上流ソースからLinux向け任意導入パッケージを作る。モデルは含めない。"""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.request

COMMIT = '14801bffb90c9ea5f2dd0f594943a97ea35c8a1a'
REPO = 'shuntilettuce/Japanese-IME-for-HarmonyOS-next'

def build(destination):
    with tempfile.TemporaryDirectory(prefix='meltype-shunti-build-') as temp:
        root = Path(temp)
        for name in ['engine.cpp', 'engine.h', 'engine_simd.h', 'ai_learn.h']:
            url = f'https://raw.githubusercontent.com/{REPO}/{COMMIT}/entry/src/main/cpp/{name}'
            (root/name).write_bytes(urllib.request.urlopen(url, timeout=60).read())
        for name in ['LICENSE', 'THIRD_PARTY_NOTICES.md', 'DATA_SOURCES.md']:
            (root/name).write_bytes(urllib.request.urlopen(f'https://raw.githubusercontent.com/{REPO}/{COMMIT}/{name}', timeout=60).read())
        subprocess.run(['g++', '-std=c++17', '-O2', '-fPIC', '-shared', '-pthread', str(root/'engine.cpp'), '-o', str(root/'libshunti.so')], check=True)
        shutil.copyfile(Path(__file__).with_name('shunti-helper.py'), root/'shunti-helper')
        (root/'shunti-helper').chmod(0o755)
        (root/'UPSTREAM.txt').write_text(f'shunti IME / shuntilettuce\nhttps://github.com/{REPO}/tree/{COMMIT}\nコード: MIT、モデル: CC BY 4.0、辞書: CC BY-SA 4.0（各詳細は同梱文書参照）\nモデル・辞書は任意導入時に別途取得します。\n', encoding='utf-8')
        files = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in root.iterdir() if p.is_file()}
        (root/'runtime-receipt.json').write_text(json.dumps({'commit': COMMIT, 'files': files}), encoding='utf-8')
        destination = Path(destination)
        destination.parent.mkdir(parents=True, exist_ok=True)
        with tarfile.open(destination, 'x:gz') as archive:
            for path in sorted(root.iterdir()):
                archive.add(path, arcname=path.name)

if __name__ == '__main__':
    build(sys.argv[1])
