# 英数入力中の時刻付き比較

<!-- meltype-flash-document-banner:start -->
<p align="center"><img src="../../../assets/branding/readme/documentation.svg" alt="Meltype-FLASH — 稲妻のように、言葉の壁を駆け抜ける日本語入力。" width="1000"></p>
<!-- meltype-flash-document-banner:end -->

試験用JSONをC#の元版`MeltypeSession`とRustの`Session::key_at`に与えます。
実アプリへの入力、IME登録、利用者の設定・辞書の読み取りは行いません。

`CoreProject`には移植基準コミット`45e14f86558aa6ea34f2c2ecd038ba95ca8c3c46`から取り出した
`src/Meltype.Core/Meltype.Core.csproj`の絶対パスを指定してください。省略時は作業ツリーのC#です。

```powershell
dotnet build tools/parity/direct/Direct.csproj -c Release -p:CoreProject=<original-core-project>
cargo +stable build -p meltype-core --example direct_trace --locked
python tools/check-direct-parity.py --csharp tools/parity/direct/bin/Release/net10.0/Direct.dll --rust <target>/debug/examples/direct_trace.exe --cases rust/core/tests/data/upstream-direct-time.json --output <new-directory>
```

入力形式は`rust/core/tests/data/upstream-direct-time.json`、またはその`cases`配列です。
各stepは`vk`、`ch`、`mods`、単調時計のミリ秒`time`を持ち、任意の`before`、`moved`、`direct`で
文脈変更・キャレット移動・モード変更を再現できます。`before`省略時は前の応答を反映した文書を使います。
固定試験は`cargo +stable test -p meltype-core --test upstream_direct_time --locked`で実行できます。

同梱スペル辞書・実際のScoreEngineを使用し、学習・ライブ変換・予測は無効です。
消費状態、確定編集、表示文字、英数モード、英語語の継続状態、文書を比較します。
候補・注釈・TSFの画面描画やOSへの実入力の互換性は示しません。速度測定にも使用しません。

TSFの英数判定停止は元版で固定700msです。旧フック方式の`InputSession.OnTimer`にある
`IdleFlushMs`・`MaxHoldMs`とは別の処理で、この比較では後者の移行完了を示しません。
