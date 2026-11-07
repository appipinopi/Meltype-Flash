#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""固定した上流ソースからLinux・Windows向け任意導入パッケージを作る。モデルは含めない。"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
import zipfile

COMMIT = '14801bffb90c9ea5f2dd0f594943a97ea35c8a1a'
REPO = 'shuntilettuce/Japanese-IME-for-HarmonyOS-next'

def windows_build(root):
    vswhere = Path(os.environ['ProgramFiles(x86)'])/'Microsoft Visual Studio/Installer/vswhere.exe'
    vs = subprocess.check_output([str(vswhere), '-latest', '-products', '*', '-requires',
        'Microsoft.VisualStudio.Component.VC.Tools.x86.x64', '-property', 'installationPath'], text=True).strip()
    if not vs:
        raise RuntimeError('MSVC x64 build tools are required')
    vcvars = Path(vs)/'VC/Auxiliary/Build/vcvarsall.bat'
    environment = subprocess.check_output(f'cmd.exe /d /s /c ""{vcvars}" x64 >nul && set"', text=True)
    env = dict(line.split('=', 1) for line in environment.splitlines() if '=' in line and not line.startswith('='))
    env = {key.upper(): value for key, value in env.items()}
    compiler = shutil.which('cl.exe', path=env['PATH'])
    if not compiler:
        raise RuntimeError('Cannot locate MSVC compiler')
    objects = root/'objects'
    objects.mkdir()
    common = [compiler, '/nologo', '/utf-8', '/std:c++17', '/EHsc', '/O2', '/MT', '/D_CRT_SECURE_NO_WARNINGS']
    # Only the dispatched AVX2 implementation is compiled with AVX2 enabled.
    for name in ['engine.cpp', 'engine_avx2.cpp', 'shunti-helper.cpp']:
        args = common + ['/c', str(root/name), '/Fo' + str(objects/(Path(name).stem+'.obj'))]
        if name == 'engine_avx2.cpp':
            args.append('/arch:AVX2')
        subprocess.run(args, env=env, check=True, cwd=objects)
    subprocess.run(common + [str(p) for p in objects.glob('*.obj')] + ['/Fe' + str(root/'shunti-helper.exe')], env=env, check=True, cwd=objects)
    shutil.rmtree(objects)
    for extension in ["lib", "exp"]:
        (root / ("shunti-helper." + extension)).unlink(missing_ok=True)


def build(destination, windows=False, macos=None):
    with tempfile.TemporaryDirectory(prefix='meltype-shunti-build-') as temp:
        root = Path(temp)
        for name in ['engine.cpp', 'engine.h', 'engine_simd.h', 'ai_learn.h'] + (['engine_avx2.cpp'] if windows else []):
            url = f'https://raw.githubusercontent.com/{REPO}/{COMMIT}/entry/src/main/cpp/{name}'
            (root/name).write_bytes(urllib.request.urlopen(url, timeout=60).read())
        for name in ['LICENSE', 'THIRD_PARTY_NOTICES.md', 'DATA_SOURCES.md']:
            (root/name).write_bytes(urllib.request.urlopen(f'https://raw.githubusercontent.com/{REPO}/{COMMIT}/{name}', timeout=60).read())
        if windows or macos:
            shutil.copyfile(Path(__file__).with_name('shunti-helper.cpp'), root/'shunti-helper.cpp')
            shutil.copyfile(Path(__file__).resolve().parents[1]/'LICENSE', root/'MELTYPE-LICENSE.txt')
            if windows:
                windows_build(root)
            else:
                subprocess.run(['clang++','-std=c++17','-O2','-arch',macos,'-mmacosx-version-min=11.0','-pthread',str(root/'engine.cpp'),str(root/'shunti-helper.cpp'),'-o',str(root/'shunti-helper')],check=True)
                subprocess.run(['codesign','--force','--sign','-',str(root/'shunti-helper')],check=True)
        else:
            subprocess.run(['g++', '-std=c++17', '-O2', '-fPIC', '-shared', '-pthread', str(root/'engine.cpp'), '-o', str(root/'libshunti.so')], check=True)
            shutil.copyfile(Path(__file__).with_name('shunti-helper.py'), root/'shunti-helper')
            (root/'shunti-helper').chmod(0o755)
        (root/'UPSTREAM.txt').write_text(f'shunti IME / shuntilettuce\nhttps://github.com/{REPO}/tree/{COMMIT}\nコード: MIT、モデル: CC BY 4.0、辞書: CC BY-SA 4.0（各詳細は同梱文書参照）\nモデル・辞書は任意導入時に別途取得します。\n', encoding='utf-8')
        files = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in root.iterdir() if p.is_file()}
        (root/'runtime-receipt.json').write_text(json.dumps({'commit': COMMIT, 'files': files}), encoding='utf-8')
        destination = Path(destination)
        destination.parent.mkdir(parents=True, exist_ok=True)
        if windows or macos:
            with zipfile.ZipFile(destination, 'x', compression=zipfile.ZIP_DEFLATED) as archive:
                for path in sorted(root.iterdir()):
                    archive.write(path, arcname=path.name)
        else:
            with tarfile.open(destination, 'x:gz') as archive:
                for path in sorted(root.iterdir()):
                    archive.add(path, arcname=path.name)

if __name__ == '__main__':
    build(sys.argv[1], windows='--windows' in sys.argv[2:], macos=sys.argv[sys.argv.index('--macos')+1] if '--macos' in sys.argv else None)
