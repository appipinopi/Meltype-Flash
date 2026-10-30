# Windows版の管理・検証記録

現在の導入・操作は [Windows版README](WINDOWS-PACKAGE.md)、現在の公開ファイルは [配布案内](DOWNLOADS.md) を参照してください。
以下は以前の開発用READMEから移した記録です。「リリースがない」などの記述は当時の状況であり、現在の公開状況を説明するものではありません。導入コマンドと登録IDの説明は、現在のFlash専用登録に合わせています。

# Meltype-flash Windows版・配布前確認用

Windows 10/11 x64向けの開発用プレビューです。x86のTIPとworkerは32bitアプリ用です。登録・更新・登録解除用の `Manage-Windows.ps1` を同梱します。管理者権限が必要です。クリーン環境での導入・更新・削除の実動確認は未完了です。

## 内容

- `meltype-flash.exe`：設定画面。`--tray` で管理トレイを起動。
- `meltype-rs.exe`：共通CoreのCLI。
- `tip/x64` と `tip/x86`：各アーキテクチャの `MeltypeTip.dll` と `meltype-tip-worker.exe`。workerはDLLの隣に必要です。
- 各実行フォルダーのVisual C++ランタイムDLL：使用したVisual Studioの再配布可能CRTから同梱。
- `LICENSE`、`THIRD-PARTY-NOTICES.md`：本体と依存ライブラリのライセンス情報。
- `package-audit.json`、`SHA256SUMS.json`：静的依存関係・ファイル検査の結果とハッシュ。

システム辞書のFSTと値データは実行ファイルに組み込まれています。別のシステム辞書TXT/FSTや `sqlite3.dll` の配置は不要です。利用者の設定とユーザー辞書はRust版のデータフォルダーに作成され、このZIPには個人データを含めません。元版とは別の保存先です。

既定のデータに触れずに設定画面を確認する場合は、PowerShellで一時的な試験用フォルダーを指定します。

```powershell
$env:MELTYPE_FLASH_DATA_DIR = Join-Path $env:TEMP 'Meltype-flash-preview-check'
./meltype-flash.exe
```

## 制限

System変換にはWindowsの日本語言語コンポーネントが必要です。任意のMozcヘルパー・Mozc-UT・Zenzaiサーバーやモデルは同梱していません。Mozcが無い場合のSystemへのフォールバックは、実Mozcの動作確認とは別です。

このZIPを展開しただけではIMEは登録されません。Meltype-flashは本家と異なるCOM／入力プロファイルIDとインストール先を使用します。旧Rust版が共有していた本家の登録IDは変更しません。元版の `Register-Tip.ps1` ではなく、同梱の管理スクリプトを使用してください。

検査の成功は、必要ファイル・PEのx64/x86・TSF公開関数・静的DLL依存・ZIP内ハッシュの確認です。クリーンなWindowsへの導入、OpenGL描画環境、動的ロードされる全コンポーネント、実アプリ入力・更新・削除の受入成功を意味しません。

## 再作成・再点検

リポジトリで `python rust/package-windows.py --help` を参照し、ビルド済みの設定/CLI/TIP/workerとVisual Studioの `VC/Redist/MSVC/<version>` を指定してください。既存フォルダーとZIPは上書きしません。

```powershell
python tools/check-rust-windows-package.py <展開フォルダー> --report <検査結果.json>
```

## 初回導入・更新・登録解除

設定画面には初回案内と「報告・更新」があり、トレイからも報告・配布一覧を開けます。
初回案内の完了は設定へ保存されます。ブラウザーに入力内容や学習データを自動送信しません。
更新案内はAutoUpdateがONなら起動時にバックグラウンドでRustフォークの公開配布を確認します。設定画面からも再確認できます。元C#版のZIPをRust版として案内しません。公開された安定版のRust Windows ZIPを取得し、GitHubのSHA-256ダイジェスト・ZIP内の全ファイルを検査してから更新します。更新の適用にはUAC確認が必要です。

まず通常のPowerShellで検査し、導入時だけ管理者PowerShellを使用します。

```powershell
.\Manage-Windows.ps1 -Action Inspect
.\Manage-Windows.ps1 -Action Install -WhatIf
.\Manage-Windows.ps1 -Action Install
.\Manage-Windows.ps1 -Action Update -Package 'C:\path\to\new-package'
.\Manage-Windows.ps1 -Action Uninstall
```

SHA256SUMS.jsonの全ファイルを検査し、Program Files/Meltype-flash/versions/<配布ハッシュ>へ配置します。
両アーキテクチャのDLLとworker・CRTをまとめて配置し、登録失敗時は以前のDLLを再登録します。
ハッシュは破損検知用です。配布元の認証や署名の代わりではありません。
Flashは元版と異なるCOM／入力プロファイルIDを使うため、元版の登録を置き換える `-ReplaceExisting` は不要です。
古いバージョンは保持するため、稼働中DLLの上書きを避けます。
更新前に現在ユーザーの同じデータフォルダーのトレイへ終了要求を送り、終了を待ちます。未保存の設定があれば処理を止めるため、保存または取り消して再実行してください。更新時は自動起動がONなら新しい実行ファイルへ付け替え、OFFなら維持します。更新後は新フォルダーからトレイを起動してください。

Uninstallはトレイの終了を待ち、実行ユーザーのRust用タスクとRun項目、IME登録を順に解除します。版別フォルダーの配布一覧とハッシュが一致するファイルを削除します。一覧にないファイルや利用者データは保持し、変更された配布ファイルが見つかった場合は削除を止めます。
ロック中のDLLは残し、削除を再開するための配布一覧も保持します。アプリ終了・Windows再起動後に次のコマンドで残りを整理できます。空でないフォルダーをまとめて削除する処理は行いません。

```powershell
.\Manage-Windows.ps1 -Action Cleanup
```

登録解除済みの状態を示すinstalled.jsonは残ります。元ZIP側の管理スクリプトを使ってCleanupしてください。再起動時の自動削除予約は未実装です。実OSでの登録・ロールバックの受入確認は残ります。

## ポータブル起動

```powershell
.\Start-Portable.ps1
# 管理トレイも起動する場合
.\Start-Portable.ps1 -Tray
```

設定画面と入力テストは、ZIP内のdataフォルダーを保存先として使えます。
環境変数は子プロセスだけへ引継ぎ、スクリプト終了時に元へ戻します。
通常起動は登録不要のバッファ入力とトレイです。新規設定では半角/全角またはAlt+Backquoteで入力をON/OFFにします。設定画面で複数登録でき、以前のCtrl+Alt+Jなどの割り当ても移行します。OS側のIMEは英数入力にし、元版Meltypeとの同時入力は避けてください。設定/Coreの確認だけなら `-SettingsOnly` を指定します。

編集可能なUI Automation Valueパターンを持つ通常入力欄に対応し、未確定文字と候補を別ウィンドウに表示して、確定時だけUnicode入力を送ります。パスワード・読取専用・判別できない入力欄では入力を取り込みません。フォーカス変更、クリック、Ctrl/Alt/Winショートカットで未確定入力を取り消します。OS辞書、ライブ変換、周辺文に基づく確定後訂正、学習の永続保存、管理者アプリへの入力は対象外です。設定変更はトレイの巡回で反映します。このホストはTSFと同等の機能を保証しません。
別アプリ内のTSF workerはこの起動スクリプトの子ではないため、ポータブルdataの共有も未対応です。

## 起動設定の引継ぎと回復

管理スクリプトは設定EXEの専用コマンドを使い、画面・トレイ・学習データを開かずに現在ユーザーの起動設定を処理します。
タスクが使用できない場合は既存のRun項目と同じフォールバックを使用します。元版Meltypeなど別名のタスクは変更しません。

```powershell
.\meltype-flash.exe --startup-action query --startup-result .\startup-state.json
.\meltype-flash.exe --startup-action rebind --startup-result .\startup-state.json
.\meltype-flash.exe --startup-action disable --startup-result .\startup-state.json
```

queryは参照のみ、rebindは有効な起動設定だけをこのEXEへ付け替え、disableはRust版が所有する現在ユーザーの起動項目を解除します。stop-trayは同じ保存先のトレイに通常終了を要求します。Enabled設定は変更せず、未保存の設定があるトレイを強制終了しません。
結果はprotocol=1のJSONです。コマンドの失敗は非ゼロで終了し、管理スクリプトもエラーとして扱います。
起動設定の更新が失敗した場合、IMEの更新と状態ファイルは完了している場合があります。その旨をエラーで表示し、旧バージョンのファイルを残します。
新フォルダーからrebindを再実行できます。

別の管理者アカウントで実行した場合、この処理の対象もその管理者です。通常使うユーザーの処理には、そのユーザーで上記コマンドを実行してください。
`-SkipCurrentUserStartup` は、手動でトレイ終了・起動設定を処理する場合に限って両方の自動処理を省略します。
起動処理用プロトコルを持たない旧プレビューの削除では、先に新版へ更新するか、手動で起動設定を解除してこの指定を使用します。

## 自動更新と実登録の確認（2026-10-21）

```powershell
.\Update-Windows.ps1 -Action Check
.\Update-Windows.ps1 -Action Stage   # 取得・検査まで。IME登録は変更しない
.\Update-Windows.ps1 -Action Apply   # 導入済みRust版の更新。UAC確認あり
.\Test-InstalledIme.ps1 -Package . -ReportDirectory C:\Temp\Meltype-registration-report
# 実登録試験（未使用の結果フォルダーを指定、元版の両ABI DLLが必要）
.\Test-InstalledIme.ps1 -Package . -ReportDirectory C:\Temp\Meltype-registration-exercise -ExerciseRegistration
```

Applyは既存の管理導入が必要です。通常ユーザーからの適用では、更新後に同じユーザーで新トレイを起動します。適用を開始した表示は完了通知ではありません。起動した側の一時ログと `Program Files/Meltype-flash/update-logs` の記録で結果を確認できます。別管理者の認証で昇格した場合の自動起動設定については下記の制限に従ってください。Stageの取得物はWindowsのTemp内の `Meltype-flash-update-*` に残します。署名・SmartScreenの配布者評価は別途必要です。

実登録試験は元DLLのパス・ハッシュとCOM/TIPレジストリを保存し、管理配布のInstall、両ABIの実COM起動とTSFプロファイルを検査して、finallyで元版の登録を戻します。既存のRust管理導入が有効な場合や版別フォルダーにファイルがある場合は上書き試験しません。登録解除済みで配布ファイルが残っていない試験用の管理記録だけなら再試験できます。既存アプリを強制終了したり既定のIMEを切り替えたりしません。外部アプリの実TSF入力はこの登録試験に含みません。

現時点でフォークに公開リリースがないため、実公開ZIPからの更新受入は未確認です。登録なし入力は専用Win32 Editを使う試験で日本語確定・同一ウィンドウ内のフォーカス変更・パスワード/読取専用の除外・終了時のフック解放を確認しました。試験用ビルドだけが、試験プロセス宛ての識別付き合成キーを許可します。通常版は合成キーを除外します。実キーボード、各エディター、候補の原版同等レイアウトは別途受入が必要です。

登録なし入力の自動試験は対話デスクトップで `cargo test -p meltype-settings --locked --no-run` を実行し、表示されたテストEXEを `python tools/test-portable-input.py <テストEXE> --unit-host` へ渡します。製品EXEは識別付き試験キーも除外するため、このドライバーの対象にしません。実製品の起動・終了は `rust/Test-TrayStop.ps1 -Binary <製品EXE> -Portable` で別途検査します。

2026-10-21、元版を停止後に管理者権限で実登録試験を実施しました。x64/x86の実登録先、TSF言語プロファイル、登録経由のITfTextInputProcessor COM起動が通過し、登録解除と元版への復元も確認しました。検査コードのIIDをWindows SDKの `aa80e7f7-2021-11d2-93e0-0060b067b86e` へ修正後の結果です。最終結果は registeredComAndProfilePassed=true / originalRestored=true / externalApplicationInputTested=false。外部アプリの実TSF入力の成功を意味しません。

## トレイの入力方式と予測表示（2026-10-21修正）

トレイの「Meltype IME」はRustのMode=Tsfを、「Meltype キーボード」はMode=Keyboardを選びます。前者は登録やOSの入力方式切替を実行しません。公式C#版が登録されている環境では、Win+SpaceのMeltypeはそのC#版のままです。明示的な`Start-Portable.ps1`／`--portable-keyboard`起動では入力ONで始まり、設定画面に登録した切替キーで切り替えます。新規設定の既定は半角/全角とAlt+Backquoteです。トレイ上部に実際のON/OFFを表示します。無効化・IMEモードへの変更でポータブル入力を止めます。

ポータブル候補欄はCoreの予測を表示し、Tabで選択、Enterで確定します。読みの続きに一致する学習文章がRust側のデータに必要で、C#版の個人データを自動では取り込みません。表示位置は取得できるWin32キャレットを優先し、取得できない場合はUIA入力欄の下になります。元版の外観・マウス操作との完全一致、およびChrome/ChatGPT/Discord実入力の確認は未完了です。[現在の検査状況](https://github.com/appipinopi/Meltype/blob/rust/rust/MIGRATION.md)も参照してください。

Chromium/Electronの空のcontenteditableには、UIAのValue/TextPatternに加えて元版と同じIAccessible2のEDITABLE状態を照会します。実際のChatGPT・Discordでの物理キー入力の確認は別途必要です。

## 直接入力IMEとポータブル入力の違い

「Meltype IME (入力欄に直接入力)」にはRust TIPのWindowsへの登録とWin+Spaceによる選択が必要です。トレイの選択だけでは登録されません。Meltype-flashはC#版と異なるCOM登録IDを使うため、元版の登録先を変更せず、`Manage-Windows.ps1 -Action Install`を管理者として実行します。開いているアプリを再起動してください。

直接入力用の設定・学習は通常`%LOCALAPPDATA%/Meltype-flash`です。`Start-Portable.ps1`はパッケージ内`data`を使うため、直接入力のトレイは`MELTYPE_FLASH_DATA_DIR`（旧 `MELTYPE_RUST_DATA_DIR` も明示指定時のみ対応）を指定せず`meltype-flash.exe --tray`で起動してください。Rust設定のMode=Tsf/Enabled=trueと、Windows側でMeltypeが選択されていることの両方が必要です。C#プロセスとポータブル入力の同時動作は避けてください。
