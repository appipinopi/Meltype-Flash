#!/usr/bin/env python3
"""Collect the required platforms from one successful workflow run."""
import hashlib
from pathlib import Path
import shutil
import sys


def collect(source: Path, output: Path):
    assets = {
        'Meltype-flash-Windows-x64.zip': 'release-windows/Meltype-flash-Windows-x64.zip',
        'Meltype-flash-Linux-x64.tar.gz': 'release-linux/Meltype-flash-Linux-x64.tar.gz',
        'Meltype-flash-macOS-arm64.zip': 'release-macos-arm64/Meltype-flash-macOS-arm64.zip',
        'Meltype-flash-macOS-x64.zip': 'release-macos-x64/Meltype-flash-macOS-x64.zip',
        'Meltype-flash-ChromeOS.zip': 'Meltype-ChromeOS-preview/chromeos-preview.zip',
        'Meltype-flash-iOS-Simulator-arm64.zip': 'release-ios/Meltype-flash-iOS-Simulator-arm64.zip',
        'Meltype-flash-iOS-unsigned.ipa': 'release-ios/Meltype-flash-iOS-unsigned.ipa',
        'Meltype-flash-iOS-source.zip': 'release-ios/Meltype-flash-iOS-source.zip',
    }
    apk = list((source / 'meltype-rust-android-debug').rglob('app-debug.apk'))
    if len(apk) != 1:
        raise ValueError(f'Expected one Android application APK, found {len(apk)}')
    inputs = {name: source / path for name, path in assets.items()}
    inputs['Meltype-flash-Android-debug.apk'] = apk[0]
    for path in inputs.values():
        if not path.is_file() or path.stat().st_size == 0:
            raise ValueError(f'Missing or empty asset: {path}')
    output.mkdir(parents=True, exist_ok=False)
    hashes = []
    for name, path in sorted(inputs.items()):
        shutil.copyfile(path, output / name)
        digest = hashlib.sha256((output / name).read_bytes()).hexdigest()
        hashes.append(f'{digest}  {name}\n')
    (output / 'SHA256SUMS.txt').write_text(''.join(hashes), encoding='utf-8')


if __name__ == '__main__':
    collect(Path(sys.argv[1]), Path(sys.argv[2]))
