# Meltype 1.3.0の参照比較

<!-- meltype-flash-document-banner:start -->
<p align="center"><img src="../../../assets/branding/readme/documentation.svg" alt="Meltype-FLASH — 稲妻のように、言葉の壁を駆け抜ける日本語入力。" width="1000"></p>
<!-- meltype-flash-document-banner:end -->

対象タグは `v1.3.0`、コミット `6310c44b728d5ad88284853f9ef37ea224993b18`。`python tools/parity/v130/regenerate.py` でタグのCore・辞書を一時フォルダーへ展開し、元ソースを変更せず.NET 10でビルドして応答を再生成する。既存のC#ソースや利用者の設定は変更しない。

`Reference.csproj` は元Coreのfriend assembly名を使って内部APIを呼ぶ比較用ツールであり、配布製品ではない。`SessionReference.cs` はWindowsのCompositionServiceと同じ設定デリゲートを接続し、OS辞書は制御したconverterへ置き換える。既定の共有セッションfactoryはPredictor/ShowTypedKeys等の接続がWindowsと異なるため使用しない。

- 472例: 数字、計算、元号、単位、季節、クリップボードの1行検査、キー設定、英字名、GitHubリポジトリ形式。
- 79入力列と追加65列: 入力表示、候補一覧、候補クリック、確定、文節操作、設定ON/OFF、文脈付き学習の読込。実OS変換エンジンやネットワークは制御した応答。
- Rustの `upstream_v130_sessions` は、ヒント・選択番号・typed・committedの画面情報を比較から除く。全画面同等とは呼ばない。
- `known-gaps.json` は途中表示の保持境界の既知差分。通過群から分けて未対応として保持する。

検査: `cargo +stable test -p meltype-core --test upstream_v130_values --test upstream_v130_sessions --test v130_learning --locked`

全機能の残りは [UPGRADE-1.3.0.md](../../../rust/UPGRADE-1.3.0.md) を参照。

追加65列は文節選択後のF6〜F10連打・部分確定・Escの組合せ・記号・末尾wを対象とする。句読点の仮想キーは実際のOEMキーとShiftを使う。設定の120ケースと追加28既定値は元1.3.0を参照し、廃止済みAutoSwitch/Bothの旧仕様を正解にしない。括弧内へのcaretBackは元のSession JSONが公開しないホスト操作なので、別のRust/TSF COM試験で確認する。

設定120ケースはbaseSettingsと各時点のsettingsDeltaで省略なく復元する形式。再生成時に元の全スナップショットと復元結果の一致を検査し、既定値の重複を減らす。
