#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
for pair in aarch64-apple-ios:iphoneos aarch64-apple-ios-sim:iphonesimulator; do
    target="${pair%%:*}"; platform="${pair##*:}"
    cargo build -p meltype-ios --release --locked --target "$target"
    mkdir -p "ios/lib/$platform"
    cp "target/$target/release/libmeltype_ios.a" "ios/lib/$platform/"
done
xcodegen generate --spec ios/project.yml
for sdk in iphoneos iphonesimulator; do
    xcodebuild -project ios/MeltypeFlash.xcodeproj -scheme MeltypeFlash -sdk "$sdk" \
        -configuration Release -derivedDataPath "$root/ios/build/$sdk" \
        CODE_SIGNING_ALLOWED=NO build
done
mkdir -p dist
ditto -c -k --sequesterRsrc --keepParent ios/build/iphonesimulator/Build/Products/Release-iphonesimulator/MeltypeFlash.app dist/Meltype-flash-iOS-Simulator-arm64.zip
python3 ios/package.py ios/build/iphoneos/Build/Products/Release-iphoneos/MeltypeFlash.app dist/Meltype-flash-iOS-unsigned.ipa
echo 'Built unsigned device and simulator applications. A device build needs Apple signing before installation.'
