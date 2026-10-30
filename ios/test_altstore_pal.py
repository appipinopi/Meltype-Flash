# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""PALの配布メタデータと、不適切な入力を拒否する検査。"""
import importlib.util
import json
from pathlib import Path
import plistlib
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('pal', Path(__file__).with_name('altstore-pal.py'))
pal = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pal)


class SourceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.app = self.root / 'MeltypeFlash.app'
        self.keyboard = self.app / 'PlugIns' / 'MeltypeFlashKeyboard.appex'
        self.keyboard.mkdir(parents=True)
        self.info = dict(CFBundleIdentifier='com.appipinopi.meltypeflash',
                         CFBundleShortVersionString='0.1.0', CFBundleVersion='12',
                         MinimumOSVersion='15.0', DTPlatformName='iphoneos')
        self.write_info(self.app, self.info)
        self.child_info = dict(self.info, CFBundleIdentifier=self.info['CFBundleIdentifier'] + '.keyboard',
                               NSExtension={'NSExtensionPointIdentifier': 'com.apple.keyboard-service'},
                               NSCameraUsageDescription='テスト用の権限説明')
        self.write_info(self.keyboard, self.child_info)
        self.adp = self.root / 'adp'
        (self.adp / 'variant').mkdir(parents=True)
        (self.adp / 'manifest.json').write_text('{"test": true}', encoding='utf-8')
        (self.adp / 'signature').write_bytes(b'test-signature')
        (self.adp / 'variant' / 'test.ipa').write_bytes(b'test-ipa')
        self.args = dict(tag='flash-v0.1.0-preview.6', marketplace_id='123456789',
                         developer_name='Test Developer', icon_url='https://example.com/icon.png',
                         download_url='https://example.com/pre6/manifest.json', release_date='2026-10-24')

    @staticmethod
    def write_info(bundle, info):
        (bundle / 'Info.plist').write_bytes(plistlib.dumps(info))

    def entitlements(self, bundle):
        identifier = self.info['CFBundleIdentifier'] + ('.keyboard' if bundle == self.keyboard else '')
        return {'application-identifier': 'TEAM.' + identifier,
                'com.apple.developer.team-identifier': 'TEAM',
                'com.apple.security.application-groups': ['group.test']}

    def generate(self):
        with patch.object(pal, 'signed_entitlements', side_effect=self.entitlements):
            return pal.make_source(self.app, self.adp, **self.args)

    def test_source_uses_actual_versions_size_and_extension_permissions(self):
        source = json.loads(json.dumps(self.generate()))
        app = source['apps'][0]
        self.assertEqual(app['versions'][0]['version'], '0.1.0')
        self.assertEqual(app['versions'][0]['buildVersion'], '12')
        self.assertEqual(app['versions'][0]['marketingVersion'], '0.1.0 Pre6')
        self.assertEqual(app['versions'][0]['size'], 8)
        self.assertEqual(app['appPermissions']['entitlements'], ['com.apple.security.application-groups'])
        self.assertEqual(app['appPermissions']['privacy']['NSCameraUsageDescription'], 'テスト用の権限説明')

    def test_pre5_and_older_are_rejected(self):
        for tag in ('flash-v0.1.0-preview.5', 'flash-v0.0.9-preview.99'):
            self.args['tag'] = tag
            with self.assertRaises(ValueError):
                self.generate()

    def test_unsigned_ipa_url_is_rejected(self):
        self.args['download_url'] = 'https://example.com/unsigned.ipa'
        with self.assertRaises(ValueError):
            self.generate()

    def test_missing_adp_signature_is_rejected(self):
        (self.adp / 'signature').unlink()
        with self.assertRaises(ValueError):
            self.generate()

    def test_missing_keyboard_is_rejected(self):
        (self.keyboard / 'Info.plist').unlink()
        self.keyboard.rmdir()
        with self.assertRaises(ValueError):
            self.generate()

    def test_simulator_and_mismatched_build_are_rejected(self):
        for key, value in (('DTPlatformName', 'iphonesimulator'), ('CFBundleVersion', '13')):
            self.write_info(self.keyboard, dict(self.child_info, **{key: value}))
            with self.assertRaises(ValueError):
                self.generate()

    def test_mismatched_app_groups_are_rejected(self):
        original = self.entitlements
        def different_group(bundle):
            result = original(bundle)
            if bundle == self.keyboard:
                result['com.apple.security.application-groups'] = ['group.other']
            return result
        self.entitlements = different_group
        with self.assertRaises(ValueError):
            self.generate()

    def test_codesign_failure_is_not_ignored(self):
        with patch.object(pal.subprocess, 'run', side_effect=subprocess.CalledProcessError(1, 'codesign')):
            with self.assertRaises(subprocess.CalledProcessError):
                pal.signed_entitlements(self.app)

    def test_development_signature_is_rejected(self):
        result = subprocess.CompletedProcess([], 0, stdout=plistlib.dumps({'get-task-allow': True}))
        with patch.object(pal.subprocess, 'run', return_value=result):
            with self.assertRaises(ValueError):
                pal.signed_entitlements(self.app)


if __name__ == '__main__':
    unittest.main()
