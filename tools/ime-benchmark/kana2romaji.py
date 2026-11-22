#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""ひらがな読み -> ローマ字入力への変換。

Meltypeコア自身のローマ字表 (rust/core/src/romaji.tsv) を逆引きして作った
対応表を使うため、エンジンが受理する綴りだけを生成する。
- っ: 次の子音を重ねる (がっこう -> gakkou)
- ん: 次が母音・や行・な行のとき nn、それ以外は n (かんい -> kanni / しんぶん -> shinbun)
- ー: - (エンジンが '-' を ー として処理する)
- カタカナはひらがなに正規化してから変換する
英字・数字・記号はそのまま返す (typed, target) の target は、
かな部分をひらがなに正規化した文字列 (Layer A の照合先)。
"""
import re

# かな -> 代表ローマ字 (romaji.tsv の逆引き、標準的な綴りを優先)
KANA_ROMAJI = {
    'あ': 'a', 'い': 'i', 'う': 'u', 'え': 'e', 'お': 'o',
    'か': 'ka', 'き': 'ki', 'く': 'ku', 'け': 'ke', 'こ': 'ko',
    'さ': 'sa', 'し': 'shi', 'す': 'su', 'せ': 'se', 'そ': 'so',
    'た': 'ta', 'ち': 'chi', 'つ': 'tsu', 'て': 'te', 'と': 'to',
    'な': 'na', 'に': 'ni', 'ぬ': 'nu', 'ね': 'ne', 'の': 'no',
    'は': 'ha', 'ひ': 'hi', 'ふ': 'fu', 'へ': 'he', 'ほ': 'ho',
    'ま': 'ma', 'み': 'mi', 'む': 'mu', 'め': 'me', 'も': 'mo',
    'や': 'ya', 'ゆ': 'yu', 'よ': 'yo',
    'ら': 'ra', 'り': 'ri', 'る': 'ru', 'れ': 're', 'ろ': 'ro',
    'わ': 'wa', 'を': 'wo', 'ゐ': 'wyi', 'ゑ': 'wye',
    'が': 'ga', 'ぎ': 'gi', 'ぐ': 'gu', 'げ': 'ge', 'ご': 'go',
    'ざ': 'za', 'じ': 'ji', 'ず': 'zu', 'ぜ': 'ze', 'ぞ': 'zo',
    'だ': 'da', 'ぢ': 'di', 'づ': 'du', 'で': 'de', 'ど': 'do',
    'ば': 'ba', 'び': 'bi', 'ぶ': 'bu', 'べ': 'be', 'ぼ': 'bo',
    'ぱ': 'pa', 'ぴ': 'pi', 'ぷ': 'pu', 'ぺ': 'pe', 'ぽ': 'po',
    'ゔ': 'vu',
    'きゃ': 'kya', 'きゅ': 'kyu', 'きょ': 'kyo',
    'しゃ': 'sha', 'しゅ': 'shu', 'しょ': 'sho', 'しぇ': 'she',
    'ちゃ': 'cha', 'ちゅ': 'chu', 'ちょ': 'cho', 'ちぇ': 'che',
    'にゃ': 'nya', 'にゅ': 'nyu', 'にょ': 'nyo',
    'ひゃ': 'hya', 'ひゅ': 'hyu', 'ひょ': 'hyo',
    'みゃ': 'mya', 'みゅ': 'myu', 'みょ': 'myo',
    'りゃ': 'rya', 'りゅ': 'ryu', 'りょ': 'ryo',
    'ぎゃ': 'gya', 'ぎゅ': 'gyu', 'ぎょ': 'gyo',
    'じゃ': 'ja', 'じゅ': 'ju', 'じょ': 'jo', 'じぇ': 'je',
    'ぢゃ': 'dya', 'ぢゅ': 'dyu', 'ぢょ': 'dyo',
    'びゃ': 'bya', 'びゅ': 'byu', 'びょ': 'byo',
    'ぴゃ': 'pya', 'ぴゅ': 'pyu', 'ぴょ': 'pyo',
    'てぃ': 'thi', 'でぃ': 'dhi',
    'ふぁ': 'fa', 'ふぃ': 'fi', 'ふぇ': 'fe', 'ふぉ': 'fo',
    'うぃ': 'wi', 'うぇ': 'we', 'うぉ': 'wo',
    'ゔぁ': 'va', 'ゔぃ': 'vi', 'ゔぇ': 've', 'ゔぉ': 'vo',
    'つぁ': 'tsa', 'つぃ': 'tsi', 'つぇ': 'tse', 'つぉ': 'tso',
    'ぁ': 'la', 'ぃ': 'li', 'ぅ': 'lu', 'ぇ': 'le', 'ぉ': 'lo',
    'ゃ': 'lya', 'ゅ': 'lyu', 'ょ': 'lyo', 'ゎ': 'lwa',
}
_SMALL = set('ぁぃぅぇぉゃゅょゎ')


def _to_hiragana(text):
    out = []
    for ch in text:
        code = ord(ch)
        if 0x30A1 <= code <= 0x30F6:  # カタカナ -> ひらがな
            out.append(chr(code - 0x60))
        else:
            out.append(ch)
    return ''.join(out)


def _units(hira):
    """ひらがな文字列を (かな単位, ローマ字) の列にする。っ・ん・ーは呼び出し側で処理するため素通しマーカーを返す。"""
    units = []
    i = 0
    while i < len(hira):
        ch = hira[i]
        if ch in ('っ', 'ん', 'ー'):
            units.append((ch, None))
            i += 1
            continue
        if i + 1 < len(hira) and hira[i + 1] in _SMALL and (ch + hira[i + 1]) in KANA_ROMAJI:
            pair = ch + hira[i + 1]
            units.append((pair, KANA_ROMAJI[pair]))
            i += 2
            continue
        units.append((ch, KANA_ROMAJI.get(ch)))
        i += 1
    return units


def convert(text):
    """(typed, kana_target) を返す。typed はエンジンへ打つ文字列、kana_target は照合用のひらがな正規化文字列。"""
    hira = _to_hiragana(text)
    typed_parts = []
    target_parts = []
    # かなの塊と非かなの塊に分けて処理する
    tokens = re.findall(r'[ぁ-ゖー]+|[^ぁ-ゖー]+', hira)
    for token in tokens:
        if not re.fullmatch(r'[ぁ-ゖー]+', token):
            typed_parts.append(token)
            target_parts.append(token)
            continue
        units = _units(token)
        romaji_units = []
        for idx, (kana, roma) in enumerate(units):
            if kana == 'ー':
                romaji_units.append('-')
                continue
            if kana == 'ん':
                nxt = units[idx + 1][1] if idx + 1 < len(units) else None
                if nxt is not None and (nxt[0] in 'aiueoy' or nxt.startswith('n')):
                    romaji_units.append('nn')
                else:
                    romaji_units.append('n')
                continue
            if kana == 'っ':
                nxt = units[idx + 1][1] if idx + 1 < len(units) else None
                if nxt and nxt[0] not in 'aiueo':
                    if nxt.startswith('ch'):
                        romaji_units.append('t')
                    else:
                        romaji_units.append(nxt[0])
                else:
                    romaji_units.append('ltu')
                continue
            romaji_units.append(roma if roma is not None else kana)
        typed_parts.append(''.join(romaji_units))
        target_parts.append(token)
    return ''.join(typed_parts), ''.join(target_parts)


if __name__ == '__main__':
    import sys
    for line in sys.stdin:
        typed, target = convert(line.rstrip('\n'))
        print(f'{typed}\t{target}')
