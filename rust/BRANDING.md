# Meltype-FLASH

このフォークのRust版の製品名は **Meltype-FLASH**。参照元のC#／WinForms版は **Meltype** と呼び分ける。
外観・入力機能の再現作業は、Meltype-FLASH向けに継続する。

## Windows の分離

| 対象 | Meltype-FLASH |
|---|---|
| 設定・トレイ実行ファイル | `meltype-flash.exe` |
| 設定画面・IME・トレイ表示 | Meltype-FLASH |
| インストール先 | `%ProgramFiles%/Meltype-flash` |
| データ既定値 | `%LOCALAPPDATA%/Meltype-flash` |
| 起動タスク／Run項目 | `Meltype-flash-<ユーザー識別値>` |
| COMサービスID | `{C540BAF6-88B1-5B30-B524-2C8702B034FC}` |
| 日本語プロファイルID | `{6D01DE9C-8AB3-5CFC-A5C6-10893CDF8762}` |
| 更新対象 | `Meltype-flash-Windows*.zip` のみ |

描画属性GUID・候補ウィンドウ名・ログ保存先も分離する。
既定値では本家や旧Rust版の設定を読み込まず、移動もしない。
`MELTYPE_FLASH_DATA_DIR` は明示的な保存先指定。旧 `MELTYPE_RUST_DATA_DIR` も互換用に受け付けるが、新変数を優先する。

## 旧Rust版からの移行

旧Rust版は本家と同じCOM登録IDを使用していたため、C#のトレイとRustのIMEが同時に動く場合がある。
Meltype-FLASHのインストーラーは旧登録の解除や本家DLLへの復元を自動実行しない。
この改名の作業では実機の登録・既存プロセス・学習データを変更していない。
本家の復元とFlashの導入は、配布物の検査後に別の導入作業として行う。
新しいパッケージ検査は、Flash専用IDを持たない古いDLLの同梱を拒否する。

## 互換性と範囲

Cargoパッケージ名（`meltype-settings`など）、C ABI、内部ワーカー名とDLLファイル名は互換性のため維持。
Windows用DLLのリソースは `FlashTip.rc` を使用し、本家C++ビルドのリソースを変更しない。
Linux IBusは `meltype-flash` というエンジン名と別の登録ファイル・保存先を使用する。
macOS・Android・ChromeOSの配布名もFlashに揃えるが、既存のOSホスト側登録IDの完全分離・各実機での共存確認は未完了。
Windowsの実登録・本家との同時入力も、この変更だけで受入確認済みとはしない。

## 今回の検証

- Rustの設定アプリ26試験・native46試験が成功。対話用1試験は未実行。
- 配布検査8試験・リリース収集3試験が成功。
- 専用ID・Flash製品リソースを持つx64 DLLでCOM検査とTSFサーバー10試験が成功。
- 管理・更新・ライフサイクル・起動設定・トレイ終了を、一時ファイルまたは私用ウィンドウで検査。
- Flash設定画面の撮影結果は `tools/parity/ui/evidence/meltype-flash-branding/`。
- x86の新しい製品DLL、実機登録、各OSのインストール済み環境での共存は未確認。
