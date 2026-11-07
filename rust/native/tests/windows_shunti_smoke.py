# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""実際のWindows shuntiとTIPワーカーで、候補・確定・無効化を確認する。モデルは別途導入する。"""
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import time


def main():
    worker, helper = (Path(p).resolve() for p in sys.argv[1:3])
    direct = subprocess.run([str(helper)], input='C\t\tはし\n', text=True,
                            encoding='utf-8', capture_output=True, timeout=30, check=True)
    lines = direct.stdout.strip('\r\n').split('\n')
    assert lines[0].rstrip('\r') == 'READY', lines
    expected = lines[1].rstrip('\r').split('\x1f')[1:]
    assert len(expected) >= 2, expected
    with tempfile.TemporaryDirectory(prefix='meltype-shunti-worker-') as temp:
        profile = Path(temp)
        config = profile/'config.json'
        settings = dict(Enabled=True, UseShunti=True, ShuntiHelperPath=str(helper),
                        UseZenzai=False, ConversionEngine='System', LiveConversion=False,
                        PredictiveCandidates=False, LearningEnabled=False, DetectionLevel='Manual')
        config.write_text(json.dumps(settings), encoding='utf-8')
        env = dict(os.environ, MELTYPE_RUST_DATA_DIR=str(profile))
        env.pop('MELTYPE_TEST_TIP_NO_CONVERTERS', None)
        process = subprocess.Popen([str(worker), '--stdio-tip-worker'], env=env,
                                   stdin=subprocess.PIPE, stdout=subprocess.PIPE)

        def call(op, **fields):
            request = json.dumps(dict(op=op, sid='test', process='test.exe', **fields)).encode()
            process.stdin.write(struct.pack('<I', len(request))+request)
            process.stdin.flush()
            header = process.stdout.read(4)
            assert len(header) == 4, (op, process.poll())
            size, = struct.unpack('<I', header)
            assert 0 < size <= 1024*1024
            return json.loads(process.stdout.read(size))

        try:
            call('hello')
            deadline = time.monotonic()+15
            while True:
                call('reset')
                for ch in 'hashi':
                    assert call('key', vk=ord(ch.upper()), ch=ord(ch))['consumed']
                reply = call('key', vk=32, ch=32)
                candidates = reply['view']['candidates']
                if candidates[:len(expected)] == expected:
                    break
                assert time.monotonic() < deadline, (expected, reply)
                time.sleep(.05)
            assert reply['view']['text'] == expected[0], reply
            selected = call('select', index=1)
            assert selected['view']['text'] == expected[1], selected
            committed = call('key', vk=13)
            assert ''.join(c['text'] for c in committed['commits']) == expected[1], committed
            settings['UseShunti'] = False
            config.write_text(json.dumps(settings), encoding='utf-8')
            call('reset')
            assert call('key', vk=65, ch=97)['view']['text'] == 'あ'
            print('PASS: Windows shunti real model, all AI candidates, selection, commit and disable')
        finally:
            process.stdin.close()
            try:
                assert process.wait(timeout=10) == 0
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
                raise


if __name__ == '__main__':
    main()
