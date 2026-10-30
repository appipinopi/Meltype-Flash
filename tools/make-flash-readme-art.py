# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Flashの6環境のREADME画像を、共通の雷ロゴと字形から再生成する。

python tools/make-flash-readme-art.py --font /path/to/NotoSansJP.ttf --write-readmes
必要なもの: fonttools。Noto Sans JP (SIL OFL 1.1) の文字をパスにする。
既存のPNG原本・アプリアイコン・上流の画像生成スクリプトは変更しない。
"""
import argparse
import base64
import hashlib
import html
import os
from pathlib import Path
import re
import subprocess
import tomllib

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "assets/branding/readme"
INK, YELLOW, GRAY = "#17181b", "#ffe34a", "#666973"
PLATFORMS = [
    ("windows", "Windows", "Windows 10 / 11", "Rust版の設定・IME", "rust/WINDOWS-PACKAGE.md"),
    ("linux", "Linux", "IBus / x64・ARM64", "デスクトップ向け日本語入力", "linux/README.md"),
    ("mac", "Mac", "macOS / Input Method Kit", "入力ソースとして使う", "mac/README.md"),
    ("android", "Android", "Android IME / QWERTY", "Android向けキーボード", "android/README.md"),
    ("chromeos", "ChromeOS", "ChromeOS専用 / Rust + WASM", "chrome.input.ime", "chromeos/README.md"),
    ("ios", "iOS", "iPhone / iPad", "カスタムキーボード", "ios/README.md"),
]
START, END = "<!-- meltype-flash-platform:start -->", "<!-- meltype-flash-platform:end -->"


class Art:
    def __init__(self, font):
        self.font = TTFont(font)
        self.glyphs = self.font.getGlyphSet(location={"wght": 650})
        self.cmap = self.font.getBestCmap()
        self.upm = self.font["head"].unitsPerEm
        self.mark = base64.b64encode((ROOT / "assets/branding/meltype-flash-256.png").read_bytes()).decode()
        self.logo = base64.b64encode((ROOT / "assets/branding/meltype-flash-logo.png").read_bytes()).decode()

    def text(self, text, x, y, size, fill=INK, max_width=None):
        names = []
        for char in text:
            if ord(char) not in self.cmap:
                raise ValueError(f"フォントに文字がありません: {char!r}")
            names.append(self.cmap[ord(char)])
        units = sum(self.glyphs[name].width for name in names)
        scale = size / self.upm
        if max_width and units * scale > max_width:
            scale = max_width / units
        pen = SVGPathPen(self.glyphs)
        for name in names:
            self.glyphs[name].draw(TransformPen(pen, (scale, 0, 0, -scale, x, y)))
            x += self.glyphs[name].width * scale
        return f'<path fill="{fill}" d="{pen.getCommands()}"/>'

    def symbol(self, x, y, size):
        return (f'<svg x="{x}" y="{y}" width="{size}" height="{size}" viewBox="30 40 196 175">'
                f'<image width="256" height="256" href="data:image/png;base64,{self.mark}"/></svg>')

    @staticmethod
    def bolt(x, y, size=1, fill=YELLOW):
        return f'<path transform="translate({x} {y}) scale({size})" fill="{fill}" d="M25 0 0 32H17L10 56 43 20H24L34 0Z"/>'

    @staticmethod
    def save(path, width, height, title, body):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
            f'viewBox="0 0 {width} {height}" role="img" aria-label="{html.escape(title, quote=True)}">'
            f'<title>{html.escape(title)}</title><rect width="100%" height="100%" fill="#fff"/>{body}</svg>\n', encoding="utf-8")

    def common_logo(self):
        self.save(OUT / "logo.svg", 1000, 240, "Meltype-flash — Mと稲妻のロゴ",
                  '<svg x="90" y="16" width="820" height="208" viewBox="140 182 1904 484">'
                  f'<image width="2172" height="724" href="data:image/png;base64,{self.logo}"/></svg>')

    def hero(self, key, name, sub, detail, _):
        body = (f'<rect x="1" y="1" width="998" height="298" rx="24" fill="{INK}"/>'
                f'<path d="M25 1H975Q999 1 999 25V33H1V25Q1 1 25 1Z" fill="{YELLOW}"/>'
                + self.text("MELTYPE-FLASH / PLATFORM", 48, 79, 16, "#b8bac3")
                + self.text(f"Meltype-flash for {name}", 48, 153, 48, "#fff", 640)
                + self.text(sub, 48, 203, 22, "#fff", 600)
                + self.text(detail + " ・ 開発プレビュー", 48, 253, 15, "#b8bac3", 600)
                + '<rect x="736" y="62" width="210" height="210" rx="28" fill="#fff"/>'
                + self.symbol(755, 74, 172)
                + self.bolt(913, 212, .85))
        self.save(OUT / key / "hero.svg", 1000, 300, f"Meltype-flash for {name}", body)

    def document_banner(self):
        motto = "稲妻のように、言葉の壁を駆け抜ける日本語入力。"
        body = (f'<rect x="1" y="1" width="998" height="218" rx="24" fill="{INK}"/>'
                f'<path d="M25 1H975Q999 1 999 25V29H1V25Q1 1 25 1Z" fill="{YELLOW}"/>'
                + self.text("MELTYPE-FLASH / README", 40, 65, 16, "#b8bac3")
                + self.text("Meltype-flash", 40, 127, 48, "#fff", 740)
                + self.text(motto, 40, 178, 23, "#fff", 745)
                + self.bolt(850, 58, 2.3))
        self.save(OUT / "documentation.svg", 1000, 220, f"Meltype-flash — {motto}", body)

    def card(self, key, name, sub, detail, _, active=False):
        body = (f'<rect x="1.5" y="1.5" width="317" height="187" rx="18" fill="{"#fffbe8" if active else "#fff"}" stroke="{INK if active else "#d9dade"}" stroke-width="{3 if active else 1}"/>'
                + self.symbol(24, 20, 46) + self.bolt(279, 22, .45)
                + self.text(name, 24, 108, 31, max_width=270)
                + self.text(sub, 24, 140, 13, GRAY, 270)
                + self.text("このページ" if active else "READMEを見る", 24, 171, 12, GRAY)
                + f'<path d="M276 162H293M286 155L293 162 286 169" stroke="{INK}" stroke-width="2" fill="none"/>'
                + (f'<rect x="24" y="119" width="50" height="3" rx="1.5" fill="{YELLOW}"/>' if active else ""))
        self.save(OUT / "cards" / f"os-{key}{'-active' if active else ''}.svg", 320, 190, f"{name} — {sub}", body)

    def heading(self, key, title, index, small=False):
        digest = hashlib.sha256(f"{index}:{title}".encode()).hexdigest()[:12]
        path = OUT / key / "headings" / f"{digest}.svg"
        body = (f'<rect x="1" y="1" width="998" height="78" rx="12" fill="#fff" stroke="#dedfe3"/>'
                f'<rect x="1" y="1" width="66" height="78" rx="12" fill="{YELLOW}"/>'
                + self.bolt(22, 20, .7, INK)
                + self.text(title, 88, 52, 29, max_width=814)
                + self.text(f"{index:02}", 936, 51, 18, GRAY))
        self.save(path, 1000, 80, title, body)
        return path


def relative(target, document):
    return Path(os.path.relpath(target, document.parent)).as_posix()


def document_banners(write):
    """OS別ヘッダー以外の全READMEと指定した導入文書に雷画像を置く。"""
    paths = subprocess.run(
        ['git', '-C', str(ROOT), 'ls-files', '-z', '--', '*README.md',
         'chromeos/docs/INSTALLATION.md'],
        check=True, capture_output=True).stdout.decode('utf-8').split('\0')
    handled = {'README.md', *(p[4] for p in PLATFORMS)}
    start = '<!-- meltype-flash-document-banner:start -->'
    end = '<!-- meltype-flash-document-banner:end -->'
    count = 0
    for file in paths:
        if not file or file in handled:
            continue
        document = ROOT / file
        original = document.read_text(encoding='utf-8')
        image = relative(OUT / 'documentation.svg', document)
        block = (f'{start}\n<p align="center"><img src="{image}" '
                 'alt="Meltype-flash — 稲妻のように、言葉の壁を駆け抜ける日本語入力。" '
                 f'width="1000"></p>\n{end}')
        if start in original:
            text = re.sub(re.escape(start) + r'.*?' + re.escape(end),
                          lambda _: block, original, count=1, flags=re.S)
        else:
            title = re.match(r'(# [^\n]+\n)', original)
            if title:
                text = title[1] + '\n' + block + '\n' + original[title.end():]
            else:
                text = block + '\n\n' + original
        if write and text != original:
            document.write_text(text, encoding='utf-8')
        count += 1
    print(f'folder READMEs: {count} banners')


def navigation(document, current=None):
    rows = []
    for offset in (0, 3):
        cards = []
        for key, name, _, _, file in PLATFORMS[offset:offset+3]:
            target = relative(ROOT / file, document)
            image = relative(OUT / "cards" / f"os-{key}{'-active' if current == key else ''}.svg", document)
            cards.append(f'  <a href="{target}"><img src="{image}" alt="{name}" width="32%"></a>')
        rows.append('<p align="center">\n' + '\n'.join(cards) + '\n</p>')
    return '\n'.join(rows)


def header(document, key, name):
    version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
    match = re.fullmatch(r"(\d+\.\d+\.\d+)-pre\.(\d+)", version)
    if not match:
        raise ValueError("Flashのプレリリース版が必要です")
    tag = f"flash-v{match[1]}-preview.{match[2]}"
    label = f"v{match[1]} Pre{match[2]}"
    badge_label = label.replace(" ", "_")
    return (f'{START}\n<p align="center"><img src="{relative(OUT / "logo.svg", document)}" alt="Meltype-flash" width="520"></p>\n'
            f'<a id="meltype-for-{key}"></a>\n'
            f'<p align="center"><img src="{relative(OUT / key / "hero.svg", document)}" alt="Meltype-flash for {name}" width="1000"></p>\n'
            '<p align="center"><b>稲妻のように、言葉の壁を駆け抜ける日本語入力。</b></p>\n'
            f'<p align="center"><a href="https://github.com/appipinopi/Meltype-Flash/releases/tag/{tag}">'
            f'<img src="https://img.shields.io/badge/Flash-{badge_label}-ffe34a" alt="Flash {label}"></a> '
            '<img src="https://img.shields.io/badge/status-開発プレビュー-666973" alt="開発プレビュー"></p>\n'
            '<p align="center"><a href="#インストール">ダウンロード・導入</a> · '
            '<a href="#使い方">使い方</a> · <a href="#困ったとき">困ったとき</a> · '
            '<a href="https://github.com/appipinopi/Meltype-Flash/issues/new/choose">不具合の報告・提案</a></p>\n'
            + navigation(document, key) + f'\n<p align="center"><a href="{relative(ROOT / "README.md", document)}">トップのREADME</a></p>\n{END}\n')


def process_readme(art, platform, write):
    key, name, _, _, file = platform
    document = ROOT / file
    original = document.read_text(encoding="utf-8")
    text = original
    if START in text:
        text = re.sub(re.escape(START) + r".*?" + re.escape(END), lambda _: header(document, key, name).rstrip(), text, count=1, flags=re.S)
    elif key in ("mac", "linux") and 'images/cards/os-linux.svg' in text:
        last = text.index('</p>', text.index('images/cards/os-linux.svg')) + len('</p>')
        text = header(document, key, name) + text[last:]
    else:
        text = header(document, key, name) + re.sub(r'^# [^\n]+\n', '', text, count=1)
    index = 0
    slugs = {}

    def heading_image(title, small):
        nonlocal index
        index += 1
        path = art.heading(key, html.unescape(title), index, small)
        return f'<img src="{relative(path, document)}" alt="{html.escape(html.unescape(title), quote=True)}" width="1000">'

    def replace_image(match):
        return heading_image(match[2], '/s' in match[1])

    def replace_markdown(match):
        title = match[2].strip()
        # 明示したアンカーで既存の目次リンクを保つ。
        slug = re.sub(r'[^\w\- ]', '', title.lower()).replace(' ', '-')
        count = slugs.get(slug, 0)
        slugs[slug] = count + 1
        if count:
            slug += f'-{count}'
        level = len(match[1])
        return f'<a id="{slug}"></a>\n<h{level}>{heading_image(title, level > 2)}</h{level}>'

    # コード例の中のMarkdownは画像化しない。
    parts = re.split(r'(^```[^\n]*\n.*?^```[^\n]*$)', text, flags=re.M | re.S)
    for pos in range(0, len(parts), 2):
        parts[pos] = re.sub(r'<img src="([^"]*/headings/[^\"]+\.svg)" alt="([^"]+)"[^>]*>', replace_image, parts[pos])
        parts[pos] = re.sub(r'^(#{2,3}) ([^\n]+)$', replace_markdown, parts[pos], flags=re.M)
    text = ''.join(parts)
    if write and text != original:
        document.write_text(text, encoding="utf-8")
    return index


def preview():
    sections = []
    for key, name, _, _, _ in PLATFORMS:
        document = ROOT / next(p[4] for p in PLATFORMS if p[0] == key)
        first = re.search(r'<img src="([^"]*/headings/[^"]+\.svg)"', document.read_text(encoding='utf-8'))
        sample = (document.parent / first[1]).resolve().relative_to(OUT).as_posix() if first else ''
        sections.append(f'<section><h2>{name}</h2><img src="{key}/hero.svg"><img src="{sample}"></section>')
    samples = []
    for key, title in [("windows", "内容"), ("linux", "動く環境"), ("linux", "インストール")]:
        document = ROOT / next(p[4] for p in PLATFORMS if p[0] == key)
        text = document.read_text(encoding='utf-8')
        match = re.search(r'<img src="([^"]*/headings/[^"]+\.svg)" alt="' + re.escape(title) + r'"', text)
        if match:
            sample = (document.parent / match[1]).resolve().relative_to(OUT).as_posix()
            samples.append(f'<img src="{sample}" alt="{title}">')
    content = ('<!doctype html><html lang="ja"><meta charset="utf-8"><link rel="icon" href="data:,"><title>Meltype-flash — 6環境のREADME画像</title>'
               '<style>*{box-sizing:border-box}body{margin:0;padding:40px;background:#eeeef0;color:#17181b;font-family:system-ui}'
               'main{max-width:1280px;margin:auto}.logo{display:block;width:520px;max-width:100%;margin:0 auto 24px}'
               'h1{font-size:16px;text-align:center;margin:0 0 24px;color:#8a96ae}h2{font-size:14px;margin:0 0 10px}'
               '.grid{display:grid;grid-template-columns:1fr 1fr;gap:26px}section img{display:block;width:100%;margin-bottom:12px}'
               '.cards{display:grid;grid-template-columns:repeat(3,1fr);gap:16px;margin-top:24px}.cards img{width:100%}'
               '.headings{margin:36px auto 0;max-width:1000px}.headings img{display:block;width:100%;margin-top:16px}'
               '@media(max-width:640px){body{padding:16px}.grid{grid-template-columns:1fr}.cards{grid-template-columns:repeat(2,1fr)}}</style>'
               '<main><img class="logo" src="logo.svg"><h1>ONE FLASH. SIX PLATFORMS.</h1><div class="grid">'
               + ''.join(sections) + '</div><div class="cards">'
               + ''.join(f'<img src="cards/os-{key}.svg" alt="{name}">' for key,name,*_ in PLATFORMS)
               + '</div><div class="headings">' + ''.join(samples) + '</div></main></html>')
    (OUT / 'preview.html').write_text(content, encoding='utf-8')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--font', default=os.environ.get('MELTYPE_README_FONT', 'C:/Windows/Fonts/NotoSansJP-VF.ttf'))
    parser.add_argument('--write-readmes', action='store_true')
    args = parser.parse_args()
    art = Art(args.font)
    art.common_logo()
    art.document_banner()
    for platform in PLATFORMS:
        art.hero(*platform)
        art.card(*platform)
        art.card(*platform, active=True)
        count = process_readme(art, platform, args.write_readmes)
        print(f'{platform[0]}: {count} headings')
    if args.write_readmes:
        document = ROOT / 'README.md'
        text = document.read_text(encoding='utf-8')
        nav_start, nav_end = '<!-- meltype-flash-os-navigation:start -->', '<!-- meltype-flash-os-navigation:end -->'
        separate_navigation = nav_start in text
        block = START + '\n![Meltype-flash](assets/branding/readme/logo.svg)\n\n' + ('' if separate_navigation else navigation(document) + '\n') + END
        if START in text:
            text = re.sub(re.escape(START) + r'.*?' + re.escape(END), lambda _: block, text, count=1, flags=re.S)
        else:
            text = text.replace('![Meltype-flash](assets/branding/readme-banner.svg)', block)
        if separate_navigation:
            text = re.sub(re.escape(nav_start) + r'.*?' + re.escape(nav_end),
                          lambda _: nav_start + '\n' + navigation(document) + '\n' + nav_end, text, count=1, flags=re.S)
        root_index = 0
        def root_heading(match):
            nonlocal root_index
            root_index += 1
            title = html.unescape(match['title'] or match['alt'])
            image = art.heading('overview', title, root_index)
            tag = f'<img src="{relative(image, document)}" alt="{html.escape(title, quote=True)}" width="1000">'
            if match['title']:
                slug = re.sub(r'[^\w\- ]', '', title.lower()).replace(' ', '-')
                return f'<a id="{slug}"></a>\n<h2>{tag}</h2>'
            return tag
        text = re.sub(r'^## (?P<title>[^\n]+)$|<img src="assets/branding/readme/overview/headings/[^\"]+\.svg" alt="(?P<alt>[^"]+)"[^>]*>', root_heading, text, flags=re.M)
        document.write_text(text, encoding='utf-8')
    document_banners(args.write_readmes)
    preview()


if __name__ == '__main__':
    main()
