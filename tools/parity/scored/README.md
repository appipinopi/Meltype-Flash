# スコア区切りの元版比較

<!-- meltype-flash-document-banner:start -->
<p align="center"><img src="../../../assets/branding/readme/documentation.svg" alt="Meltype-flash — 稲妻のように、言葉の壁を駆け抜ける日本語入力。" width="1000"></p>
<!-- meltype-flash-document-banner:end -->

`rust/core/tests/data/upstream-scored-keys.json` は、基準コミット
`46db6ca2f941eb9580f4d5ed103ee0bee71111c7`（本家v1.4.0）の実際のC# Coreから採取した87入力です。
元版の `ScoredSegmentationTests` の12入力と、既定入力の固定試験75入力について、
スコア区切りをONにして全打鍵を採取しました。期待値はRustの出力から作っていません。
ファイルに元版の実行ファイル・比較ホスト・Coreと辞書の全ソース・採取ログ・比較レポートのSHA-256を記録しています。
通常判定の既存75入力もv1.4.0と全打鍵が一致することを別途確認しています。

同梱スペル辞書は両版で有効、OSスペル辞書・学習・ライブ変換・予測は無効、周辺削除は可能です。
比較項目は消費状態、確定編集、表示文字、変換状態です。候補・注釈・画面描画・実アプリ入力の比較ではありません。

## 固定試験

リポジトリーのルートから実行します。

```powershell
cargo +stable test -p meltype-core --test upstream_scored_keys --locked
python rust/native/tests/scored_smoke.py <target>/debug/meltype_native.dll
```

Windows DLLの試験は一時ディレクトリー内だけで設定を読み書きします。IMEの登録・有効化は行いません。
設定OFFの75入力、独自表12入力、英数判定の時刻付き32入力は従来の別の固定試験で検査します。

## 元版を再ビルドして照合

以下の出力ディレクトリーがまだ存在しない状態で実行してください。`CoreProject` を省略すると、
比較ホストは作業ツリーのC#を使用します。基準コミットとの比較では必ず取り出したCoreを指定します。

```powershell
New-Item -ItemType Directory dist/scored-reference -ErrorAction Stop
git archive --format=tar --output=dist/scored-reference/baseline.tar 46db6ca2f941eb9580f4d5ed103ee0bee71111c7 src/Meltype.Core dictionaries
tar -xf dist/scored-reference/baseline.tar -C dist/scored-reference
$scoredCore = (Resolve-Path dist/scored-reference/src/Meltype.Core/Meltype.Core.csproj).Path
dotnet publish tools/performance/windows/Compare.csproj -c Release -r win-x64 -p:PublishAot=true "-p:CoreProject=$scoredCore" -o dist/scored-reference/csharp
cargo +stable build -p meltype-core --example benchmark --release --locked
python tools/check-core-parity.py --csharp dist/scored-reference/csharp/Compare.exe --rust <target>/release/examples/benchmark.exe --cases rust/core/tests/data/upstream-scored-keys.json --scored --baseline-ref 46db6ca2f941eb9580f4d5ed103ee0bee71111c7 --output dist/scored-parity --check
```

`--cases` は配列形式と固定試験の `cases` オブジェクト形式の両方に対応します。
`--scored` を指定すると全入力でスコア区切りをONにします。指定しなければ各入力の `scored` フィールド、
そのフィールドが無ければ従来どおりOFFを使用します。

全87入力の比較に成功した後だけ、次のコマンドで期待値を再生成できます。

```powershell
python tools/parity/scored/regenerate.py dist/scored-parity --csharp dist/scored-reference/csharp/Compare.exe --rust <target>/release/examples/benchmark.exe --reference-source dist/scored-reference
```

生成処理は比較の不一致・実行ファイルの差替え・本家ソースとGitの差を拒否し、期待値にはC#の採取結果だけを使います。
v1.4.0では`seeyo`の途中表示が`seeよ`に変わるため、古い固定値を本家の実出力から更新しています。
`Goog`の途中表示は`Goおg`、Enterでの確定は`Goog`です。未完の子音だけで語全体を英字へ戻さない回帰試験を追加しています。

## 残る範囲

Rustに長さの累乗による採点、未知語の低い点、隣接区間・切替・読めない文字の減点、
`sha/tha/dha` の途中で英語と助詞「は」を区切る探索を追加しました。
`MELTYPE_SCORED=1` と元版の4つの `MELTYPE_SCORED_*` 重みも読み込みます。

ただし、英字以外を含む入力は共有の従来判定へ委譲しており、元版の記号・数字・ドメイン等の
強制区間ルールを採点経路へ全て移植した状態ではありません。全文脈・全判定レベル、学習した語を
区間ごとに適用する判定、全独自表・候補・変換器との組合せも照合が残ります。
87件の一致を設定全体または全Coreの移行完了として扱わないでください。速度評価にも使いません。
