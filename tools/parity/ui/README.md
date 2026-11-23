# 画面の検証範囲

<!-- meltype-flash-document-banner:start -->
<p align="center"><img src="../../../assets/branding/readme/documentation.svg" alt="Meltype-FLASH — 稲妻のように、言葉の壁を駆け抜ける日本語入力。" width="1000"></p>
<!-- meltype-flash-document-banner:end -->

`capture_settings.py <visual-test付き実行ファイル> <出力フォルダー>` は利用中のデータから隔離した6プロファイルで、設定・辞書・定型文・学習・バックアップ・入力テストを描画します。設定の保存やSQLiteへの移行を行わなかったことも確認します。Linuxでは `xvfb-run -a` が必要です。

Windowsの記録は [evidence/windows-fst-sqlite](evidence/windows-fst-sqlite) にあります。これは元版と全画面が一致したという証拠ではありません。元版 `src/Meltype/UI/UserDictionaryForm.cs` は560×520、`SnippetsForm.cs` は620×580の独立ウィンドウですが、Rustは740×740の共通ウィンドウ内のページです。設定項目名・説明・列挙名の一致、画面レイアウトの一致、実ファイル選択/キーボード操作は別々に確認します。

Windows候補は `tsf_candidate_smoke.py` が元のC++描画器との寸法・ピクセル・操作を、`tsf_candidate_view_smoke.py` が実COM ABIで文節/候補の投影と位置を検査します。全設定や全OS上の全画面の一致をこれらだけで完了にしません。

## 小さな設定表示差の修正（2026-10-20）

[6ページと長い説明の描画記録](evidence/windows-small-fixes-20261020/report.json)を追加。判定テストの見出しとアプリ別設定の「有効」/プロセス名の例を1.3.1-pre.1に合わせ、下部を説明→判定テスト→操作ボタンの順にした。説明は4行分の高さに固定し全文をツールチップで表示、最後の設定行の下に16ポイントの余白を置いた。管理ページの切替を上部へ移し、200ポイントのプロファイル選択欄とボタンを横に並べられる幅を確保した。

設定・辞書・定型文・学習・バックアップ・入力テストの6ページを隔離データで採取し、設定ページと長いAppRules説明を目視した。採取時にconfigやSQLiteを保存・移行していない。既存の設定操作12テストとClippyも成功。説明を指定する試験はvisual-testビルドで `capture_settings.py <実行ファイル> <出力先> --pages settings --help-key AppRules`。通常ビルドはこの試験用環境変数を処理しない。

既存の60項目のメタデータ基準は45e14f8を維持する。最新版で増えた設定・動作、全画面の寸法一致、ファイル選択や全OSの実操作は [版別の未対応一覧](../../../rust/UPSTREAM-1.3.1-pre1-GAPS.md)を参照。

## Meltype-FLASH の独立ウィンドウ（v0.1.1向け）

`check_dialog_windows.py <visual-test付きmeltype-flash.exe> <結果.json>` はWindowsで私用の設定フォルダーを使い、ログ・報告・誤判定の報告・About・更新の5ウィンドウを検査する。
タイトルとクライアント寸法を取得し、各子ウィンドウだけにWM_CLOSEを送っても設定ウィンドウが残ること、設定と辞書を変更しないことを確認する。通常のユーザーアプリには操作しない。
描画本文の検査は `capture_settings.py ... --pages settings --lifecycle report`（`corrections` / `log` / `about` / `updates` も可）。撮影時は本文をルートに埋め込むため、OSのタイトルバーの画素一致の証拠にはしない。
`corrections` の撮影では「hashi → 橋 → 箸」の合成データを使用する。
最終結果は [evidence/flash-dialogs-v011](evidence/flash-dialogs-v011)。
