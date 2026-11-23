# 共通Core・データ互換の照合

<!-- meltype-flash-document-banner:start -->
<p align="center"><img src="../../../assets/branding/readme/documentation.svg" alt="Meltype-FLASH — 稲妻のように、言葉の壁を駆け抜ける日本語入力。" width="1000"></p>
<!-- meltype-flash-document-banner:end -->

元版 `45e14f86558aa6ea34f2c2ecd038ba95ca8c3c46` の `src/Meltype.Core` を変更せず、実際の `MeltypeSession.CreateDefault`、`Settings`、各学習ストア、辞書、定型文、`Backup` API を呼び出す。試験データは指定した専用ディレクトリーだけに作る。利用中のプロファイルを指定しない。

```powershell
dotnet build tools/parity/core-data/CoreData.csproj
cargo +stable build -p meltype-core --example core_data --release --locked
python tools/parity/core-data/check.py --csharp tools/parity/core-data/bin/Debug/net10.0/Meltype.Core.Tests.dll --rust target/release/examples/core_data.exe --output dist/core-data-check
python tools/parity/core-data/freeze.py dist/core-data-check
cargo +stable test -p meltype-core --test upstream_core_data --test data_interop
```

`CARGO_TARGET_DIR` を指定した場合は実行ファイルの場所を合わせる。比較対象を別の作業ディレクトリーに展開する場合は `dotnet build -p:CoreProject=<元版Coreプロジェクトの絶対パス>` を使う。出力ディレクトリーは毎回新しくする。速度測定には使わない。

2026-10-20の採取では以下が一致した。

- 入力371列の各打鍵：消費したキー、確定編集、表示文字、変換状態、誤字提案。4判定レベル、スコア区切りON/OFF、英語の周辺文脈、周辺削除可否、独自ローマ字表、言語学習、削除後の再入力、保護された相対パス、複数確定をまたぐ言語学習を含む。
- 設定120ケース：既定値、既知のbool・数値・列挙・文字列設定、旧設定の移行、アプリルールの正規化、プロファイル追加・切替・名前変更・削除と拒否時の状態。Rustが将来互換のため保持する未知フィールドは、未知項目を捨てるC#との比較から除く。
- C#の7ファイルをRustで読む、Rustで変更したバックアップをC#で復元する、C#で変更したバックアップを既存Rust状態へ復元して読み直す、という3方向の意味上の一致。

7ファイルは `model.json`、`languages.json`、`conversions.json`、`translations.json`、`snippets.json`、`userdict.txt`、`phrases.txt`。Rustの正規状態は `state.json` のまま保持し、バックアップ時に最新状態からC#向けファイルを生成する。したがって学習削除後に古いコピーが再出現しない。C#バックアップの復元では対象カテゴリーだけを差し替え、既存の `state.json` に隠されないよう統合する。変更前ファイルは `.before-restore` に保存し、書込エラー時は書込済みの対象を戻す。電源断をまたぐ複数ファイルの原子性は保証しない。

データの比較対象は言語の選択・回数・有効性、選択済み変換、翻訳頻度、ユーザー辞書の候補順、定型文本文、フレーズ候補・件数、接頭辞学習のカウンター。日時そのものと設定はこのデータスナップショットには含めず、設定を別の120ケース、日時の小数保持・BOM・部分復元・不正データ・再起動をRustの専用テストで検査する。C#のタイムゾーン表記なし日時も読み込み、変更のない既存日時は小数部を保持する。

期待値はC#の採取結果だけから `rust/core/tests/data/upstream-core-data.json` に固定し、通常のワークスペーステストで再確認する。`freeze.py` は全比較が成功していない記録を拒否する。採取とホストのSHA-256は固定ファイルに残す。全UI、OS連携、全設定の組合せ、外部変換器・OSスペルの全候補、全入力言語の完全同等を証明する試験ではない。

最終修正後のWindows検証は全202テスト、ワークスペース／TSF-server Clippy、x64/x86の既存ABI回帰24本と追加371列を各ABIで成功。予測とSystem／制御Mozc helper／Hybridのワーカーも成功した。文脈は元版の `LanguageOf`／`LanguageOfStart`／`IsEnglishSentence` に従って直前・直後の文字種と英文を判定する。既存の時刻付き32列も一致。`I like. `の後のConservative `sushi` は元版の実セッションで `sushi` と確認できたため、以前の手書き期待値 `すし` を訂正した。失敗ログはローカルのdistに保持している。
