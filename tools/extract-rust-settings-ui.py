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
