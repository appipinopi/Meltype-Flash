# Core・各OSのベンチマーク

`platform-benchmarks.yml` は Windows・macOS・Linux のCIで、本家C# 1.4.0（`46db6ca2f941eb9580f4d5ed103ee0bee71111c7`）・Rust前版・Rust現在版のCoreを比較し、続けてRustのネイティブC ABIを比較します。21種類の同じ入力を使い、版ごとの状態の一致、ソース・バイナリのSHA-256、CPU時間、最大常駐メモリを記録します。CIは本家をこのコミット、現在版をHEADへ固定します。

```sh
python -m pip install matplotlib==3.10.8 numpy==2.4.2
python tools/compare-rust-benchmarks.py --build --with-csharp --samples 3 --warmups 1 --rounds 3 --plot --output output/desktop-core
python tools/compare-rust-benchmarks.py --build --backend native --samples 3 --warmups 1 --rounds 2 --plot --output output/desktop-native
python tools/benchmark-platforms.py --input output --output output/platform-coverage
```

前版はCargoの現在版より古い最新のRustタグを選びます。pre4ならpre3、0.1.1 pre1なら0.1.0です。図には実際の版を表示します。CoreはC#を含む10枚（Rustのみ5枚＋3者比較5枚）、ネイティブC ABIはRustのみ5枚です。各パネルの軸を調整し、大きく離れた外れ値を本文の表へ記載します。元の観測値は残します。

WindowsはプロセスのCPU・メモリをWin32で取得します。macOS・Linuxは`wait4`でその子プロセスだけのCPU時間と最大RSSを取得します。Linuxの平均RSSは50ms間隔のサンプルです。macOSの平均RSS、POSIXのprivate bytesは未測定とします。Windowsのworking setとPOSIXのRSSはOSごとに定義が異なるため、OS横断の順位や倍率は出しません。CPU・RAMは起動・セッション作成・ウォームアップ・JSON出力も含み、キー処理だけの値ではありません。

ネイティブC ABIは版ごとの一時プロファイルを作成し、ユーザーの設定や辞書へ触れません。区間内はキー処理・C文字列のコピーと解放です。設定とセッション作成は区間外。ライブ変換・予測・学習を無効にします。TSF・InputMethodKit・IBus、候補画面、アプリへの反映は測定範囲外です。

Android・iOS・ChromeOSは実機が利用できるまで未測定です。デスクトップCoreの測定やクロスコンパイルを、そのOSの実測として扱いません。集計には6 OSすべてとIME全体・実機での入力の欄を残します。

CIは「ベンチマーク」から手動実行します。`benchmark_baseline`は通常`auto`で、タグやコミットを明示することもできます。実験用ブランチへのpushでは自動実行しません。結果は3日間のCI成果物で受け渡し、リリースAssetsへ添付しません。測定用のRustホストは隔離したソースへコピーし、製品の入力処理へ測定コードを追加しません。
