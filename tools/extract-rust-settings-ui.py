# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Yukishiro, appipinopi
"""Keep Rust settings labels, descriptions, categories and enum labels upstream-aligned."""
import argparse
import json
from pathlib import Path
import re
import subprocess

root = Path(__file__).resolve().parent.parent
baseline = "46db6ca2f941eb9580f4d5ed103ee0bee71111c7"
try:
    source = subprocess.check_output(
        ["git", "show", baseline + ":src/Meltype.Core/Config/Settings.cs"],
        cwd=root, stderr=subprocess.PIPE,
    ).decode("utf-8-sig")
except subprocess.CalledProcessError as error:
    raise SystemExit(f"Missing v1.4.0 reference; fetch it first: git fetch --no-tags upstream {baseline}") from error

def string(raw):
    return json.loads('"' + raw + '"')

fields = []
pattern = r"public\s+([\w<>?\[\]]+)\s+(\w+)\s*\{\s*get;\s*set;"
previous = 0
for match in re.finditer(pattern, source):
    typ, key = match.groups()
    attrs = source[previous:match.start()]
    previous = match.end()
    if "Category(" not in attrs:
        continue
    attrs = attrs[attrs.rfind("[Category("):]
    def attr(name):
        found = re.search(name + r'\("((?:\\.|[^"\\])*)"\)', attrs)
        return string(found.group(1)) if found else ""
    typ = {"bool": "Boolean", "int": "Int32", "string": "String", "string[]": "String[]"}.get(typ, typ)
    if typ.startswith("List<"):
        typ = "List`1"
    fields.append(dict(key=key, type=typ, category=attr("Category"),
                       label=attr("DisplayName"), description=attr("Description")))
enums = {}
for match in re.finditer(r"public enum (\w+)\s*\{(.*?)\n\}", source, re.S):
    enums[match[1]] = {name: string(label) for label, name in re.findall(
        r'\[Description\("((?:\\.|[^"\\])*)"\)\]\s*(\w+)', match[2])}
assert fields and len({f["key"] for f in fields}) == len(fields)
assert all(f["label"] and f["category"] for f in fields)
# Flash-only settings that are not in the pinned upstream metadata. Keeping
# them here lets --check continue to verify the upstream fields exactly while
# still shipping the extra settings.
FLASH_FIELDS = [
    dict(key="UiLanguage", type="UiLanguage", category="1. 全般",
         label="設定画面の言語",
         description="設定画面の表示言語です。自動はOSの言語に合わせます。"),
    dict(key="StartConversionAtLastClause", type="Boolean", category="3. 候補",
         label="変換を始めたときに最後の文節を選ぶ",
         description="SpaceやTabで変換を始めたとき、先頭ではなく最後の文節を選んだ状態にします。長い文を後ろから直すときに使います。"),
    dict(key="TimeZone", type="String", category="1. 全般",
         label="タイムゾーン",
         description="「とけい」「じこく」で出す現在時刻のタイムゾーンです。例: Asia/Tokyo, UTC, America/New_York。"),
    dict(key="PrereleaseUpdate", type="Boolean", category="9. 更新",
         label="プレリリース版も自動更新の対象にする",
         description="自動更新で、正式版だけでなくプレリリース版も確認します。既定はOFFです。"),
    dict(key="NowPlayingCandidates", type="Boolean", category="5. 候補",
         label="「なう」で再生中の曲を候補に出す",
         description="「なう」を変換したとき、Windowsで再生中の曲名・アーティストを候補に出します。再生中の曲がないときは時刻を出します。入力内容は送信しません。"),
    dict(key="ShowTsfModeIndicator", type="Boolean", category="2. 入力モード",
         label="Meltype IME でも入力モードをカーソルの近くに表示",
         description="Meltype IME の入力モードを切り替えたとき、カーソルの近くに「あ」（日本語）か「A」（英数）を1.2秒表示します。"),
    dict(key="KanaLayout", type="KanaLayout", category="2. 入力モード",
         label="かな配列",
         description="かな入力のときに使う配列です。JISかな・月・花・NICOLA・中指NICOLA・TRONから選びます。月・花は前置シフト、NICOLA系・TRONは親指シフトです。"),
    dict(key="RomajiLayout", type="RomajiLayout", category="2. 入力モード",
         label="ローマ字の配列",
         description="ローマ字入力のときに使う配列です。QWERTYのほか、SKY配列を選べます。"),
    dict(key="ReadingLattice", type="Boolean", category="3. 判定",
         label="ReadingLatticeで日英を判定",
         description="日本語の語・英単語・未知語をラティスにして、日英の区切りを選びます。上流1.5.0の判定の作り直しをFlashの辞書へ接続した試験版です。"),
    dict(key="ZenzaiDevice", type="String", category="4. 変換",
         label="Zenzai: 使う GPU",
         description="llama-server に渡す --device の値です。空なら CPU/既定の GPU を使います。"),
    dict(key="SkkMode", type="Boolean", category="2. 入力モード",
         label="SKKライクな入力を使う",
         description="大文字で▽開始、Spaceで▼変換、Enterで確定します。送り仮名と辞書登録にも対応します。小文字は通常のローマ字・かな入力へそのまま流れます。"),
    dict(key="AsyncConversion", type="Boolean", category="4. 変換",
         label="変換を非同期で行う",
         description="変換キーの入力時に待たずに戻り、結果が届いてから候補表示を更新します。Windows TSF・IBus・macOSに対応します。既定はオフです。"),
]
FLASH_DEFAULTS = {"TimeZone": "Asia/Tokyo", "PrereleaseUpdate": False, "NowPlayingCandidates": True, "ShowTsfModeIndicator": True, "KanaLayout": "Jis", "RomajiLayout": "Qwerty", "ReadingLattice": False}
FLASH_ENUMS = {
    "UiLanguage": {"Auto": "自動 (OSに合わせる)", "Japanese": "日本語", "English": "English"},
    "KanaLayout": {"Jis": "JIS かな", "Tsuki": "月配列", "Hana": "花配列", "Nicola": "NICOLA (親指シフト)", "NakayubiNicola": "中指NICOLA", "Tron": "TRON 配列"},
    "RomajiLayout": {"Qwerty": "QWERTY", "Sky": "SKY 配列"},
}
for field in FLASH_FIELDS:
    if field["key"] not in {item["key"] for item in fields}:
        fields.append(field)
for name, values in FLASH_ENUMS.items():
    enums.setdefault(name, values)
data = dict(upstream=baseline, fields=fields, enums=enums)
text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
path = root / "rust/settings-app/settings-ui.json"
# Preserve the Rust profile/host defaults and legacy keys used by migration,
# while adding new scalar settings from the same pinned upstream source.
# Composite settings (rules, profiles and shortcuts) have their own migration.
defaults_path = root / "rust/core/data/settings-defaults.json"
defaults_text = defaults_path.read_text(encoding="utf-8")
defaults = json.loads(defaults_text)
field_keys = {field["key"] for field in fields}
for match in re.finditer(
    r'public\s+(bool|int|string)\s+(\w+)\s*\{\s*get;\s*set;\s*\}'
    r'(?:\s*=\s*(true|false|-?\d+|"(?:\\.|[^"\\])*")\s*;)?', source
):
    typ, key, initializer = match.groups()
    if key in field_keys and key not in defaults:
        defaults[key] = json.loads(initializer) if initializer is not None else {
            "bool": False, "int": 0, "string": ""
        }[typ]
shortcut_source = subprocess.check_output(
    ["git", "show", baseline + ":src/Meltype.Core/Input/ToggleShortcut.cs"], cwd=root
).decode("utf-8-sig")
constants = {name: int(value, 0) for name, value in re.findall(
    r"public const int (\w+) = (0x[0-9A-Fa-f]+|\d+);", shortcut_source
)}
modifiers = {name: int(value) for name, value in re.findall(
    r"(None|Shift|Control|Alt|Win) = (\d+)", shortcut_source
)}
default_shortcuts = shortcut_source.split("Defaults() =>", 1)[1].split("];", 1)[0]
defaults["ToggleShortcuts"] = [
    {"Key": constants[key], "Modifiers": modifiers.get(modifier, 0)}
    for key, modifier in re.findall(
        r"new\(\) \{ Key = (\w+)(?:, Modifiers = ShortcutModifiers\.(\w+))? \}",
        default_shortcuts
    )
]
assert defaults["ToggleShortcuts"], "Upstream shortcut defaults were not recognized"
defaults["SettingsVersion"] = int(re.search(r"public const int CurrentVersion = (\d+);", source)[1])
for legacy in ("HankakuTogglesKeyboard", "ToggleKeyboardShortcuts"):
    defaults.pop(legacy, None)
for key, value in FLASH_DEFAULTS.items():
    defaults.setdefault(key, value)
defaults_output = json.dumps(defaults, ensure_ascii=False, indent=2) + "\n"
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--check", action="store_true")
args = parser.parse_args()
if args.check:
    if path.read_text(encoding="utf-8") != text:
        raise SystemExit("Settings UI metadata differs from upstream; run this script to regenerate.")
    if defaults_text != defaults_output:
        raise SystemExit("New upstream settings defaults are missing; run this script to regenerate.")
else:
    path.write_text(text, encoding="utf-8")
    defaults_path.write_text(defaults_output, encoding="utf-8")
print(f"{len(fields)} settings fields and {len(enums)} enum label tables verified")
