# OSごとのアプリ別入力設定

Windowsは`Code.exe`のような実行ファイル名、Linuxは`code`のようなアプリ名、Macは`com.microsoft.VSCode`のようなアプリIDを指定します。設定の「アプリを追加」で、そのOSの名前を登録できます。Macの`.app`フォルダー名をWindowsの`.exe`へ変更する必要はありません。

| OS | VS Code | ターミナルの例 |
| --- | --- | --- |
| Windows | `Code.exe` | `WindowsTerminal.exe` |
| Linux | `code`、`code-insiders` | `gnome-terminal-server`、`konsole`、`kitty` |
| Mac | `com.microsoft.VSCode` | `com.apple.Terminal`、`com.googlecode.iterm2` |

既定のWindows名が保存済みでも、対応する既知のLinux名・MacのアプリIDから同じ設定を参照します。たとえば`code`は既存の`Code.exe`の「コード」設定を使います。ネイティブ名を直接登録した設定を優先し、同じ名前では最後の設定を使います。未知の実行ファイルから機械的に`.exe`を取り除く処理はしません。アプリ名やIDは実行環境によって異なる場合があるため、その場合は取得される名前で個別に登録してください。

## 入力元の取得

- Windowsキーボード方式: 前面ウィンドウの実行ファイル名でアプリ別設定を適用します。「ゲーム」または無効の指定ではキーを捕捉しません。「入力欄とみなすアプリ」には `Everything.exe` などをカンマ区切りで指定できます。この指定でもパスワード欄・読み取り専用欄・無効な欄は除外します。

- Mac: 入力開始時に`NSWorkspace`の前面アプリからアプリIDを取得し、Rustへ渡します。入力メニューで明示的に選んだ英数・コード入力は維持します。
- Linux / fcitx5: 入力コンテキストの`program()`を使います。Terminalフラグによる既存のコード入力も維持します。
- Linux / IBus: 対応するIBusでは`has-focus-id`を有効にし、`focus-in-id`の`gtk3-im:code`などからアプリ名を取得します。`xim`・`gnome-shell`・`QIBusInputContext`など、アプリ名が渡されない入力経路では推測しません。旧IBusでは従来の入力方法へ戻し、アプリ別の自動切替は使いません。

参照: [fcitx5の入力コンテキスト](https://github.com/fcitx/fcitx5/blob/master/src/lib/fcitx/inputcontext.h)、[IBusのfocus-in-id定義](https://github.com/ibus/ibus/blob/main/src/ibusengine.c)。Linux・Macの全アプリで情報を取得できることを保証するものではありません。

## 設定画面のクリック修正

設定画面全体の右クリックメニューを、子のスイッチや入力欄より後に登録していたため、透明なクリック領域が操作を遮っていました。背景の領域を先に登録し、各設定項目を操作できるようにしました。設定画面全体へのマウス押下・解放でスイッチが変わる回帰テストを追加しています。これはWindowsだけでなく、同じRust設定画面を使うLinux・Macにも適用されます。
