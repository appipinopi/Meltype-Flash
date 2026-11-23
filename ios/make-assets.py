#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Regenerate the iOS asset catalog from the existing Flash mark (Pillow required)."""
import json
from pathlib import Path
from PIL import Image


def generate():
    root = Path(__file__).resolve().parent
    catalog = root / 'Assets.xcassets'
    source = Image.open(root.parent / 'assets/branding/meltype-flash-mark.png').convert('RGBA')
    # iOS app icons must be opaque; preserve the supplied artwork on white.
    mark = Image.new('RGBA', source.size, 'white')
    mark.alpha_composite(source)
    mark = mark.convert('RGB')
    info = {'version': 1, 'author': 'xcode'}

    def contents(folder, **data):
        folder.mkdir(parents=True, exist_ok=True)
        (folder / 'Contents.json').write_text(json.dumps(dict(data, info=info), indent=2) + '\n', encoding='utf-8')

    contents(catalog)
    icons = catalog / 'AppIcon.appiconset'
    images = []
    for idiom, sizes, scales in (
        ('iphone', [20, 29, 40, 60], [2, 3]),
        ('ipad', [20, 29, 40, 76], [1, 2]),
        ('ipad', [83.5], [2]),
        ('ios-marketing', [1024], [1]),
    ):
        for size in sizes:
            for scale in scales:
                name = f'icon-{int(size * scale)}.png'
                images.append({'idiom': idiom, 'size': f'{size}x{size}', 'scale': f'{scale}x', 'filename': name})
    contents(icons, images=images)
    for item in images:
        pixels = int(float(item['size'].split('x')[0]) * int(item['scale'][0]))
        mark.resize((pixels, pixels), Image.Resampling.LANCZOS).save(icons / item['filename'])
    brand = catalog / 'BrandMark.imageset'
    contents(brand, images=[{'idiom': 'universal', 'filename': 'mark.png'}])
    mark.resize((264, 264), Image.Resampling.LANCZOS).save(brand / 'mark.png')


if __name__ == '__main__':
    generate()
