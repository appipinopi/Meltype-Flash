<!-- meltype-flash-platform:start -->
<p align="center"><img src="../assets/branding/readme/logo.svg" alt="Meltype-flash" width="520"></p>
<a id="meltype-for-windows"></a>
<p align="center"><img src="../assets/branding/readme/windows/hero.svg" alt="Meltype-flash for Windows" width="1000"></p>
<p align="center"><b>稲妻のように、言葉の壁を駆け抜ける日本語入力。</b></p>
<p align="center"><a href="https://github.com/appipinopi/Meltype-Flash/releases/tag/flash-v0.1.0-preview.7"><img src="https://img.shields.io/badge/Flash-v0.1.0_Pre7-ffe34a" alt="Flash v0.1.0 Pre7"></a> <img src="https://img.shields.io/badge/status-開発プレビュー-666973" alt="開発プレビュー"></p>
<p align="center"><a href="#インストール">ダウンロード・導入</a> · <a href="#使い方">使い方</a> · <a href="#困ったとき">困ったとき</a> · <a href="https://github.com/appipinopi/Meltype-Flash/issues/new/choose">不具合の報告・提案</a></p>
<p align="center">
  <a href="WINDOWS-PACKAGE.md"><img src="../assets/branding/readme/cards/os-windows-active.svg" alt="Windows" width="32%"></a>
  <a href="../linux/README.md"><img src="../assets/branding/readme/cards/os-linux.svg" alt="Linux" width="32%"></a>
  <a href="../mac/README.md"><img src="../assets/branding/readme/cards/os-mac.svg" alt="Mac" width="32%"></a>
</p>
<p align="center">
  <a href="../android/README.md"><img src="../assets/branding/readme/cards/os-android.svg" alt="Android" width="32%"></a>
  <a href="../chromeos/README.md"><img src="../assets/branding/readme/cards/os-chromeos.svg" alt="ChromeOS" width="32%"></a>
  <a href="../ios/README.md"><img src="../assets/branding/readme/cards/os-ios.svg" alt="iOS" width="32%"></a>
</p>
<p align="center"><a href="../README.md">トップのREADME</a></p>
<!-- meltype-flash-platform:end -->

Windowsの入力欄に直接入力するRust版IMEと、登録不要のポータブル入力を用意しています。
**開発プレビューです。Windowsへの登録と、各アプリでの実入力の検証は別に扱っています。**

<a id="内容"></a>
<h2><img src="../assets/branding/readme/windows/headings/df8764ebcf5d.svg" alt="内容" width="1000"></h2>

- `meltype-flash.exe`：設定画面・管理トレイ。
- `MeltypeTip.dll` と `meltype-tip-worker.exe`：Windows TSF用IME。64bit用と32bitアプリ用を同梱します。
- `meltype-rs.exe`：共通Coreを確認するCLI。
- 管理・更新・ポータブル起動のスクリプト、辞書・ライセンス・チェックサム。

TSF向けZenzai接続は既定オフの試験実装です。使う場合は自分でモデルとllama-serverを用意します。配布物には同梱しません。実モデルの精度・速度と通常アプリでの入力確認は未実施です。[設定と制限](UPGRADE-1.4.0.md#windowsのzenzai)を参照してください。

[動く環境](#動く環境) · [インストール](#インストール) · [使い方](#使い方) · [困ったとき](#困ったとき) · [アンインストール](#アンインストール) · [開発者向け](#開発者向け)

<a id="動く環境"></a>
<h2><img src="../assets/branding/readme/windows/headings/f378dd1f4672.svg" alt="動く環境" width="1000"></h2>

- Windows 10 / 11 x64。32bitアプリ用のTIP / workerも同梱します。
- IMEを登録する場合は管理者権限。
- 設定画面にはOpenGL 2.0以上が必要です。

Windows ARM64向けのネイティブ配布を案内するものではありません。
本家C#版とは専用の登録ID・実行ファイル名・データ保存先を使います。[共存方針](BRANDING.md) を参照してください。

<a id="インストール"></a>
<h2><img src="../assets/branding/readme/windows/headings/4f36b3b314d8.svg" alt="インストール" width="1000"></h2>

<a id="セットアップ版"></a>
<h3><img src="../assets/branding/readme/windows/headings/f361bd6b29a2.svg" alt="セットアップ版" width="1000"></h3>

Pre7ではセットアップEXEは添付していません。次のZIPからの導入手順を使ってください。

警告やセキュリティ製品の検出が出た場合は、配布元・ファイル名・チェックサムを確認し、保護機能の無効化を前提に進めないでください。

<a id="zipから登録する"></a>
<h3><img src="../assets/branding/readme/windows/headings/b7962eec4c15.svg" alt="ZIPから登録する" width="1000"></h3>

[Pre7](https://github.com/appipinopi/Meltype-Flash/releases/tag/flash-v0.1.0-preview.7)の`Meltype-flash-Windows-x64.zip`を全体展開します。
管理者PowerShellで、そのフォルダーから実行します。

```powershell
.\Manage-Windows.ps1 -Action Inspect
.\Manage-Windows.ps1 -Action Install
```

既存導入の更新は新しい配布フォルダーから `-Action Update`。使用中のDLLと旧バージョンのファイルは保持します。
本家のCOM登録を置き換えるための `-ReplaceExisting` を通常のFlash導入で指定する必要はありません。

<a id="登録不要で試す"></a>
<h3><img src="../assets/branding/readme/windows/headings/c0cbccf1a1d5.svg" alt="登録不要で試す" width="1000"></h3>

同じ`Meltype-flash-Windows-x64.zip`を全体展開し、PowerShellから次を使います。

```powershell
.\Start-Portable.ps1
# 設定画面だけを確認する場合
.\Start-Portable.ps1 -SettingsOnly
```

登録不要のバッファ入力です。OS側のIMEは英数入力へ切り替え、ほかの日本語入力と同時に処理しない状態で試してください。

<a id="使い方"></a>
<h2><img src="../assets/branding/readme/windows/headings/504586dee137.svg" alt="使い方" width="1000"></h2>

<a id="登録したime"></a>
<h3><img src="../assets/branding/readme/windows/headings/1d7ca330ed92.svg" alt="登録したIME" width="1000"></h3>

Win + Spaceで入力方法を選んでからテキスト欄へ入力します。設定・トレイの起動だけでは、OSの入力方法は切り替わりません。

| キー | 操作 |
| --- | --- |
| Space | 変換・候補切替 |
| Enter | 確定 |
| Esc | 取消 |
| ← / → | 変換中の文節選択 |
| F6〜F10 | ひらがな・カタカナ・英数などの字種切替 |

設定アプリの入力テストでは、通常の外部アプリへのIME登録と分けてCoreの動作を確認できます。

<a id="ポータブル入力"></a>
<h3><img src="../assets/branding/readme/windows/headings/c5082295d280.svg" alt="ポータブル入力" width="1000"></h3>

Ctrl + Alt + Jで入力をON / OFFにします。未確定文字と候補は別ウィンドウに表示し、確定時に文字を送ります。
通常のUI Automation対応入力欄が対象で、パスワード・読取専用・判別できない入力欄、管理者アプリへの入力は対象外です。
フォーカス移動やショートカットで未確定入力を取り消します。
OS辞書・ライブ変換・確定後訂正・学習の永続保存など、登録したIMEと同じ機能は保証しません。

<a id="プライバシー"></a>
<h2><img src="../assets/branding/readme/windows/headings/3f227be2ff9a.svg" alt="プライバシー" width="1000"></h2>

登録したIMEの既定データは `%LOCALAPPDATA%\Meltype-flash`、ポータブル起動は展開先の `data` を使います。
通常の入力と変換は端末内で処理します。更新確認・更新ファイルの取得や、自分で開く不具合報告は通信を伴います。
本家の個人データを自動で取り込む前提にはしていません。

<a id="困ったとき"></a>
<h2><img src="../assets/branding/readme/windows/headings/ecff8916cc2a.svg" alt="困ったとき" width="1000"></h2>

<details>
<summary>設定画面は開くのに入力できない</summary>

IME登録済みか、Win + SpaceでFlashを選んだかを確認します。トレイの「Meltype IME」を選ぶ操作だけでは、Windowsへの登録は実行しません。
設定を変えた後はアプリを開き直し、ポータブル入力や本家プロセスと重なっていないかも確認してください。

</details>

<details>
<summary>アプリによって入力できない・候補が違う</summary>

TSF版とポータブル入力では対応する入力欄が異なります。どちらを使ったか、OS・アプリ名・入力例・期待する結果を [Issues](https://github.com/appipinopi/Meltype-Flash/issues/new/choose) に記載してください。
登録検査の成功は、すべての外部アプリでの入力成功を意味しません。

</details>

<details>
<summary>更新したい</summary>

セットアップ版は新しいセットアップの案内に従います。管理ZIP版は新しいZIPから `Manage-Windows.ps1 -Action Update` を実行します。
更新スクリプトがある管理導入では `Update-Windows.ps1 -Action Check` で確認し、Stage / Applyは [管理・検証記録](WINDOWS-DEVELOPMENT.md) の対象条件を確認して使ってください。
ポータブル版は必要な `data` を保存してから、新しい配布を別フォルダーへ展開します。

</details>

<a id="アンインストール"></a>
<h2><img src="../assets/branding/readme/windows/headings/0f0d9bc7afc2.svg" alt="アンインストール" width="1000"></h2>

先にWin + Spaceで別の入力方法へ切り替え、Flashのトレイと設定画面を終了します。
セットアップ版はWindowsの「インストールされているアプリ」から削除します。
管理ZIP版は管理者PowerShellで導入に使った管理一式から実行します。

```powershell
.\Manage-Windows.ps1 -Action Uninstall
```

使用中のDLLなど、削除できない旧ファイルが残る場合は、関連アプリを閉じてから管理スクリプトのCleanupを使います。
ポータブル版は終了後、展開フォルダーを削除します。個人データの扱いは導入形式で異なるため、必要な設定・辞書を先に保存してください。

<a id="開発者向け"></a>
<h2><img src="../assets/branding/readme/windows/headings/f4832b647e65.svg" alt="開発者向け" width="1000"></h2>

Rust 1.88以上とMSVC Build Toolsが必要です。リポジトリのルートから実行します。

```powershell
cargo test -p meltype-core --locked
cargo run -p meltype-settings
.\rust\build-windows.ps1 -WithTip
```

ソースから起動する設定アプリは `meltype-flash.exe` です。登録・更新・起動設定・COM検証の詳細は [Windows管理・検証記録](WINDOWS-DEVELOPMENT.md)、実装差分は [移行記録](MIGRATION.md) を参照してください。

<a id="ライセンス"></a>
<h2><img src="../assets/branding/readme/windows/headings/ce4422debab0.svg" alt="ライセンス" width="1000"></h2>

[GPL-3.0-or-later](../LICENSE)。ランタイムと辞書などの表示は [THIRD-PARTY-NOTICES.md](../THIRD-PARTY-NOTICES.md) を参照してください。
