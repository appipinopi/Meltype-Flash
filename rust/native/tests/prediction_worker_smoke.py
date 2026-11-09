# SPDX-License-Identifier: GPL-3.0-or-later
"""Prediction selection/phrase learning through the actual framed Windows worker."""
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile

worker=Path(sys.argv[1]).resolve()
fixture=json.loads((Path(__file__).resolve().parents[2]/'core/tests/data/upstream-prediction-keys-v150.json').read_text(encoding='utf-8'))
rows=[r for r in fixture['cases'] if r['case']['learning']]
assert len(rows)==4
with tempfile.TemporaryDirectory(prefix='meltype-prediction-worker-') as directory:
    for index,row in enumerate(rows):
        profile=Path(directory,str(index));profile.mkdir()
        (profile/'config.json').write_text(json.dumps(dict(LiveConversion=False,PredictiveCandidates=True,LearningEnabled=True,DetectionLevel='Manual',TabConversion=False)),encoding='utf-8')
        (profile/'state.json').write_text(json.dumps(dict(phrases=row['case']['phrases'])),encoding='utf-8')
        env=dict(os.environ,MELTYPE_RUST_DATA_DIR=str(profile),MELTYPE_TEST_TIP_NO_CONVERTERS='1')
        process=subprocess.Popen([str(worker),'--stdio-tip-worker'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,env=env)
        def call(op,**fields):
            request=json.dumps(dict(op=op,sid='prediction',process='test.exe',**fields)).encode()
            process.stdin.write(struct.pack('<I',len(request))+request);process.stdin.flush()
            header=process.stdout.read(4);assert len(header)==4,(op,process.poll())
            length=struct.unpack('<I',header)[0];assert 0<length<=1048576
            return json.loads(process.stdout.read(length))
        def projection(value):
            view=value['view']
            return dict(consumed=value['consumed'],commits=value['commits'],view={k:view[k] for k in ('text','converting','predictions','selectedPrediction')} if view else None)
        try:
            call('hello')
            for key,expected in zip(row['keys'],row['expected'],strict=True):
                actual=call('select',index=key[1],prediction=True) if key[0]==-1 else call('key',vk=key[0],ch=key[1],mods=key[2])
                assert projection(actual)==projection(expected),(row['case']['name'],key,expected,actual)
            call('close')
        finally:
            process.stdin.close();process.wait(timeout=10);assert process.returncode==0
        state=json.loads((profile/'state.json').read_text(encoding='utf-8'))
        phrases=sorted([{k:p[k] for k in ('reading','text','count')} for p in state['phrases']],key=lambda p:(p['reading'],p['text']))
        assert phrases==row['phrasesAfter'],(row['case']['name'],phrases,row['phrasesAfter'])
print('Windows worker: 4 original prediction Enter/click traces and full-reading phrase counts passed')
