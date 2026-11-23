# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""AIセットアップの整合性・再利用・失敗時の配置をネット接続なしで検証。"""
import hashlib
import io
import json
from pathlib import Path
import runpy
import tarfile
import tempfile
import unittest
from unittest.mock import patch


class SetupTests(unittest.TestCase):
    def setUp(self):
        self.module = runpy.run_path(str(Path(__file__).resolve().parents[1] / 'rust/setup-ai-linux.py'))
        self.setup = self.module['setup']
        self.globals = self.setup.__globals__
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.model = b'model fixture'
        self.calls = []

    def archive(self, unsafe=False):
        stream = io.BytesIO()
        with tarfile.open(fileobj=stream, mode='w:gz') as archive:
            entry = tarfile.TarInfo('../escape' if unsafe else 'llama/llama-server')
            data = b'runtime fixture'
            entry.mode = 0o755
            entry.size = len(data)
            archive.addfile(entry, io.BytesIO(data))
        return stream.getvalue()

    def run_setup(self, unsafe=False, corrupt=False, model=None):
        runtime = self.archive(unsafe)
        name = 'llama-b12125-bin-ubuntu-x64.tar.gz'
        url = 'https://github.com/ggml-org/llama.cpp/releases/download/b12125/' + name
        release = {'assets': [{'name': name, 'browser_download_url': url,
                               'digest': 'sha256:' + hashlib.sha256(runtime).hexdigest()}]}
        def fetch(request, **kwargs):
            address = request.full_url
            self.calls.append(address)
            if 'api.github.com' in address:
                return io.BytesIO(json.dumps(release).encode())
            if address == url:
                return io.BytesIO(runtime)
            return io.BytesIO(b'corrupt' if corrupt else self.model)
        with patch.dict(self.globals, MODEL_SHA=hashlib.sha256(self.model).hexdigest()), patch('platform.system', return_value='Linux'), patch('platform.machine', return_value='x86_64'), patch('urllib.request.urlopen', side_effect=fetch), patch('subprocess.run'):
            return self.setup(self.root, model)

    def test_install_and_reuse_verified_environment(self):
        paths = self.run_setup()
        self.assertTrue(Path(paths['server']).is_file())
        self.assertTrue(Path(paths['model']).is_file())
        self.assertFalse(list((self.root / 'ai').glob('.setup-*')))
        self.calls.clear()
        self.assertEqual(paths, self.run_setup())
        self.assertEqual([], self.calls)

    def test_selected_versions_have_separate_installations_and_reuse_offline(self):
        def spec(name):
            return {'directory': name, 'files': [{'name': 'model.gguf', 'url': 'https://huggingface.co/example/model/resolve/pinned/model.gguf', 'sha256': hashlib.sha256(self.model).hexdigest()}]}
        first = self.run_setup(model=spec('jinen-v2-small-q5_k_m'))
        second = self.run_setup(model=spec('jinen-v2-xsmall-q5_k_m'))
        self.assertNotEqual(first['model'], second['model'])
        self.assertTrue(Path(first['model']).is_file())
        self.calls.clear()
        self.assertEqual(first, self.run_setup(model=spec('jinen-v2-small-q5_k_m')))
        self.assertEqual(self.calls, [])
        wrong = spec('jinen-v2-small-q5_k_m'); wrong['files'][0]['sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, '一致'):
            self.run_setup(model=wrong)
        self.assertEqual(self.calls, [])

    def test_corrupt_model_does_not_publish_environment(self):
        with self.assertRaisesRegex(ValueError, 'SHA-256'):
            self.run_setup(corrupt=True)
        self.assertEqual([], list((self.root / 'ai').iterdir()))

    def test_archive_cannot_escape_runtime_directory(self):
        with self.assertRaises(tarfile.FilterError):
            self.run_setup(unsafe=True)
        self.assertEqual([], list((self.root / 'ai').iterdir()))

    def test_modified_cached_model_is_rejected_without_network(self):
        paths = self.run_setup()
        Path(paths['model']).write_bytes(b'changed')
        self.calls.clear()
        with self.assertRaises(ValueError):
            self.run_setup()
        self.assertEqual([], self.calls)


if __name__ == '__main__':
    unittest.main()
