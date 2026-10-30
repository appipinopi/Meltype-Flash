# SPDX-License-Identifier: GPL-3.0-or-later
"""Verify isolated Flash dialogs create native windows and close independently.
Only windows owned by the child test process are inspected/closed. No IME is registered.
"""
import argparse, ctypes as c, hashlib, json, os, subprocess, tempfile, time
from pathlib import Path
if os.name != 'nt': raise SystemExit('Windows only')
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('binary', type=Path)
p.add_argument('report', type=Path)
a=p.parse_args()
settings_version=json.loads((Path(__file__).resolve().parents[3]/'rust/core/data/settings-defaults.json').read_text(encoding='utf-8'))['SettingsVersion']
user=c.WinDLL('user32', use_last_error=True)
P=c.c_void_p
callback=c.WINFUNCTYPE(c.c_int,P,c.c_ssize_t)
user.GetWindowThreadProcessId.argtypes=[P,c.POINTER(c.c_uint32)]
user.GetWindowTextW.argtypes=[P,c.c_wchar_p,c.c_int]
user.PostMessageW.argtypes=[P,c.c_uint,P,c.c_ssize_t]
user.IsWindow.argtypes=[P]
user.IsWindowVisible.argtypes=[P]
user.EnumWindows.argtypes=[callback,c.c_ssize_t]
class Rect(c.Structure): _fields_=[('left',c.c_long),('top',c.c_long),('right',c.c_long),('bottom',c.c_long)]
user.GetClientRect.argtypes=[P,c.POINTER(Rect)]
def windows(pid):
    result={}
    @callback
    def visit(hwnd, ignored):
        owner=c.c_uint32();user.GetWindowThreadProcessId(hwnd,c.byref(owner))
        if owner.value==pid and user.IsWindowVisible(hwnd):
            text=c.create_unicode_buffer(512);user.GetWindowTextW(hwnd,text,512)
            if text.value: result[text.value]=hwnd
        return 1
    user.EnumWindows(visit,0)
    return result
results=[]
for name,title in [('log','ログ / 判定理由'),('report','不具合の報告・提案'),('corrections','直した誤判定を報告'),('about','について'),('updates','更新')]:
    with tempfile.TemporaryDirectory(prefix='flash-native-dialog-') as temporary:
        root=Path(temporary)
        config=json.dumps(dict(SettingsVersion=settings_version,LearningEnabled=False,WelcomeShown=True,TsfIntroduced=True,AutoUpdate=False))
        (root/'config.json').write_text(config,encoding='utf-8')
        (root/'userdict.txt').write_text('はし\t橋\n',encoding='utf-8')
        env=dict(os.environ,MELTYPE_FLASH_DATA_DIR=str(root),MELTYPE_RUST_DATA_DIR=str(root),MELTYPE_UI_CAPTURE=str(root/'capture'),MELTYPE_UI_LIFECYCLE=name,MELTYPE_UI_PAGE='0',MELTYPE_UI_CAPTURE_NATIVE_DIALOG='1')
        env.pop('MELTYPE_TRAY_SMOKE',None)
        with (root/'process.log').open('w',encoding='utf-8') as log:
            process=subprocess.Popen([str(a.binary.resolve())],env=env,stdout=log,stderr=subprocess.STDOUT)
            try:
                deadline=time.monotonic()+15
                found=None
                while time.monotonic()<deadline and process.poll() is None:
                    current=windows(process.pid)
                    target=next(((caption,hwnd) for caption,hwnd in current.items() if caption=='Meltype-flash '+title),None)
                    if target and current.get('Meltype-flash 設定'):
                        found=(target,current.get('Meltype-flash 設定'));break
                    time.sleep(.025)
                if found is None: raise AssertionError(f'{name}: no native dialog; {windows(process.pid)}')
                (caption,child),parent=found
                assert parent and child!=parent, f'{name}: dialog reused settings window'
                rect=Rect();assert user.GetClientRect(child,c.byref(rect))
                assert rect.right>=400 and rect.bottom>=200
                # A user closes only this dialog. The draft settings window must survive.
                assert user.PostMessageW(child,0x10,None,0)
                deadline=time.monotonic()+3
                while user.IsWindow(child) and time.monotonic()<deadline and process.poll() is None: time.sleep(.01)
                assert not user.IsWindow(child), f'{name}: dialog did not close'
                assert user.IsWindow(parent), f'{name}: closing dialog destroyed settings window'
                process.wait(timeout=20)
                assert process.returncode==0, (name,(root/'process.log').read_text())
                assert (root/'config.json').read_text(encoding='utf-8')==config
                assert not (root/'userdict.sqlite3').exists()
                results.append(dict(dialog=name,title=caption,nativeWindow=True,client=[rect.right,rect.bottom],independentClose=True,settingsUnchanged=True))
            finally:
                if process.poll() is None: process.terminate();process.wait(timeout=10)
a.report.parent.mkdir(parents=True,exist_ok=True)
a.report.write_text(json.dumps(dict(passed=True,binary_sha256=hashlib.sha256(a.binary.read_bytes()).hexdigest(),dialogs=results,registeredIme=False),ensure_ascii=False,indent=2),encoding='utf-8')
print('PASS: five native dialog windows, independent close, settings/dictionary unchanged; no IME registration')
