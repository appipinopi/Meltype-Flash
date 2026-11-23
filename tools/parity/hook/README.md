# 旧入力方式の状態・再入力・学習の比較

<!-- meltype-flash-document-banner:start -->
<p align="center"><img src="../../../assets/branding/readme/documentation.svg" alt="Meltype-FLASH — 稲妻のように、言葉の壁を駆け抜ける日本語入力。" width="1000"></p>
<!-- meltype-flash-document-banner:end -->

元版 `InputSession` / `PendingInput` / `UserModel` / `ScoreEngine` を実際に動かす比較ホストです。
基準は `45e14f86558aa6ea34f2c2ecd038ba95ca8c3c46`。TSFの英数入力中の700ms判定とは別の経路です。
キーの送信先はメモリー内のキューだけで、OSフックの登録・IME切替・実アプリへの再入力は行いません。

## 固定試験と再比較

`rust/core/tests/data/upstream-hook-session.json` に元版から採取した182ケースを保存しています。
消費状態、状態名、セッションID、判定・点数・判定の説明、再入力する全イベント、再入力依頼、
学習用の終了結果を全操作で比較します。判定への寄与は分類名と点数を比較し、寄与の説明文だけは比較しません。
16ケースは学習あり／なし、誤爆・見逃し、最大6文字、無効な見出し、回数上限、最長一致を含みます。

```powershell
cargo +stable test -p meltype-core --test upstream_hook_session --test prefix_learning --locked
cargo +stable build -p meltype-core --example hook_trace --locked
dotnet build tools/parity/hook/Hook.csproj -c Release "-p:CoreProject=<基準コミットから取り出したMeltype.Core.csprojの絶対パス>"
python tools/check-hook-parity.py --csharp tools/parity/hook/bin/Release/net10.0/Hook.dll --rust <target>/debug/examples/hook_trace.exe --cases rust/core/tests/data/upstream-hook-session.json --output <まだ存在しない出力フォルダー>
python rust/native/tests/hook_smoke.py <target>/debug/meltype_native.dll
```

`CoreProject` を省略すると作業ツリーのC#を参照します。元版との比較では基準コミットのCoreを指定してください。
ホストはJITで動き、元版の非公開フィールドから実際のセッションIDを読みます。IDを応答から推測していません。
速度の評価には使用しません。入力列だけの再生成は `python tools/parity/hook/cases.py <新しいJSONのパス>` です。

## RustとネイティブABI

Rustでは `input_session::InputSession::execute` に `Command` と現在の設定を渡します。
`&mut self` で遷移を直列化し、処理結果に再入力・終了結果を返します。処理中のファイル保存や外部コールバックはありません。
ネイティブABIでは既存のセッションハンドルと `meltype_control` を使用します。

```json
{"op":"hook_session","command":{"op":"key","vk":75,"scan":37,"time":10000,"isUp":false,"extended":false,"injected":false,"modifierDown":false,"allowed":true}}
```

コマンドは `key`、`timer`、`context`、`take`、`abort`、`snapshot` です。
時刻はホストの単調時計のミリ秒。`key` の `allowed` / `reason` はホスト側の保留可否の結果です。
対象アプリ・秘密欄・フォーカス・入力デバイス等の適格性判定を行うのはOS側の呼び出し元です。
`take` はバッチを返した後もFlushingを維持し、次の空の取り出しでCommittedまたはIdleへ進みます。
再入力中に届いたキーも同じキューに積みます。押下／解放、スキャンコード、拡張・注入フラグを保持します。
ワーカーは任意の `sessionId` を `take` に指定でき、古い依頼が新しいセッションのキーを取り出すのを防ぎます。
停止時は `abort` の戻り値を処理してからハンドルを破棄してください。CoreはキーをOSへ自動送信しません。

終了結果を保存するワーカーは、別の管理操作として以下を呼びます。

```json
{"op":"prefix_learn","letters":"konn","outcome":"JapaneseAccepted","decidedEnglish":false}
```

結果は `JapaneseAccepted` / `JapaneseRejected` / `StayedAccepted` / `StayedRejected` の4種類。
学習無効または秘密モードのセッションでは保存しません。新しい統計はファイルロック下で `state.json` に保存します。
旧 `model.json` の有効な3〜6文字の見出しを取り込み、破損時は元のバイト列を `.broken` に残して他のデータを保持します。
取り込み済みの空のモデルは旧ファイルから再取り込みしないため、全削除後に古い学習が復活しません。
既存の学習一覧API、`prefix:<見出し>` の個別削除、日時指定削除、全削除、バックアップ・復元にも統合しています。
設定画面の判定テストとTSFの英数判定にもこの統計を渡します。保存後はセッションのモデルを再読み込みします。

## 残る範囲

CoreとABIの比較であり、Windowsの低レベルフック・再入力ワーカー・フォーカス先への配送を実機で確認したものではありません。
既存TSFの時間判定と混同せず、Windowsの連携・UI・インストール更新の完了条件は別途確認します。
全独自ローマ字表・ユーザー辞書・他プロセスからの変更通知との組合せも網羅できていません。
