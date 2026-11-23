# IMEベンチマーク (1万文コーパス)

Meltype-FLASHの変換精度を、カテゴリ別の固定コーパス1万文で測る仕組みです。
既存の固定文精度ベンチマーク (`rust/core/data/strict-accuracy.json`、30件) とは別に、
規模の大きいコーパスでカテゴリ別の弱点を見るためのものです。

## バージョンの指定方法

`run_benchmark.py` の `--version` / `--compare` と、GitHub Actions
(IMEベンチマーク) の手動実行入力で、同じ指定が使えます。

| 指定 | 意味 |
|---|---|
| `latestrelease` | 最新の正式リリース |
| `latestrelease-1` | 1つ前の正式リリース (`-2` は2つ前) |
| `1.1.26` | そのバージョン (`v1.1.26`、`flash-v1.1.26`、`flash-v0001.0001.0026` でも可) |
| `HEAD` | 現在のチェックアウト |
| URL | `https://github.com/appipinopi/Meltype-Flash/latestrelease` のようなURLもそのまま渡せる (`/releases/tag/<タグ>` 形式も可) |

通常のGitHubの画面で見る場合は、最新版は
`https://github.com/appipinopi/Meltype-Flash/releases/latest`、
特定版は `https://github.com/appipinopi/Meltype-Flash/releases/tag/flash-v0001.0001.0026`
です。「1つ前」を指すURLはGitHubに無いため、`latestrelease-1` はこのツールの指定として提供します。

## 自動測定

- リリースを公開すると、ワークフロー `IMEベンチマーク` が自動で動き、
  そのリリースと1つ前のリリースを測定して、結果の表を Actions の
  Step Summary に出し、レポート (report.json / report.md) を artifact に保存します。
- 手動実行 (workflow_dispatch) では、任意のバージョン同士の比較もできます。

## 測定の2層

- **Layer A (かな)**: コーパスの読みをローマ字で打ち、コアの確定文字列が
  読み (ひらがな) と一致するか。カテゴリ別の完全一致率と文字精度を出します。
  どのバージョンでも測定できます (対象タグのソースに
  `rust/core/examples/ime_benchmark.rs` を差し込んでビルドします)。
- **Layer B (漢字かな交じり)**: `expected` の漢字かな交じり文との一致。
  AIモデル等の変換層はコア単体では確定しないため、変換コマンドを
  `--converter-cmd` (または環境変数 `IME_BENCH_CONVERTER_CMD`) で接続した
  場合だけ測定します。入出力は JSONL (`{"id":..,"input":..}` →
  `{"id":..,"actual":..}`) です。未設定の場合は表に「未測定」と明記されます。

## ローカル実行

```bash
# バージョン解決だけ確認
python tools/ime-benchmark/run_benchmark.py --version latestrelease-1 --resolve-only

# 最新版と1つ前を比較
python tools/ime-benchmark/run_benchmark.py --version latestrelease --compare latestrelease-1

# 開発中 (HEAD) を測定
python tools/ime-benchmark/run_benchmark.py --version HEAD
```

## コーパス

- `data/ime_benchmark_2026-11.part*.jsonl` (3分割、全10,000件):
  基本変換1000・複合語1000・固有名詞1000・難読趣味ネットスラング750・
  英日混在2000・特殊入力2000・実用文章2250。
  各行は `input` (ひらがな読み) と `expected` (期待する変換結果) のペア。
  GitHubのファイルサイズ制限のため分割してあり、測定時は自動で結合されます。
- `generate_benchmark.py`: コーパスの生成スクリプト。月替わりで
  `--month 2026-12` のように実行すると、シードが変わった新しいセットを生成します。
- `kana2romaji.py`: 読み→ローマ字変換。エンジン自身のローマ字表
  (`rust/core/src/romaji.tsv`) の逆引きに基づきます。

## コア比較ベンチ (第2弾 / mode=core)

Meltype 本家・フォーク・派生・独立実装の「コア」を、同じ 1 万文コーパスで比べる仕組みです。
対象は `core_targets.json`(31 対象、対象外 18 件の理由つき)。

- Layer A (変換器なし): ローマ字を打鍵し、Enter で確定した文字列が読み (かな) と一致するか。
  ローマ字・かな・英日境界・保護の判定だけを見る。
- Layer B (共通 Mozc): 本家 v1.7.1 の Linux 配布物に同梱の `meltype_mozc_helper` を
  同じ条件でつなぎ、Space 変換の表示または Enter 確定が `expected` と一致するか。
  C# の各版・フォークと MonakIME が同じヘルパーを共有します。
  Meltype-Flash はコア同梱の `bundled-mozc` (移植 Mozc 辞書) で変換します。
- Zenzai・Jev・azooKey・Google 日本語入力などの AI 層はすべて無効にします。

```bash
# 対象一覧 (人が読む形)
python tools/ime-benchmark/core_bench_plan.py
# 1 対象だけ測る (手元で配線を確認するとき)
LIMIT=200 python tools/ime-benchmark/core_bench_runner.py --target upstream_v171 --out bench-out/upstream-v1.7.1.json
```

C# は版ごとにセッション API が違うため、`core_bench_runner.py` がソースを見て
`harness/csharp/Program.gen{1,2,3}.cs` を選び、ビルドできなければ次の候補を試します。
結果 JSON には `status`(ok/partial/error)・`failure`(段階とログ)・`harness`(実際に通ったもの) が入り、
失敗してもジョブは落ちません。

## テスト

```bash
python -m unittest discover -s tools -p test_ime_benchmark.py
python -m unittest discover -s tools -p test_core_benchmark.py
```
