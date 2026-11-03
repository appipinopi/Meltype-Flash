#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
case "$(uname -s)" in
  Darwin)
    cargo build --manifest-path "$root/Cargo.toml" --locked --release --workspace --target-dir "$root/target"
    MELTYPE_NATIVE_LIBRARY="$root/target/release/libmeltype_native.dylib" \
      MELTYPE_RUST_REGISTER_HELPER="$root/target/release/meltype-register-input-source" \
      MELTYPE_RUST_SELECT_HELPER="$root/target/release/meltype-select-input-source" \
      MELTYPE_RUST_CORE=1 MELTYPE_VERSION=1.0.1 bash "$root/mac/build.sh" --no-install
    ;;
  Linux)
    bash "$root/rust/build-linux.sh"
    ;;
  *) echo 'Run rust/build-windows.ps1 on Windows.' >&2; exit 1 ;;
esac
