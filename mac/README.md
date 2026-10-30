<!-- meltype-flash-platform:start -->
<p align="center"><img src="../assets/branding/readme/logo.svg" alt="Meltype-flash" width="520"></p>
<a id="meltype-for-mac"></a>
<p align="center"><img src="../assets/branding/readme/mac/hero.svg" alt="Meltype-flash for Mac" width="1000"></p>
<p align="center"><b>稲妻のように、言葉の壁を駆け抜ける日本語入力。</b></p>
<p align="center"><a href="https://github.com/appipinopi/Meltype-Flash/releases/tag/flash-v0.1.0-preview.7"><img src="https://img.shields.io/badge/Flash-v0.1.0_Pre7-ffe34a" alt="Flash v0.1.0 Pre7"></a> <img src="https://img.shields.io/badge/status-開発プレビュー-666973" alt="開発プレビュー"></p>
<p align="center"><a href="#インストール">ダウンロード・導入</a> · <a href="#使い方">使い方</a> · <a href="#困ったとき">困ったとき</a> · <a href="https://github.com/appipinopi/Meltype-Flash/issues/new/choose">不具合の報告・提案</a></p>
<p align="center">
  <a href="../rust/WINDOWS-PACKAGE.md"><img src="../assets/branding/readme/cards/os-windows.svg" alt="Windows" width="32%"></a>
  <a href="../linux/README.md"><img src="../assets/branding/readme/cards/os-linux.svg" alt="Linux" width="32%"></a>
  <a href="README.md"><img src="../assets/branding/readme/cards/os-mac-active.svg" alt="Mac" width="32%"></a>
</p>
<p align="center">
  <a href="../android/README.md"><img src="../assets/branding/readme/cards/os-android.svg" alt="Android" width="32%"></a>
  <a href="../chromeos/README.md"><img src="../assets/branding/readme/cards/os-chromeos.svg" alt="ChromeOS" width="32%"></a>
  <a href="../ios/README.md"><img src="../assets/branding/readme/cards/os-ios.svg" alt="iOS" width="32%"></a>
</p>
<p align="center"><a href="../README.md">トップのREADME</a></p>
<!-- meltype-flash-platform:end -->

macOSの入力ソースとして動く、Rust版Meltype-flashです。共通CoreにSwiftのInput Method KitとazooKeyの変換エンジンを接続しています。
**開発プレビューです。Developer ID署名・Apple公証、物理端末や各アプリでの入力確認は未完了です。**

<a id="内容"></a>
<h2><img src="../assets/branding/readme/mac/headings/df8764ebcf5d.svg" alt="内容" width="1000"></h2>

ローマ字入力、英語・日本語の判定、変換候補、確定、文節操作をMacの入力方法へ接続しています。
画面・メニュー・辞書・入力モードの移植状況は [移行記録](../rust/MIGRATION.md) を参照してください。

[動く環境](#動く環境) · [インストール](#インストール) · [使い方](#使い方) · [困ったとき](#困ったとき) · [アンインストール](#アンインストール) · [開発者向け](#開発者向け)

<a id="動く環境"></a>
<h2><img src="../assets/branding/readme/mac/headings/f378dd1f4672.svg" alt="動く環境" width="1000"></h2>

- macOS 13以降。
- Apple Siliconには `Meltype-flash-macOS-arm64.zip`、Intelには `Meltype-flash-macOS-x64.zip`。
- アドホック署名のプレビューを自分で導入できる環境。

本家のHomebrew caskは本家版です。このFlash版のインストールには下記の配布ファイルを使ってください。

<a id="インストール"></a>
<h2><img src="../assets/branding/readme/mac/headings/4f36b3b314d8.svg" alt="インストール" width="1000"></h2>

1. [FlashのPre7](https://github.com/appipinopi/Meltype-Flash/releases/tag/flash-v0.1.0-preview.7) から、CPUに合うmacOS ZIPを取得します。
2. **ZIP全体を展開**します。`Meltype.app` だけでなく、インストール用のcommand・shellスクリプトも同じフォルダーに必要です。
3. `Install Meltype.command` を実行します。macOSが許可を求める場合は、システム設定の「プライバシーとセキュリティ」で取得元を確認して操作してください。
4. インストーラーの終了後、入力メニューでMeltypeを選びます。初回に出てこなければログアウト・ログインして確認します。

ターミナルから導入する場合は、展開したフォルダー内で `bash install.sh` を実行します。
配布内のバンドル名・入力メニュー名には互換用の `Meltype.app` / Meltypeが残っています。通常のアプリのようにダブルクリックして使うものではありません。
本家と一部のOSホスト側IDが残るため、同時インストール・同時動作は未確認です。先に標準の入力ソースへ切り替えてから導入してください。

<a id="使い方"></a>
<h2><img src="../assets/branding/readme/mac/headings/936b17951891.svg" alt="使い方" width="1000"></h2>

入力メニューで選択し、テキスト欄にローマ字を入力します。候補や未確定文字が出たら、次のキーで操作します。

| キー | 操作 |
| --- | --- |
| Space | 変換・候補の切替 |
| Enter | 確定 |
| Esc | 取消 |
| ← / → | 変換中の文節選択 |
| F6 / F7 / F8 | ひらがな / カタカナ / 半角カタカナ |
| F9 / F10 | 全角英数 / 半角英数 |
| 英数 / かな | 対応するJISキーボードで直接入力 / 日本語入力へ切替 |

ファンクションキーが明るさ等の操作になる場合は、Macのキーボード設定やFnキーの動作も確認してください。
アプリ別の通常入力・コード入力については [アプリ別設定](../rust/APP-RULES.md) を参照してください。

<a id="設定辞書"></a>
<h3><img src="../assets/branding/readme/mac/headings/6a6763f17c9f.svg" alt="設定・辞書" width="1000"></h3>

共通Coreの既定データ保存先は `~/Library/Application Support/Meltype-flash` です。
明示した `MELTYPE_FLASH_DATA_DIR` または互換用の `MELTYPE_RUST_DATA_DIR` がある場合は、その保存先を使います。
旧本家のデータを自動で取り込む前提にはしないでください。利用できる管理画面と辞書操作の範囲は [Rust版の構成](../rust/README.md) を参照してください。

<a id="プライバシー"></a>
<h2><img src="../assets/branding/readme/mac/headings/5b98193660c4.svg" alt="プライバシー" width="1000"></h2>

入力の判定・変換はローカルのCoreと同梱変換エンジンで行います。学習や辞書などの個人データは上記の保存先に置きます。
初回の**ソースビルド**では、Rust・Swiftの依存関係や辞書のダウンロードが発生します。
不具合報告に個人の文章やデータフォルダー全体を添付しないでください。

<a id="困ったとき"></a>
<h2><img src="../assets/branding/readme/mac/headings/4466f22d7395.svg" alt="困ったとき" width="1000"></h2>

<details>
<summary>入力ソースに出てこない・起動しない</summary>

ZIPを全体展開したか、CPUに合う版か、`~/Library/Input Methods/Meltype.app` があるかを確認します。
入力ソースの追加画面で日本語のMeltypeを探し、見つからなければログアウト・ログインして確認してください。
標準のABCや日本語入力へ戻せる状態を保ち、エラー全文とOS・CPUを [Issues](https://github.com/appipinopi/Meltype-Flash/issues/new/choose) に報告してください。

</details>

<details>
<summary>本家の説明と画面・変換結果が違う</summary>

FlashはRustプレビューです。本家の `.NET` ビルドやHomebrew caskを使うと別の実装になります。
Flashのソースから作る場合は、下記の `rust/build-desktop.sh` を使ってください。

</details>

<a id="アンインストール"></a>
<h2><img src="../assets/branding/readme/mac/headings/9648f59fe13b.svg" alt="アンインストール" width="1000"></h2>

1. 入力メニューでABCなど別の入力ソースを選びます。
2. システム設定のキーボードからMeltypeの入力ソースを外します。
3. Finderの「フォルダへ移動」で `~/Library/Input Methods` を開き、今回導入した `Meltype.app` を取り除きます。

設定・学習データは別フォルダーです。残しておけば再導入時に利用できます。削除する場合は必要なデータを先に保存し、対象がFlashのフォルダーであることを確認してください。
本家と同名のバンドルを使っている場合は、取り除くものを確認してください。

<a id="開発者向け"></a>
<h2><img src="../assets/branding/readme/mac/headings/9f8c2f09ae3c.svg" alt="開発者向け" width="1000"></h2>

<a id="ソースからビルドする"></a>
<h3><img src="../assets/branding/readme/mac/headings/f9d4e9685bb1.svg" alt="ソースからビルドする" width="1000"></h3>

macOS、XcodeまたはCommand Line Tools、Rust 1.88以上が必要です。リポジトリのルートから実行します。

```sh
bash rust/build-desktop.sh
```

`rust/build-desktop.sh` はRustのライブラリと登録用ヘルパーを作り、SwiftホストをRust Core向けにビルドします。ビルド結果は `mac/build/Meltype.app`。ビルドだけでは入力ソースの選択は終わりません。
通常の導入は、同梱インストーラーまたは `mac/install.sh` が期待する配布構成で行ってください。
`mac/build.sh` を単独で実行すると本家C# Coreのビルド経路になるため、Flash用には使いません。

共通Coreは `cargo test -p meltype-core --locked`、MacのOS連携は [Mac用CI](../.github/workflows/macos-rust.yml) と [移行記録](../rust/MIGRATION.md) を参照してください。ABI・IMKを使う試験の成功と、物理端末の各アプリでの入力確認は別です。

<a id="ライセンス"></a>
<h2><img src="../assets/branding/readme/mac/headings/7b96bc7bad42.svg" alt="ライセンス" width="1000"></h2>

[GPL-3.0-or-later](../LICENSE)。変換エンジン・Swift依存関係の表示は [THIRD-PARTY-NOTICES.md](../THIRD-PARTY-NOTICES.md) を参照してください。
