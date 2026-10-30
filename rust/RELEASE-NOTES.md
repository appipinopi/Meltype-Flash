# Meltype-flash v0.1.0 Pre4

2026年10月23日の開発版です。上流のClosed「優先: 中」145件を照合し、Rust版に残る移植漏れを修正しました。

## Pre4の変更

- Windows: TSFへ全角スペース設定を伝え、通常SpaceとShift+Spaceを設定どおり処理します。
- Windows: Brave等のChromium系ブラウザーで、通常欄にアプリ独自のPassword属性だけが付く場合を処理します。Windows側の秘密欄指定と読み取り失敗時の入力拒否は維持します。
- Windows: メニュー操作中のキーをアプリへ渡し、変換中のCtrl+K/Lを文節長変更の処理へ届けます。
- Windows: Keyboard方式で遠隔操作ツール等の注入入力設定を反映し、Chrome Remote Desktop・Parsecの稼働判定を追加しました。自己送信したキーは再処理しません。
- 共通Core: zh/zj/zk/zl等の矢印・記号入力、gameoverの英語保持、Fキーで表記を選んだ後の新しい入力を修正しました。
- macOS: 辞書選択パネルの有効化と前面表示を追加しました。macOSでのビルド・実行確認は未実施です。
- Linux: 設定画面の初回案内にWindows専用の導入手順が表示される問題を修正しました。WSLg上のGTK入力欄と実IBusエンジンで、日本語の入力・確定を確認しました。

全145件の実機での解消を保証するものではありません。行頭スラッシュの新仕様、TSFの左右Alt単独切替などに差分が残っています。Rustの対象試験274件とWindows側の追加試験327件が成功しました。

## Pre3までの変更

2026年10月22日のプレリリースです。Pre2以降に確認したWindows・Linux・macOS・Androidと共通Coreの不具合を修正しました。

- Windows: ポータブル入力の修飾キー・長文・フォーカス切替・アプリ別設定を修正。変換キーを処理し、メールアドレス等の入力欄では英数字を直接入力します。
- Windows: トレイ、終了処理、報告・修正履歴ダイアログを改善。テキスト入力アプリの設定を実際の対象判定へ接続しました。
- Linux: フォーカス移動時の二重確定、機密欄と学習抑止の区別、周辺テキスト削除の検証、候補選択、カーソル移動、Shift+Tabを修正。IBusの設定画面起動とインストール失敗時の復元を改善しました。
- macOS: マーク中の文字列を置き換える際、既存文字列と照合してからマークを解除し、重複挿入を防止します。
- Android: フリック入力と実際の編集処理を接続し、操作・ライフサイクルの回帰検査を追加しました。
- 共通Core: 登録名「ちな→千奈」により「ちなみに」が「千奈みに」となる分割を修正。OpenRouter・TokenRouterを英単語辞書へ追加しました。

上流の高優先度Issueはクローズ済みを含む42件を照合し、現在のRust版に確認できた不具合を修正しました。各Issueに挙がる実アプリ・実機条件すべてでの解消を保証するものではありません。ベンチマークは同じコンパイラー・21入力ケースでPre2と比較し、結果と測定条件をリリース本文に掲載します。測定資料はダウンロード用に添付しません。

## 配布と導入

共通Rustエンジンを使うWindows・Linux・macOS・Android・ChromeOS向けプレビューです。元C#版と全機能・変換精度が同等の完成版ではありません。実機・実アプリでの受入確認が残っています。

| 環境 | 配布ファイル | 導入 |
|---|---|---|
| Windows 10/11 x64 | Meltype-flash-Windows-x64.zip | 全体を展開し、README-Windows.mdを参照。管理者PowerShellでManage-Windows.ps1を実行。32bitアプリ用TIP/workerも同梱 |
| Linux x64（Ubuntu 24.04でビルド） | Meltype-flash-Linux-x64.tar.gz | 展開後、sudo bash install.sh --system。システムのIBus/GLibと設定画面のGUI実行環境が必要 |
| macOS 13以降・Apple Silicon | Meltype-flash-macOS-arm64.zip | ZIP全体を展開し、Install Meltype.commandを実行 |
| macOS 13以降・Intel | Meltype-flash-macOS-x64.zip | ZIP全体を展開し、Install Meltype.commandを実行 |
| Android 8.0以降・arm64/x86_64 | Meltype-flash-Android-debug.apk | 開発検証用APKをインストール後、システムのキーボード設定でMeltype-flash Previewを有効化 |
| ChromeOS | Meltype-flash-ChromeOS.zip | 展開後、chrome://extensionsで「パッケージ化されていない拡張機能を読み込む」。入力方法にMeltype-flashを追加 |

Windowsは元C#版と異なる専用COM登録IDを使います。配布ZIPを展開するだけではIME登録は行いません。導入済みのFlash版を更新する場合は、同梱の`Manage-Windows.ps1 -Action Update`を管理者PowerShellで実行してください。旧バージョンのファイルと個人データは保持します。設定画面にはOpenGL 2.0以上が必要です。CIではソフトウェアOpenGLで実トレイの終了試験を行い、実Windowsでも設定クリックと更新を確認しています。

LinuxはRust製IBus版です。Mozcヘルパーを同梱せず、変換は共通Coreの補助辞書に限られます。fcitx5用パッケージは含みません。IBusがアプリ名を渡さない入力経路（gnome-shellや一部のQt・XIMなど）ではアプリ別の自動切替を行いません。

macOSはSwift InputMethodKitとazooKeyを利用します。署名はアドホック署名で、Developer ID署名・Apple公証は行っていません。初回起動はmacOSの許可操作が必要になる場合があります。

Androidはエミュレーターで検証するdebug APKです。Play Store配布・正式な配布署名・物理端末での確認は未対応です。このAPKの署名鍵はCI実行ごとに変わるため、別のCI生成版からの上書き更新は保証できません。アンインストールするとアプリ内データを失うので、必要なデータは先にバックアップしてください。Mozcは同梱しません。

ChromeOSはManifest V3とchrome.input.imeを使用する試験版です。ChromeOS製品版・管理ポリシーによって導入できない場合があります。Windows/macOS/Linux上のChrome用IMEではありません。Chrome Web Store公開は行っていません。サイト別切替は初期状態では無効です。設定の「サイトごとの自動切替」を有効にし、タブ参照を許可して保存してください。周辺テキストを渡さないエディターではコメント・文字列内の日本語入力を自動判別できないため、そのサイトを通常入力へ変更してください。ChromeOS実機でのPre3の入力検証は未実施です。

RustとAndroidの内部バージョンは`0.1.0-pre.4`です。AndroidのversionCodeとiOSのビルド番号は4です。ChromeOSは既存版から更新できる拡張機能の内部バージョン`0.1.3`を使い、表示名を`0.1.0 Pre4`にしています。

WindowsのC#最新版との比較検査には、`nihongoGitHub`の分割差が残っています。Rustは「にほんご」と「GitHub」に分け、比較対象のC#版は全体を英字として扱います。この比較検査は引き続き有効で、Pre3の共通Core・各OSの動作検査とは別に追跡します。

各ファイルのSHA-256はSHA256SUMS.txtを参照してください。WindowsとChromeOSのZIPには内部ファイルのハッシュも含みます。ソースと各環境の詳細は同じリリースタグ内のrust/README.md、android/README.md、chromeos/README.mdを参照してください。
