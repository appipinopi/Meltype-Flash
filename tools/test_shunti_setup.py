# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""shunti追加パッケージの検証と拒否時の後始末。"""
import hashlib
import io
import json
from pathlib import Path
import runpy
import tarfile
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

class SetupTests(unittest.TestCase):
    def fixture(self, root, traversal=False):
        archive = root/'runtime.tar.gz'
        payload = {'shunti-helper': b'fixture', 'libshunti.so': b'fixture'}
        receipt = {'files': {name: hashlib.sha256(data).hexdigest() for name, data in payload.items()}}
        payload['runtime-receipt.json'] = json.dumps(receipt).encode()
        if traversal:
            payload['../escape'] = b'bad'
        with tarfile.open(archive, 'w:gz') as bundle:
            for name, data in payload.items():
                info = tarfile.TarInfo(name); info.size = len(data)
                bundle.addfile(info, io.BytesIO(data))
        return archive

    def test_verified_install_reuse_and_modified_model_rejection(self):
        module = runpy.run_path(str(Path(__file__).resolve().parents[1]/'rust/setup-shunti-linux.py'))
        setup = module['setup']
        def download(url, path, expected):
            path.write_bytes(b'fixture')
        with tempfile.TemporaryDirectory() as temp, patch('platform.system', return_value='Linux'), patch('platform.machine', return_value='x86_64'):
            root = Path(temp)
            with patch.dict(setup.__globals__, download=download), patch('subprocess.run', return_value=SimpleNamespace(returncode=0, stdout='READY\nかな\x1f仮名\n', stderr='')):
                result = setup(root, self.fixture(root))
            with patch.dict(setup.__globals__, download=lambda *args: self.fail('unexpected download')):
                self.assertEqual(setup(root), result)
                (Path(result['helper']).parent/'kkc_model.bin').write_bytes(b'modified')
                with self.assertRaisesRegex(ValueError, '変更'):
                    setup(root)

    def test_traversal_is_rejected_without_published_environment(self):
        setup = runpy.run_path(str(Path(__file__).resolve().parents[1]/'rust/setup-shunti-linux.py'))['setup']
        with tempfile.TemporaryDirectory() as temp, patch('platform.system', return_value='Linux'), patch('platform.machine', return_value='x86_64'):
            root = Path(temp)
            with self.assertRaisesRegex(ValueError, '不正'):
                setup(root, self.fixture(root, traversal=True))
            self.assertEqual(list((root/'ai').iterdir()), [])

if __name__ == '__main__':
    unittest.main()
