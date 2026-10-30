<!-- meltype-flash-platform:start -->
<p align="center"><img src="../assets/branding/readme/logo.svg" alt="Meltype-flash" width="520"></p>
<a id="meltype-for-chromeos"></a>
<p align="center"><img src="../assets/branding/readme/chromeos/hero.svg" alt="Meltype-flash for ChromeOS" width="1000"></p>
<p align="center"><b>稲妻のように、言葉の壁を駆け抜ける日本語入力。</b></p>
<p align="center"><a href="https://github.com/appipinopi/Meltype/releases/tag/flash-v0.1.0-preview.3"><img src="https://img.shields.io/badge/Flash-v0.1.0_Pre3-ffe34a" alt="Flash v0.1.0 Pre3"></a> <img src="https://img.shields.io/badge/status-開発プレビュー-666973" alt="開発プレビュー"></p>
<p align="center"><a href="#インストール">ダウンロード・導入</a> · <a href="#使い方">使い方</a> · <a href="#困ったとき">困ったとき</a> · <a href="https://github.com/appipinopi/Meltype/issues/new/choose">不具合の報告・提案</a></p>
<p align="center">
  <a href="../rust/WINDOWS-PACKAGE.md"><img src="../assets/branding/readme/cards/os-windows.svg" alt="Windows" width="32%"></a>
  <a href="../linux/README.md"><img src="../assets/branding/readme/cards/os-linux.svg" alt="Linux" width="32%"></a>
  <a href="../mac/README.md"><img src="../assets/branding/readme/cards/os-mac.svg" alt="Mac" width="32%"></a>
</p>
<p align="center">
  <a href="../android/README.md"><img src="../assets/branding/readme/cards/os-android.svg" alt="Android" width="32%"></a>
  <a href="README.md"><img src="../assets/branding/readme/cards/os-chromeos-active.svg" alt="ChromeOS" width="32%"></a>
  <a href="../ios/README.md"><img src="../assets/branding/readme/cards/os-ios.svg" alt="iOS" width="32%"></a>
</p>
<p align="center"><a href="../README.md">トップのREADME</a></p>
<!-- meltype-flash-platform:end -->

ChromeOSの `chrome.input.ime` と共通Rust CoreのWASMを接続する、日本語入力の試験版です。
**通常のWindows・Mac・Linux上のChrome用IMEではありません。Chrome Web Storeへは公開していません。**

<a id="内容"></a>
<h2><img src="../assets/branding/readme/chromeos/headings/df8764ebcf5d.svg" alt="内容" width="1000"></h2>

ローマ字・かな、英語・日本語の判定、漢字変換、予測候補、候補選択、設定と学習の保存を接続しています。
AI変換は搭載しません。デスクトップの設定を読み込んでも、Zenzaiの有効化・サーバー・モデルの設定は取り込みません。通常の辞書変換・予測候補・学習は端末内で使えます。
漢字変換には、固定版Mozcの語彙と接続コストを同梱しています。「にほんご → 日本語」「きょうはいいてんきです → 今日はいい天気です」のように、端末内で変換します。Mozc本体とは異なる上限付きの探索処理であり、候補や順位はデスクトップ版と一致するとは限りません。
設定画面では、通常入力とコード入力、サイトごとの切替、入力のカスタマイズ、詳細JSON、スナップショットの保存を扱います。

「入力のカスタマイズ」で日英判定、句読点、タイプミス補正、Tab変換、英語前後の空白、学習を変更し、「入力設定を保存」で反映します。詳細JSONとも同期します。「保存済み設定へ戻す」で未保存の編集を取り消せます。学習をオフにしても既存の履歴は削除しません。
候補はChromeOS標準の表示を使い、色・フォント・位置の変更は未対応です。[カスタマイズの進め方](../docs/customization.md) に対応範囲と次の案をまとめています。
[導入画面の記録](docs/INSTALLATION.md) はありますが、Pre3の修正を含む実機での変換・各アプリ入力の確認は未完了です。

[動く環境](#動く環境) · [インストール](#インストール) · [使い方](#使い方) · [困ったとき](#困ったとき) · [アンインストール](#アンインストール) · [開発者向け](#開発者向け)

<a id="動く環境"></a>
<h2><img src="../assets/branding/readme/chromeos/headings/f378dd1f4672.svg" alt="動く環境" width="1000"></h2>

- `chrome.input.ime` を利用できるChromeOS。
- Manifest V3の拡張機能を手動読込できる環境。
- パッケージの最低Chromeバージョン指定は119。119以降の全版で動作確認した意味ではありません。

管理端末では導入や権限が制限される場合があります。OSのDeveloper Modeへ切り替える手順ではありません。

<a id="インストール"></a>
<h2><img src="../assets/branding/readme/chromeos/headings/4f36b3b314d8.svg" alt="インストール" width="1000"></h2>

1. [FlashのPre3](https://github.com/appipinopi/Meltype/releases/tag/flash-v0.1.0-preview.3) から `Meltype-flash-ChromeOS.zip` を取得し、フォルダーへ全体展開します。
2. ChromeOSの `chrome://extensions` を開き、拡張機能の「デベロッパーモード」を有効にします。
3. 「パッケージ化されていない拡張機能を読み込む」で、`manifest.json` のあるフォルダーを選びます。
4. ChromeOSの言語・入力方法の設定からMeltypeを追加し、入力方法として選びます。

メニューの表示名に `Meltype Rust` が残っている版もあります。画面の経路は [画像付きの導入案内](docs/INSTALLATION.md) を参照してください。
既存版を更新する場合は、先に設定画面で必要なスナップショットを保存します。同じフォルダーを新しいファイルで更新し、拡張機能を再読み込みしてください。学習を残したい場合は、先に拡張機能を削除しないでください。

<a id="使い方"></a>
<h2><img src="../assets/branding/readme/chromeos/headings/936b17951891.svg" alt="使い方" width="1000"></h2>

| 操作 | 動作 |
| --- | --- |
| 文字キー | 共通Coreでローマ字・かなと英語を判定 |
| 半角/全角 / Alt+Backquote | 日本語と半角英数を切替。未確定入力は確定してから切替。通常のBackquoteは記号として入力。 |
| Space | 変換・候補切替 |
| 候補クリック / Enter | 候補を選択して確定 |
| Esc | 取消 |
| F6 / F7など | 対応する字種へ切替。OSに取られるキーは届かない場合があります。 |

例: `hashi` → Space → 「橋」「箸」の候補を選択 → Enterで確定。

ChatGPTなどのチャット欄では、`chatgptwotsukau` → Space → `chatgptを使う` → Enterで確定します。英字の文節はそのまま残し、日本語の文節を変換します。複数文節の候補は選択中の文節に対するものです。左右キーで文節を選び、Enterで文全体を確定できます。未確定入力がある間のCtrl+Enterは確定だけを行い、次のCtrl+Enterをアプリへ渡します。この手順は実WASMと模擬IME APIで検証しており、ChromeOS実機のChatGPT画面での確認は未実施です。

ChromeOSの入力方式切替はOS側の設定を使います。

<a id="サイトごとの通常入力コード入力"></a>
<h3><img src="../assets/branding/readme/chromeos/headings/56ec4448115b.svg" alt="サイトごとの通常入力・コード入力" width="1000"></h3>

初期状態ではサイト別の自動切替は無効です。設定画面で「サイトごとの自動切替」を有効にし、必要なタブ参照を許可して保存します。
`vscode.com`・`vscode.dev`・`github.dev` は初期候補です。権限を許可しない場合は手動の入力モードを使えます。
周辺テキストを渡さないエディターではコメント・文字列の判定ができないため、そのサイトは通常入力へ切り替えてください。
サイト別設定はCoreの学習スナップショットとは別で、現在のJSON書き出しには含まれません。

<a id="プライバシー"></a>
<h2><img src="../assets/branding/readme/chromeos/headings/5b98193660c4.svg" alt="プライバシー" width="1000"></h2>

入力と変換は拡張機能内のWASMで処理し、設定・学習は `chrome.storage.local` に保存します。学習禁止やシークレットでは保存を抑止します。
サイト別切替の任意権限 `tabs` は、現在のタブを調べるために使います。ページへ入力用スクリプトを挿入する構成ではありません。
拡張機能の削除でローカルデータを失うため、必要なスナップショットは先に保存してください。

<a id="困ったとき"></a>
<h2><img src="../assets/branding/readme/chromeos/headings/4466f22d7395.svg" alt="困ったとき" width="1000"></h2>

<details>
<summary>拡張機能として入ったのに入力方法が出ない</summary>

ChromeOSであること、OS側の入力方法へ追加したこと、管理ポリシーで許可されていることを確認します。
通常のデスクトップChromeではこのIME APIを使えません。拡張機能画面に表示されるだけでIME対応とは判断できません。

</details>

<details>
<summary>英字のまま・候補が出ない・設定が切り替わらない</summary>

まずSpaceで候補を確認します。サイトの周辺文字が取れない場合は通常入力へ変更し、更新後は拡張機能を再読み込みしてください。
旧版は補助辞書しか接続しておらず、「橋」など一部の語以外は漢字候補が出ませんでした。修正版では `pkg/kanji.bin` も必要です。JSやWASMだけを個別にコピーせず、同じ拡張機能フォルダーをZIPの全内容で更新してから、拡張機能を再読み込みしてください。
OS・拡張機能の版・サイト・入力手順を [Issues](https://github.com/appipinopi/Meltype/issues/new/choose) に記載してください。タブURLや文章に個人情報がないか確認してください。

</details>

<a id="アンインストール"></a>
<h2><img src="../assets/branding/readme/chromeos/headings/9648f59fe13b.svg" alt="アンインストール" width="1000"></h2>

入力方法を別のものへ切り替え、ChromeOSの入力方法一覧からMeltypeを外してから、`chrome://extensions` で拡張機能を削除します。
削除前に必要な設定・学習スナップショットを保存してください。展開フォルダーも不要になってから削除します。

<a id="開発者向け"></a>
<h2><img src="../assets/branding/readme/chromeos/headings/9f8c2f09ae3c.svg" alt="開発者向け" width="1000"></h2>

Rustと `wasm-bindgen-cli`、Pythonが必要です。リポジトリのルートから実行します。
初回ビルドでは `native/mozc/MOZC_COMMIT` に固定したMozcの辞書と接続コストを取得します。取得したデータは `tools/data/chromeos-mozc/<commit>/` に保存し、次回から再利用します。実行時の外部通信は不要です。

```sh
python chromeos/build.py
```

`dist/chromeos-preview` と同名ZIPを作ります。別のwasm-bindgen実行ファイルは `--wasm-bindgen`、出力先は `--out` で指定します。`CARGO_TARGET_DIR` も尊重します。
実WASMでの変換・候補・学習復元、模擬IME APIでの順序・UTF-16位置・競合・エラーを検査します。
模擬APIでの成功とChromeOS実機での成功は別です。[移行記録](../rust/MIGRATION.md) と [ビルドスクリプト](build.py) も参照してください。
パッケージ内の `pkg/MOZC-DICTIONARY-NOTICES.txt` に、辞書の出典・固定commit・取得ファイルのSHA256・ライセンス本文を含めています。

<a id="ライセンス"></a>
<h2><img src="../assets/branding/readme/chromeos/headings/cfc7325a7d0e.svg" alt="ライセンス" width="1000"></h2>

[GPL-3.0-or-later](../LICENSE)。ZIPにはライセンス・第三者表示・解決済み依存関係の表示を同梱します。[THIRD-PARTY-NOTICES.md](../THIRD-PARTY-NOTICES.md) を参照してください。
