# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Linux更新の配布選択と整合性検査。ネット接続・インストールは行わない。"""
import io
import json
from pathlib import Path
import runpy
import unittest
from types import SimpleNamespace
from unittest.mock import patch


class LinuxUpdateTests(unittest.TestCase):
    def setUp(self):
        self.main = runpy.run_path(str(Path(__file__).resolve().parents[1] / 'rust/update-linux.py'))['main']
        self.asset = {'name': 'meltype-flash_1.0.2_amd64.deb', 'digest': 'sha256:' + 'a' * 64,
                      'browser_download_url': 'https://github.com/appipinopi/Meltype-Flash/releases/download/v1.0.2/test.deb'}

    def run_update(self, current='1.0.1', action='check'):
        releases = [{'tag_name': 'v1.0.2', 'assets': [self.asset]}]
        def fetch(url):
            return io.BytesIO(json.dumps(releases).encode() if 'api.github.com' in url else b'corrupt package')
        with patch.dict(self.main.__globals__, fetch=fetch), patch('platform.machine', return_value='x86_64'), patch.object(Path, 'exists', return_value=True), patch('sys.argv', ['update', action, current]), patch('sys.stdout', new_callable=io.StringIO) as output:
            self.main()
            return output.getvalue()

    def test_newer_release(self):
        self.assertIn('利用できます', self.run_update())

    def test_published_asset_name(self):
        self.asset['name'] = 'Meltype-flash-Linux-x64.deb'
        self.assertIn('利用できます', self.run_update())

    def test_current_release(self):
        self.assertIn('最新', self.run_update('1.0.2'))

    def test_missing_digest(self):
        self.asset['digest'] = ''
        with self.assertRaisesRegex(ValueError, '公開パッケージ'):
            self.run_update()

    def test_corrupt_download_never_opens_installer(self):
        with patch('subprocess.run') as launch:
            with self.assertRaisesRegex(ValueError, 'SHA-256'):
                self.run_update(action='apply')
            launch.assert_not_called()

    def test_install_waits_for_package_manager_and_reports_cancellation(self):
        install = self.main.__globals__['install_package']
        with patch('shutil.which', side_effect=lambda name: '/usr/bin/' + name), patch('subprocess.run', return_value=SimpleNamespace(returncode=0)) as launch:
            install(Path('/tmp/package.deb'), True)
            self.assertEqual(launch.call_args.args[0], ['/usr/bin/pkexec', '/usr/bin/apt-get', 'install', '-y', str(Path('/tmp/package.deb'))])
            self.assertEqual(launch.call_args.kwargs['timeout'], 600)
        with patch('shutil.which', return_value='/usr/bin/tool'), patch('subprocess.run', return_value=SimpleNamespace(returncode=126)):
            with self.assertRaisesRegex(ValueError, 'キャンセル'):
                install(Path('/tmp/package.deb'), True)

    def test_package_manager_failure_is_not_reported_as_success(self):
        install = self.main.__globals__['install_package']
        with patch('shutil.which', return_value='/usr/bin/tool'), patch('subprocess.run', return_value=SimpleNamespace(returncode=1, stderr='dependency failure', stdout='')):
            with self.assertRaisesRegex(ValueError, 'dependency failure'):
                install(Path('/tmp/package.deb'), True)


if __name__ == '__main__':
    unittest.main()
