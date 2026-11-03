# 長文・英語境界の全打鍵比較

<!-- meltype-flash-document-banner:start -->
<p align="center"><img src="../../../assets/branding/readme/documentation.svg" alt="Meltype-FLASH — 稲妻のように、言葉の壁を駆け抜ける日本語入力。" width="1000"></p>
<!-- meltype-flash-document-banner:end -->

基準は元版 `45e14f86558aa6ea34f2c2ecd038ba95ca8c3c46` の C# NativeAOT。
`rust/core/tests/data/upstream-long-keys.json` は、21種類のベンチマーク入力と42種類の短い境界入力を元版で実行して保存した期待値です。期待値をRustから作成しません。採取した実行ファイルのSHA-256を同じJSONに記録しています。

条件は Balanced、ScoredSegmentation OFF、周辺文字削除可、同梱スペル辞書ON、OSスペルAPI／学習／予測／ライブ変換OFF。各キーのconsumed、確定編集、表示文字、convertingを比較します。候補一覧、UI描画、全設定の互換性や実アプリのTSF入力は検証対象外です。

21種類は日本語・混在・英語・URLの32/64/128/256文字、短文4種類、削除後のカタカナ切替。42種類は日本語／英語の後の短い英単語、末尾の子音、単独a/i、thanの後の日本語の音を含みます。

通常のRust検査:

```powershell
cargo +stable test -p meltype-core --test upstream_long_keys --locked
```

Windows DLLでも同じ期待値を検査します。実行時のデータは一時ディレクトリーに作成し、実IMEへの登録やキー入力は行いません。

```powershell
cargo +stable build -p meltype-native --locked
python rust/native/tests/scored_smoke.py target/debug/meltype_native.dll --golden rust/core/tests/data/upstream-long-keys.json
```

DLLのパスは`CARGO_TARGET_DIR`に応じて変更してください。CIでは各OSの既定releaseライブラリーを読みます。

新しい元版ホストとの再照合（出力先は毎回新しい名前）:

```powershell
python tools/check-core-parity.py --csharp <Compare.exe> --rust <benchmark.exe> --cases rust/core/tests/data/upstream-long-keys.json --output dist/long-parity-new --check
```

ホストのビルド方法と速度測定の条件は [Windowsベンチマーク](../../performance/windows/README.md) を参照してください。互換性の検査と速度測定は別々に行います。
