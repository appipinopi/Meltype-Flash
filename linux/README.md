<!-- meltype-flash-platform:start -->
<p align="center"><img src="../assets/branding/readme/logo.svg" alt="Meltype-flash" width="520"></p>
<a id="meltype-for-linux"></a>
<p align="center"><img src="../assets/branding/readme/linux/hero.svg" alt="Meltype-flash for Linux" width="1000"></p>
<p align="center"><b>稲妻のように、言葉の壁を駆け抜ける日本語入力。</b></p>
<p align="center"><a href="https://github.com/appipinopi/Meltype-Flash/releases/tag/flash-v0.1.0-preview.7"><img src="https://img.shields.io/badge/Flash-v0.1.0_Pre7-ffe34a" alt="Flash v0.1.0 Pre7"></a> <img src="https://img.shields.io/badge/status-開発プレビュー-666973" alt="開発プレビュー"></p>
<p align="center"><a href="#インストール">ダウンロード・導入</a> · <a href="#使い方">使い方</a> · <a href="#困ったとき">困ったとき</a> · <a href="https://github.com/appipinopi/Meltype-Flash/issues/new/choose">不具合の報告・提案</a></p>
<p align="center">
  <a href="../rust/WINDOWS-PACKAGE.md"><img src="../assets/branding/readme/cards/os-windows.svg" alt="Windows" width="32%"></a>
  <a href="README.md"><img src="../assets/branding/readme/cards/os-linux-active.svg" alt="Linux" width="32%"></a>
  <a href="../mac/README.md"><img src="../assets/branding/readme/cards/os-mac.svg" alt="Mac" width="32%"></a>
</p>
<p align="center">
  <a href="../android/README.md"><img src="../assets/branding/readme/cards/os-android.svg" alt="Android" width="32%"></a>
  <a href="../chromeos/README.md"><img src="../assets/branding/readme/cards/os-chromeos.svg" alt="ChromeOS" width="32%"></a>
  <a href="../ios/README.md"><img src="../assets/branding/readme/cards/os-ios.svg" alt="iOS" width="32%"></a>
</p>
<p align="center"><a href="../README.md">トップのREADME</a></p>
<!-- meltype-flash-platform:end -->

Linuxの入力ソースとして使うRust版Meltype-flashです。**公開プレビューはRust製IBusエンジン**です。
fcitx5アダプターのソースと試験はありますが、公開IBusパッケージにfcitx5版は同梱していません。

<a id="内容"></a>
<h2><img src="../assets/branding/readme/linux/headings/df8764ebcf5d.svg" alt="内容" width="1000"></h2>

ローマ字・かな入力、英語・日本語の判定、補助辞書による漢字候補、確定、設定・学習・辞書保存を共通Coreへ接続しています。
公開版にはMozcヘルパーを同梱していないため、辞書の規模や変換結果には制限があります。

[動く環境](#動く環境) · [インストール](#インストール) · [使い方](#使い方) · [困ったとき](#困ったとき) · [アンインストール](#アンインストール) · [開発者向け](#開発者向け)

<a id="動く環境"></a>
<h2><img src="../assets/branding/readme/linux/headings/f378dd1f4672.svg" alt="動く環境" width="1000"></h2>

- Ubuntu 24.04以降向けに作成したx64 / ARM64パッケージ。
- IBusを使用するデスクトップ環境。GNOMEの「日本語入力」などへ追加します。
- システムのIBus / GLibと、設定画面を動かすGUI環境。

他のディストリビューションや、Wayland / X11・GTK / Qtの組合せすべてでの動作を保証するものではありません。
fcitx5を使う環境は、下記のソースビルドと開発状況を参照してください。

<a id="インストール"></a>
<h2><img src="../assets/branding/readme/linux/headings/4f36b3b314d8.svg" alt="インストール" width="1000"></h2>

<a id="ubuntu向けdeb"></a>
<h3><img src="../assets/branding/readme/linux/headings/a3beabf6efc7.svg" alt="Ubuntu向けdeb" width="1000"></h3>

Pre7ではdebとARM64版を添付していません。次のx64向けtar.gzを使ってください。

[Pre7](https://github.com/appipinopi/Meltype-Flash/releases/tag/flash-v0.1.0-preview.7)の`Meltype-flash-Linux-x64.tar.gz`を全体展開し、`Meltype-flash-linux`フォルダーで実行します。

```sh
bash install.sh
```

導入後はログアウト・ログインし、OSのIBus入力ソースにMeltype-flashを追加して選びます。標準の配布物は補助辞書による変換に制限があります。

<a id="使い方"></a>
<h2><img src="../assets/branding/readme/linux/headings/f2353c3aa1bc.svg" alt="使い方" width="1000"></h2>

入力ソースをMeltype-flashへ切り替え、テキスト欄にローマ字を入力します。

| キー | 操作 |
| --- | --- |
| Space | 変換・候補の切替 |
| Enter | 確定 |
| Esc | 取消 |
| ← / → | 変換中の文節選択 |
| F6〜F10 | ひらがな・カタカナ・英数などの字種切替 |

補助辞書による候補を確認する例は `hashi` → Space → 「橋」「箸」などを選んでEnterです。
入力ソースの切替キーはOS側の設定を使います。
GNOMEの入力ソースにある「設定」からも設定画面を開けます（この修正を含むZIP・debを導入した場合）。
設定アプリは `meltype-flash`、共通CLIは `meltype-rs` です。インストール形式によってはフルパスで起動します。
アプリ別設定は [専用の案内](../rust/APP-RULES.md) を参照してください。IBusがアプリ名を渡さない経路では、自動切替を行いません。

<a id="プライバシー"></a>
<h2><img src="../assets/branding/readme/linux/headings/5b98193660c4.svg" alt="プライバシー" width="1000"></h2>

共通Coreのデータは `${XDG_DATA_HOME:-~/.local/share}/Meltype-flash` に保存します。`MELTYPE_FLASH_DATA_DIR` で明示した保存先はそちらを優先します。
通常の変換はローカルで処理します。設定・学習・辞書のバックアップには個人の語句が含まれる場合があります。

<a id="困ったとき"></a>
<h2><img src="../assets/branding/readme/linux/headings/4466f22d7395.svg" alt="困ったとき" width="1000"></h2>

<details>
<summary>入力ソースに出てこない</summary>

まずIBusを使う環境か、導入後にログインし直したかを確認してください。
ユーザー領域への導入ではIBusが追加のコンポーネント検索パスを必要とするため、通常の導入は上記のシステム導入を使います。
`fcitx5` の設定画面にIBus版を追加することはできません。

</details>

<details>
<summary>変換の候補が少ない・アプリによって入力できない</summary>

公開IBus版は補助辞書による制限があります。OS・デスクトップ・Wayland/X11・アプリ名・入力例を [Issues](https://github.com/appipinopi/Meltype-Flash/issues/new/choose) に添えてください。
端末や入力方式の違いを確かめるため、別の一般的なテキスト欄でも結果を比較してください。

</details>

<a id="アンインストール"></a>
<h2><img src="../assets/branding/readme/linux/headings/9648f59fe13b.svg" alt="アンインストール" width="1000"></h2>

先にOSの入力ソースを別のものへ切り替え、Meltype-flashを入力ソース一覧から外します。

debで導入した場合:

```sh
sudo apt remove meltype-flash
```

ZIPの `--system` で導入した場合:

```sh
sudo bash /usr/libexec/meltype-flash/ibus/install.sh --system --uninstall
```

個人の設定・学習は残ります。必要なデータを保存してから、削除するか判断してください。

<a id="開発者向け"></a>
<h2><img src="../assets/branding/readme/linux/headings/9f8c2f09ae3c.svg" alt="開発者向け" width="1000"></h2>

<a id="ソースからビルドする"></a>
<h3><img src="../assets/branding/readme/linux/headings/f9d4e9685bb1.svg" alt="ソースからビルドする" width="1000"></h3>

Rust 1.88以上、Cリンカー、IBus / GLibの開発用パッケージが必要です。リポジトリのルートで実行します。

```sh
cargo test -p meltype-core --locked
bash rust/build-linux.sh
```

`linux/build/Meltype-flash-linux` とtar.gzを作成します。ビルドだけではインストールしません。
Mozcヘルパーがない場合は、補助辞書によるパッケージを作ります。
fcitx5のRustアダプターは `linux/fcitx5` にあり、必要な開発用ライブラリとCMakeを使います。[Rust用CI](../.github/workflows/rust.yml) と [移行記録](../rust/MIGRATION.md) を参照してください。
本家向けの `linux/build.sh` / `linux/install.sh` とFlashの上記スクリプトを取り違えないでください。

<a id="ターミナルと追加の切替キー"></a>
<h3><img src="../assets/branding/readme/linux/headings/b5a47c3564e2.svg" alt="ターミナルと追加の切替キー" width="1000"></h3>

ターミナルは英数のコード入力を既定にします。半角/全角でその入力行を日本語にし、もう一度押すと英数へ戻せます。カーソル移動・Tab・Ctrl操作で入力欄へキーを渡した場合や、フォーカスを移した場合は、コード入力へ戻ります。

Rust版IBusとfcitx5では、既存の設定 `ToggleKeyboardShortcuts` に `Shift+Space; Ctrl+Space` のように指定すると、それらを切替キーとして処理します。切替時は表示中の入力を一度だけ確定し、余分な空白を挿入しません。設定が空なら、Shift+Spaceの従来の動作を保ちます。

IBusはWSLの隔離したXvfb・D-Bus環境でも検証しています。通常のデスクトップや各アプリでの実機確認とは区別してください。

<a id="ライセンス"></a>
<h2><img src="../assets/branding/readme/linux/headings/7b96bc7bad42.svg" alt="ライセンス" width="1000"></h2>

[GPL-3.0-or-later](../LICENSE)。辞書と依存ライブラリのライセンスは [THIRD-PARTY-NOTICES.md](../THIRD-PARTY-NOTICES.md) を参照してください。
