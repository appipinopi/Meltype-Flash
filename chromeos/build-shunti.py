#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""shuntiの実行コードだけをWASMへコンパイルする。モデルは同梱しない。"""
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]

def build(destination):
    destination = Path(destination)
    destination.mkdir(parents=True, exist_ok=True)
    source = ROOT/'rust/ai/vendor/shunti'
    subprocess.run(['em++', '-std=c++17', '-O3', '-msimd128', '-fexceptions', str(source/'engine.cpp'),
        '-sMODULARIZE=1', '-sDYNAMIC_EXECUTION=0', '-sEXPORT_ES6=1', '-sENVIRONMENT=web,worker', '-sFILESYSTEM=0',
        '-sALLOW_MEMORY_GROWTH=1', '-sMAXIMUM_MEMORY=536870912', '-sINITIAL_MEMORY=16777216',
        '-sEXPORTED_FUNCTIONS=["_kkc_open","_kkc_close","_kkc_set_threads","_kkc_convert","_malloc","_free"]',
        '-o', str(destination/'shunti-engine.mjs')], check=True)
    for name in ['LICENSE','THIRD_PARTY_NOTICES.md','DATA_SOURCES.md','UPSTREAM.json']:
        shutil.copyfile(source/name,destination/name)
    shutil.copyfile(ROOT/'rust/ai/packages.json',destination/'packages.json')
    shutil.copyfile(ROOT/'rust/ai/model-catalog.json',destination/'model-catalog.json')

if __name__ == '__main__':
    build(sys.argv[1])
