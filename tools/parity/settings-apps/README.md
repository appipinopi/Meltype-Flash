# アプリ別設定のC#比較

<!-- meltype-flash-document-banner:start -->
<p align="center"><img src="../../../assets/branding/readme/documentation.svg" alt="Meltype-FLASH — 稲妻のように、言葉の壁を駆け抜ける日本語入力。" width="1000"></p>
<!-- meltype-flash-document-banner:end -->

元版の Settings.Load/ForApp/ProfileFor/KindFor/IsAppEnabled/IsGame を直接呼び、384構成をRustの共通ポリシーと比較する。全て隔離ファイルで行う。実プロセスのゲーム判定やOSフックはこの試験の対象外。

```powershell
dotnet build tools/parity/settings-apps/SettingsApps.csproj
cargo build --release -p meltype-core --example core_data --locked
python tools/parity/settings-apps/check.py --csharp tools/parity/settings-apps/bin/Debug/net10.0/SettingsApps.dll --rust target/release/examples/core_data.exe --output dist/settings-apps-check
cargo test -p meltype-core --test upstream_settings_apps
```

固定期待値 `rust/core/tests/data/upstream-settings-apps.json` はC#出力由来。拡張子なし `matrix` と `matrix.exe` は異なる実行名として扱い、大文字小文字は区別しない。
