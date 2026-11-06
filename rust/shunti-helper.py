#!/usr/bin/python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""任意導入したshuntelligenceを既存の有界変換IPCへ接続する。"""
import ctypes as C
from pathlib import Path
import sys

def main():
    root = Path(__file__).resolve().parent
    lib = C.CDLL(str(root / 'libshunti.so'))
    pointer = C.c_void_p
    u16 = C.POINTER(C.c_uint16)
    lib.kkc_open_files.argtypes = [C.c_char_p, C.c_char_p]
    lib.kkc_open_files.restype = pointer
    lib.kkc_close.argtypes = [pointer]
    lib.kkc_set_threads.argtypes = [pointer, C.c_int]
    lib.kkc_convert.argtypes = [pointer, u16, C.c_int, u16, C.c_int, C.c_int, C.c_int, u16, C.c_int]
    lib.kkc_convert.restype = C.c_int
    engine = lib.kkc_open_files(bytes(root / 'kkc_lex.bin'), bytes(root / 'kkc_model.bin'))
    if not engine:
        raise RuntimeError('shuntiの辞書・モデルを開けません')
    lib.kkc_set_threads(engine, 2)
    def utf16(text):
        raw = text.encode('utf-16-le')
        return (C.c_uint16 * (len(raw)//2)).from_buffer_copy(raw)
    print('READY', flush=True)
    try:
        while True:
            line = sys.stdin.readline(65537)
            if not line:
                break
            if len(line) > 65536:
                break
            fields = line.rstrip('\r\n').split('\t')
            if len(fields) != 3 or fields[0] != 'C':
                print('ERR', flush=True)
                continue
            context, reading = fields[1][-80:], fields[2]
            if not reading or len(reading) > 512:
                print('', flush=True)
                continue
            ctx, kana = utf16(context), utf16(reading)
            output = (C.c_uint16 * 65536)()
            count = lib.kkc_convert(engine, ctx, len(ctx), kana, len(kana), 8, 1, output, len(output))
            candidates = bytes(output).decode('utf-16-le').split('\0')[:max(0, count)]
            candidates = [s.replace('\x1e', ' ').replace('\x1f', ' ').replace('\t', ' ').replace('\n', ' ').replace('\r', ' ') for s in candidates if s]
            print('\x1f'.join([reading] + candidates) if candidates else '', flush=True)
    finally:
        lib.kkc_close(engine)

if __name__ == '__main__':
    main()
