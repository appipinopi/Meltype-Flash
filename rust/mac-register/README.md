# macOS installer tools in Rust

<!-- meltype-flash-document-banner:start -->
<p align="center"><img src="../../assets/branding/readme/documentation.svg" alt="Meltype-FLASH — 稲妻のように、言葉の壁を駆け抜ける日本語入力。" width="1000"></p>
<!-- meltype-flash-document-banner:end -->

`meltype-register-input-source` replaces the standalone Swift registration tool.
It keeps its optional app-path argument, Japanese messages, OSStatus failure,
30 checks with 100 ms waits, enabled-bundle lookup and best-effort
`TextInputMenuAgent` refresh. It is independent of the IME, native-core dylib,
azooKey and Swift runtime. `meltype-select-input-source` replaces the installer
Swift selection script: it checks enabled sources up to ten times, waits 300 ms
on failure and confirms the selected source before reporting success.

Build on macOS:

```sh
cargo build --release --locked -p meltype-mac-register
```

The Rust desktop build includes both tools as
`Meltype.app/Contents/MacOS/MeltypeRegisterInputSource` and
`Meltype.app/Contents/MacOS/MeltypeSelectInputSource`. `mac/install.sh` prefers
the bundled selector. Legacy packages keep their Swift fallback. The Rust path
builds only the `MeltypeIME` Swift product, so the former Swift registration
executable is not part of that package.

Platform operations use Foundation and Carbon directly from Rust. UTF-16 string
construction preserves leading BOM/NUL/supplementary characters. Carbon lists
and current-source copies are released on all exits; bundle-ID equality uses
canonical Unicode equivalence as the former Swift code does.

Validation separates its scope:

- Six portable control tests cover registration errors, absent bundle IDs, retry
  boundaries, best-effort refresh errors, duplicate selectable sources,
  read-back mismatch and cleanup. Two macOS SDK tests exercise real Foundation
  child processes (true/false/missing executables) and string construction.
- `mac/test/register-helper/run.sh` compares the actual Rust CLI with the former
  Swift helper for four missing-path cases and extra-argument handling. These
  paths never reach Carbon or agent restart. Direct dynamic-library dependencies
  must not include Swift, the native core or azooKey.
- The macOS workflow packages and signs the real app, checks the bundled helper
  by execution/protocol, checks both tools' linked libraries and runs the
  original product self-test with isolated app data. It does not install/register
  an IME or select an OS input source. CI evidence for the new selection/package
  self-test changes is pending.
