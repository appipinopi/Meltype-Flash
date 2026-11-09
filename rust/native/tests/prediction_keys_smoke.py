# SPDX-License-Identifier: GPL-3.0-or-later
"""Original prediction display, navigation and persisted phrase-count traces.

Compare edits, displayed clauses, active selection and the complete active list.
Annotations and inactive selection sentinels are outside this projection.
"""
import ctypes as c
import json
import os
from pathlib import Path
import sys
import tempfile

filename = 'meltype_native.dll' if os.name == 'nt' else ('libmeltype_native.dylib' if sys.platform == 'darwin' else 'libmeltype_native.so')
api = c.CDLL(str(Path(sys.argv[1] if len(sys.argv) > 1 else Path('target/release') / filename).resolve()))
allocator = c.CDLL('ucrtbase') if os.name == 'nt' else c.CDLL(None)
allocator.malloc.argtypes = [c.c_size_t]
allocator.malloc.restype = c.c_void_p
clause_type = c.CFUNCTYPE(c.c_void_p, c.c_char_p, c.c_char_p)
candidate_type = c.CFUNCTYPE(c.c_void_p, c.c_char_p)
queries = []


def allocate(text):
    data = text.encode() + b'\0'
    pointer = allocator.malloc(len(data))
    assert pointer
    c.memmove(pointer, data, len(data))
    return pointer


def candidate(reading, index):
    return f'【{reading}{index}】'


@clause_type
def clauses(raw, _context):
    reading = raw.decode()
    parts = ['はし', 'かみ'] if reading == 'はしかみ' else [reading]
    return allocate('\n'.join(r + '\t' + candidate(r, 0) for r in parts))


@candidate_type
def candidates(raw):
    reading = raw.decode()
    queries.append(reading)
    return allocate('\n'.join(candidate(reading, i) for i in range(20)))


api.meltype_init.argtypes = [clause_type, candidate_type, c.c_void_p]
api.meltype_init.restype = c.c_int
api.meltype_create_at.argtypes = [c.c_char_p]
api.meltype_create_at.restype = c.c_void_p
api.meltype_destroy.argtypes = [c.c_void_p]
api.meltype_destroy.restype = None
api.meltype_free.argtypes = [c.c_void_p]
api.meltype_free.restype = None
api.meltype_handle_key.argtypes = [c.c_void_p, c.c_int, c.c_int, c.c_int, c.c_char_p, c.c_char_p]
api.meltype_handle_key.restype = c.c_void_p
api.meltype_select_prediction.argtypes = [c.c_void_p, c.c_int]
api.meltype_select_prediction.restype = c.c_void_p
assert api.meltype_init(clauses, candidates, None) == 1


def projection(result):
    view = result['view']
    fields = ('text', 'converting', 'clauses', 'predictions', 'selectedPrediction') + (('selectedClause', 'selectedIndex', 'candidates') if view and view['converting'] else ())
    return dict(consumed=result['consumed'], commits=result['commits'],
                view={k: view[k] for k in fields} if view else None)


fixture = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(__file__).resolve().parents[2] / 'core/tests/data/upstream-prediction-keys-v150.json'
golden = json.loads(fixture.read_text(encoding='utf-8'))
assert len(golden['cases']) == 110
with tempfile.TemporaryDirectory(prefix='meltype-prediction-keys-') as directory:
    for number, row in enumerate(golden['cases']):
        profile = Path(directory, str(number))
        profile.mkdir()
        (profile / 'config.json').write_text(json.dumps(dict(LiveConversion=False, PredictiveCandidates=True,
            LearningEnabled=row['case']['learning'], DetectionLevel=row['case']['level'], ScoredSegmentation=False,
            TabConversion=row['case'].get('tab', False))), encoding='utf-8')
        (profile / 'state.json').write_text(json.dumps(dict(phrases=row['case']['phrases'])), encoding='utf-8')
        handle = api.meltype_create_at(str(profile).encode())
        assert handle
        queries.clear()
        try:
            for index, (key, expected) in enumerate(zip(row['keys'], row['expected'], strict=True)):
                pointer = api.meltype_select_prediction(handle, key[1]) if key[0] == -1 else api.meltype_handle_key(handle, *key, None, None)
                assert pointer, (row['case']['name'], index)
                try:
                    actual = json.loads(c.string_at(pointer))
                finally:
                    api.meltype_free(pointer)
                assert projection(actual) == projection(expected), (row['case']['name'], index, key, projection(expected), projection(actual))
            state = json.loads((profile / 'state.json').read_text(encoding='utf-8'))
            actual_phrases = sorted([{k:p[k] for k in ('reading','text','count')} for p in state.get('phrases', [])], key=lambda p:(p['reading'],p['text']))
            assert actual_phrases == row['phrasesAfter'], (row['case']['name'], row['phrasesAfter'], actual_phrases)
        finally:
            api.meltype_destroy(handle)
print(f"Native ABI: {fixture.stem}: {len(golden['cases'])} original public key traces passed")
