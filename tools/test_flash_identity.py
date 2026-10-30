# SPDX-License-Identifier: GPL-3.0-or-later
import importlib.util, shutil, tempfile, unittest
from pathlib import Path
spec=importlib.util.spec_from_file_location('flash_identity',Path(__file__).with_name('check-flash-identity.py'))
audit=importlib.util.module_from_spec(spec);spec.loader.exec_module(audit)
REPO=Path(__file__).resolve().parent.parent
class SyncRegressionTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup);self.root=Path(self.temp.name)
        for name in audit.REQUIRED:
            target=self.root/name;target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(REPO/name,target)
    def replace(self,name,old,new):
        path=self.root/name;text=path.read_text(encoding='utf-8-sig');self.assertIn(old,text);path.write_text(text.replace(old,new),encoding='utf-8')
    def test_flash_branch_is_valid(self):
        self.assertEqual(audit.inspect(self.root),[])
    def test_synced_csharp_readme_is_rejected(self):
        (self.root/'README.md').write_text('# Meltype\nInstall C# from https://github.com/yksr-melt/Meltype/releases',encoding='utf-8')
        self.assertTrue(any('README' in error for error in audit.inspect(self.root)))
    def test_upstream_build_trigger_cannot_silently_return(self):
        shutil.copyfile(self.root/'docs/ci-legacy/build.yml', self.root/'.github/workflows/build.yml')
        self.replace('.github/workflows/build.yml',"github.repository == 'yksr-melt/Meltype'",'true')
        self.assertTrue(any('C# automation' in error for error in audit.inspect(self.root)))
    def test_archived_build_guard_is_preserved(self):
        self.replace('docs/ci-legacy/build.yml',"github.repository == 'yksr-melt/Meltype'",'true')
        self.assertTrue(any('C# automation' in error for error in audit.inspect(self.root)))
    def test_shared_ime_id_is_rejected(self):
        self.replace('rust/Manage-Windows.ps1','C540BAF6-88B1-5B30-B524-2C8702B034FC','417D801B-A9BD-4C26-BD16-356A825A6998')
        self.assertTrue(any('registration' in error for error in audit.inspect(self.root)))
    def test_settings_executable_cannot_be_reverted(self):
        self.replace('rust/settings-app/Cargo.toml','name = "meltype-flash"','name = "meltype-settings"')
        self.assertTrue(any('executable' in error for error in audit.inspect(self.root)))
    def test_sync_must_not_publish_a_release(self):
        self.replace('.github/workflows/rust-release.yml','on:','on:\n  push:',)
        self.assertTrue(any('side effect' in error for error in audit.inspect(self.root)))
if __name__=='__main__':unittest.main()
