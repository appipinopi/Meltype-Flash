# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Capture six Rust settings pages in isolated profiles; capture is not UI equivalence."""
import argparse,hashlib,json,os,subprocess,struct,zlib
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('binary',type=Path);p.add_argument('output',type=Path)
p.add_argument('--pages',nargs='+',choices=['settings','dictionary','snippets','learning','backup','input-test'])
p.add_argument('--lifecycle',choices=['welcome','about','updates','report','corrections','log'],help='Capture a lifecycle window over the selected page.')
p.add_argument('--help-key',help='Show this settings explanation in a visual-test build.')
a=p.parse_args();a.output.mkdir(parents=True,exist_ok=False)
settings_version=json.loads((Path(__file__).resolve().parents[3]/'rust/core/data/settings-defaults.json').read_text(encoding='utf-8'))['SettingsVersion']
results=[]
for page,name in enumerate(['settings','dictionary','snippets','learning','backup','input-test']):
    if a.pages and name not in a.pages: continue
    profile=(a.output/(name+'-profile')).resolve();profile.mkdir()
    config=json.dumps(dict(SettingsVersion=settings_version,LearningEnabled=False,LiveConversion=False,WelcomeShown=True,TsfIntroduced=True,AutoUpdate=False))
    (profile/'config.json').write_text(config,encoding='utf-8')
    (profile/'userdict.txt').write_text('はし\t橋\nはし\t箸\n',encoding='utf-8')
    output=(a.output/name).resolve()
    env=dict(os.environ,MELTYPE_FLASH_DATA_DIR=str(profile),MELTYPE_RUST_DATA_DIR=str(profile),MELTYPE_UI_CAPTURE=str(output),MELTYPE_UI_PAGE=str(page));env.pop('MELTYPE_TRAY_SMOKE',None)
    env.pop('MELTYPE_UI_HELP_KEY',None)
    env.pop('MELTYPE_UI_LIFECYCLE',None)
    if a.lifecycle: env['MELTYPE_UI_LIFECYCLE']=a.lifecycle
    if a.help_key: env['MELTYPE_UI_HELP_KEY']=a.help_key
    startup=None
    if os.name=='nt':
        startup=subprocess.STARTUPINFO();startup.dwFlags|=subprocess.STARTF_USESHOWWINDOW;startup.wShowWindow=0
    with (a.output/(name+'.log')).open('w',encoding='utf-8') as log:
        subprocess.run([str(a.binary.resolve())],env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=45,startupinfo=startup)
    width,height=json.loads(output.with_suffix('.json').read_text(encoding='utf-8'))
    rgba=output.with_suffix('.rgba').read_bytes();assert len(rgba)==width*height*4
    assert (profile/'config.json').read_text(encoding='utf-8')==config,'capture must not save settings'
    assert not (profile/'userdict.sqlite3').exists(),'capture must not migrate user data'
    def chunk(kind,data):
        return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
    scanlines=b''.join(b'\0'+rgba[y*width*4:(y+1)*width*4] for y in range(height))
    output.with_suffix('.png').write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',width,height,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(scanlines))+chunk(b'IEND',b''))
    results.append(dict(page=name,width=width,height=height,rgba_sha256=hashlib.sha256(rgba).hexdigest(),captured=True,origin_pixel_equivalence_verified=False))
(a.output/'report.json').write_text(json.dumps(dict(binary_sha256=hashlib.sha256(a.binary.read_bytes()).hexdigest(),pages=results,all_screens_equivalent=False),indent=2),encoding='utf-8')
print('Captured',len(results),'isolated settings pages; original layout parity remains unverified')
