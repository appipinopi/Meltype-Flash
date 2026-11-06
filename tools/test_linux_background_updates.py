# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
import json
import os
from pathlib import Path
import runpy
import tempfile
import unittest
from unittest.mock import patch


class BackgroundUpdates(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.module = runpy.run_path(str(Path(__file__).resolve().parents[1] / 'rust/linux-background-updates.py'))
        self.configure = self.module['configure']
        self.environment = patch.dict(os.environ, XDG_CONFIG_HOME=str(self.root / 'config'))
        self.environment.start()
        self.addCleanup(self.environment.stop)

    def test_daily_timer_and_disable_remove_only_owned_files(self):
        with patch.dict(self.configure.__globals__, systemctl=lambda *args: True):
            self.assertEqual('daily', self.configure(self.root / 'data', self.root, '1.0.4', True))
            timer = self.root / 'config/systemd/user/meltype-flash-update.timer'
            self.assertIn('OnCalendar=daily', timer.read_text())
            self.assertEqual('disabled', self.configure(self.root / 'data', self.root, '1.0.4', False))
            self.assertFalse(timer.exists())

    def test_login_fallback_when_user_systemd_is_unavailable(self):
        with patch.dict(self.configure.__globals__, systemctl=lambda *args: False):
            self.assertEqual('login', self.configure(self.root / 'data', self.root, '1.0.4', True))
            desktop = self.root / 'config/autostart/meltype-flash-update.desktop'
            self.assertIn('Exec=', desktop.read_text(encoding='utf-8'))

    def test_foreign_file_is_preserved(self):
        desktop = self.root / 'config/autostart/meltype-flash-update.desktop'
        desktop.parent.mkdir(parents=True)
        desktop.write_text('foreign')
        with patch.dict(self.configure.__globals__, systemctl=lambda *args: False):
            with self.assertRaises(ValueError):
                self.configure(self.root / 'data', self.root, '1.0.4', True)
        self.assertEqual('foreign', desktop.read_text())

    def test_successful_recent_check_does_not_run_again(self):
        import time
        import types
        import sys
        (self.root / 'config.json').write_text(json.dumps({'AutoUpdate': True}))
        (self.root / '.last-update-check').write_text(str(time.time()))
        fake = types.SimpleNamespace(LOCK_EX=1, LOCK_NB=2, flock=lambda *args: None)
        with patch.dict(sys.modules, fcntl=fake), patch('subprocess.run') as launch:
            self.module['run'](self.root, self.root, '1.0.4')
            launch.assert_not_called()

    def test_manual_mode_never_starts_a_download(self):
        (self.root / 'config.json').write_text(json.dumps({'AutoUpdate': False}))
        with patch('subprocess.run') as launch:
            self.module['run'](self.root, self.root, '1.0.4')
            launch.assert_not_called()


if __name__ == '__main__':
    unittest.main()
