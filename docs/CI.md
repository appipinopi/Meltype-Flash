# CIの構成

Meltype-flashの検査は、次のGitHub Actionsワークフローで行います。

| ファイル | 役割 | 起動条件 |
| --- | --- | --- |
| `rust.yml` | Windows・Linux・macOSのRust検査、ABI・設定画面・参照実装との比較、Android検査の呼び出し | 関連ファイルのPR、手動 |
| `rust-android.yml` | JNI・APK・lint・実エミュレーターの検査 | Rust preview／リリースからの呼び出し、手動 |
| `fcitx-rust.yml` | Rust／従来版fcitx5アダプターのビルド、JSON・キー・状態・イベントABIの比較 | 関連ファイルのPR、Rust previewの手動実行からの呼び出し、単独の手動実行 |
| `macos-rust.yml` | Carbon・IMK・Swiftの統合検査とmacOSパッケージの検査 | 関連ファイルのPR、手動 |
| `chromeos.yml` | WASMとChromeOS拡張機能のビルド | 関連ファイルのPR、リリースからの呼び出し、手動 |
| `flash-identity.yml` | Flash固有の実行ファイル・登録ID・保存先・公開条件の保護 | rust／mainへのpush・PR、手動 |

Androidの手順は`rust-android.yml`だけで管理します。fcitx5の手順も`fcitx-rust.yml`だけで管理し、PRでは専用CI、Rust previewの手動実行では再利用呼び出しで検査します。

Rust previewは参照用C#コード、設定画面の検査スクリプト、CI補助スクリプト、ブランド画像の変更でも起動します。`docs/`内だけの変更では起動しません。PRの更新時は同じPRの古い検査を中止します。手動実行は進行中の実行を中止せず、Android・fcitx5は呼び出し元ごとに待機を分けます。

制限時間はRust desktopが90分、Androidが45分、fcitx5が30分、Flash identityが5分です。macOS SDKは既存の15分／30分を維持します。

## ベンチマークと画像

`rust.yml`のWindows desktopジョブで、C#の最新安定版・前のRust公開版・現在のRust版の共通Coreを比較します。Cargoのバージョンより前の公開タグを数値順に探します。現在はPre4なので`flash-v0.1.0-preview.3`が基準です。手動実行の`benchmark_baseline`でタグまたはコミットを指定することもできます。

両版を同じRustコンパイラでReleaseビルドします。現在版のRelease依存関係を利用し、前のRust版はCoreと測定ホストだけを追加ビルドします。C#の最新安定タグも自動選択し、.NET 10のNativeAOTでCoreと測定ホストを追加ビルドします。プロセスを交互に測定する前に両バイナリを固定し、元のビルド先が上書きされても影響を受けないようにします。各版のコード・依存関係・同梱辞書の違いを含む比較です。コンパイラ・コミット・辞書・ソース・バイナリのハッシュも保存します。

日本語・混在・英語・URL・編集の計21ケースで、各ケースを1回ウォームアップして5回測定します。C#→旧Rust→現在Rust、旧Rust→現在Rust→C#、現在Rust→C#→旧Rustの3巡で、各版・各ケース15試行です。リリース作成時は3試行×9巡です。外部変換・予測・学習・TSF・描画は対象外で、プロセス全体のCPU・RAMとキー処理の時間は区別します。

実行のSummaryに中央値・新/旧の時間比・比較対象の状態一致を表示します。`windows-common-benchmark`成果物から、次を取得できます（保存期間14日）。

- `charts/speed-32-64.png`、`speed-128-256.png`、`speed-short-edit.png`: 全21ケースの時間の分布
- `charts/cpu.png`、`charts/ram.png`: 各版3プロセスのCPU・メモリ観測値
- `report.json`、`*.jsonl`、`resources.json`、ビルドログ: 測定条件・ハッシュ・生データ
- `charts/summary.json`、CSV、統計JSON、`README.md`: 再描画用データと説明

箱ひげ図は全観測値で統計を計算し、表示範囲を自動調整します。大きく離れた外れ値は画像に件数、本文の表に値を記載します。動作が違うケースには`TRACE MISMATCH`を付け、同等動作の速度比較から除外します。ソースやバイナリが測定中に変わった場合は画像生成を失敗させます。共有ランナーの速度変動を考慮し、固定倍率での合否判定は行いません。

画像にはmatplotlib 3.10.8とNumPy 2.4.2を使います。依存関係を測定前に導入し、測定中は同じジョブで他のビルドを動かしません。測定ステップは35分（リリースは40分）で打ち切り、後続テストが失敗した場合も測定済みデータを保存します。成果物には基準ソースやバイナリを含めません。

測定専用のジョブやOSマトリックスは増やしません。3者比較は既存のWindowsジョブ内でPythonが自動実行し、整形確認はLinuxの1回にまとめます。従来の手動C#比較は`tools/benchmark-windows.py`で引き続き実行できます。

## 手元での確認

変更したワークフローに`actionlint`を実行します。加えて、Flashの識別情報を次で確認できます。

```powershell
python tools/check-flash-identity.py
python -m unittest discover -s tools -p test_flash_identity.py -v
```

各OSの統合検査はGitHub Actionsで確認します。リリース公開は引き続き手動操作で、`rust-release.yml`と`rust-distributions.yml`が担当します。上流C#版のビルド・配布ワークフローにあるリポジトリ条件は維持します。

## 整理時点で確認した既存の失敗

2026年10月22日の[実行39777313015](https://github.com/appipinopi/Meltype-Flash/actions/runs/39777313015)には、Windowsの`nihongoGitHub`の参照実装との分節不一致と、Androidの確認ダイアログで「復元」ボタンを検出できない失敗があります。今回の整理では該当テストを維持しています。CI設定の検証成功と、各OSの統合検査成功は別に確認してください。


新しい3者レポートからは、Rust前版・現在版の2者比較5枚（`rust-*.png`）と、C#・Rust前版・現在版の3者比較5枚（従来の画像名）の計10枚を生成します。両セットは同じ測定データを使い、それぞれに適した表示範囲を選びます。本文には両セットの外れ値表と画像10枚を掲載します。凡例と表には`Rust v0.1.0 pre3`、`Rust v0.1.0 pre4`、`Rust v0.1.0`のように実際の版を表示します。`v0.1.1 pre1`の前版は、公開タグに`v0.1.0`があればその正式版です。過去の2者レポートは引き続き5枚で再描画できます。
