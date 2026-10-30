# 上流Issueの対応記録

対象: [yksr-melt/Meltypeの未解決Issue](https://github.com/yksr-melt/Meltype/issues)。2026年10月24日に取得した全242件。

これはMeltype-flash側の記録です。上流Issueの変更・終了は行いません。未解決Issueには不具合・辞書要望・機能提案・外部サービスに関する相談が含まれます。

「共通コア確認」は回帰テストの条件で確認した範囲です。OS固有の変換エンジン、報告アプリの画面操作、署名・公証、実機確認まで完了したことを意味しません。既存コードにIssue番号があっても修正済みとは判定しません。

以下はPre6の準備時に記録した対応と検証範囲です。210件等の検証結果はPre6へ版番号を更新する前の修正コードでの結果であり、Pre7最終コミットの結果ではありません。Pre7にはこれらの変更を取り込みます。公開状況と配布ビルドは[リリースノート](../rust/RELEASE-NOTES.md)と公開リリース本文を参照してください。

全242件への対応は未完了です。48件分について限定された修正・確認・部分対応を記録しました。未対応・紹介等の14件にも理由を記載し、残り180件は未検証です。追加操作は[設定と候補](ISSUE-OPTIONS.md)を参照してください。

## 軽量化

辞書の読み取り専用データをArcで共有し、変更時だけ該当データを複製します。辞書複製時のRust追加ヒープ要求量は4,189,182バイトから0バイトになりました。初期辞書・OSのRSS・アロケーターの管理領域・変換エンジンは別です。測定コマンド: `cargo run -p meltype-core --release --locked --example memory_footprint`。

## 全件一覧

| Issue | 内容 | Flash側の状況 |
| --- | --- | --- |
| [#1125](https://github.com/yksr-melt/Meltype/issues/1125) | [誤判定] | 共通コア修正・回帰テスト確認。通常／点数方式で報告された英語の綴りを保つ。報告アプリでの実機確認は未実施。 |
| [#1124](https://github.com/yksr-melt/Meltype/issues/1124) | [不具合] 特定のアプリ等で再変換できない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1123](https://github.com/yksr-melt/Meltype/issues/1123) | [誤判定] ost | 共通コア修正・回帰テスト確認。通常／点数方式で報告された英語の綴りを保つ。報告アプリでの実機確認は未実施。 |
| [#1122](https://github.com/yksr-melt/Meltype/issues/1122) | [誤判定] midi | 共通コア修正・回帰テスト確認。通常／点数方式で報告された英語の綴りを保つ。報告アプリでの実機確認は未実施。 |
| [#1121](https://github.com/yksr-melt/Meltype/issues/1121) | [誤判定] vocaloid | 共通コア修正・回帰テスト確認。通常／点数方式で報告された英語の綴りを保つ。報告アプリでの実機確認は未実施。 |
| [#1120](https://github.com/yksr-melt/Meltype/issues/1120) | llama-serverを同梱する | 未対応。推論サーバー同梱の機能提案。軽量配布と両立する配布単位・対象モデルの設計が必要。 |
| [#1118](https://github.com/yksr-melt/Meltype/issues/1118) | [不具合] 設定画面が出ません | Windows TSFの右クリックを入力モード切替から設定起動へ修正し、単体テスト確認。実際のタスクバー操作は未検証。 |
| [#1117](https://github.com/yksr-melt/Meltype/issues/1117) | [誤判定] | 共通コア修正・回帰テスト確認。通常／点数方式で報告された英語の綴りを保つ。報告アプリでの実機確認は未実施。 |
| [#1116](https://github.com/yksr-melt/Meltype/issues/1116) | [不具合] 変換候補に設定したGithubリポジトリの Issue/PRの内容が出ない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1115](https://github.com/yksr-melt/Meltype/issues/1115) | [再現条件不明] 漢字変換後、その後の一文字が自動変換されなくなる？ | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1114](https://github.com/yksr-melt/Meltype/issues/1114) | [誤判定] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1113](https://github.com/yksr-melt/Meltype/issues/1113) | 無変換キーの動作について | MuhenkanKatakanaCycleを追加。無変換で全角カタカナ→半角カタカナ→ひらがな。共通コア・native ABI・隔離IBusで検証。設定UIにも表示。報告アプリ実機は未確認。 |
| [#1112](https://github.com/yksr-melt/Meltype/issues/1112) | Windows (Meltype IME): Meltype.exe を起動し直した後、各アプリで最初に打ったキーが英字のまま入る | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1111](https://github.com/yksr-melt/Meltype/issues/1111) | [不具合] 「英単語の前後に半角スペース」を有効にすると、日本語単語入力後にも半角スペースが入ることがある | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1110](https://github.com/yksr-melt/Meltype/issues/1110) | [不具合] 公式サイトにて | 未対応。上流のbot／公開サイトの報告。該当する運用コードと公開先の確認が必要。 |
| [#1107](https://github.com/yksr-melt/Meltype/issues/1107) | [機能提案] 変換確定のキー設定 | 入力中のCtrl+Enterによる確定を検証。Windows TSFで先に確定してキーをアプリへ渡す経路を修正。TSFキー写像・受理検査と隔離IBusで確認。空入力時はアプリに渡る。自由な割り当ての全OS接続と実アプリ確認は未対応。 |
| [#1106](https://github.com/yksr-melt/Meltype/issues/1106) | [辞書] ブルアカ->ブルーアーカイブ | 補助辞書へ報告の候補を追加し、候補一覧で確認。同音語は複数残す。OSの変換器での順位・実機確認は未実施。 |
| [#1103](https://github.com/yksr-melt/Meltype/issues/1103) | botのhelpコマンドにあるわかりにくい部分を修正する | 未対応。上流のbot／公開サイトの報告。該当する運用コードと公開先の確認が必要。 |
| [#1097](https://github.com/yksr-melt/Meltype/issues/1097) | [提案] AUR (Arch Linux) で入れられるようにする (meltype-bin / meltype-ut) | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1095](https://github.com/yksr-melt/Meltype/issues/1095) | [不具合] 任意のアプリ: 全体に効いてしまっている | 共通コア修正・回帰テスト確認。英語文節へのF6〜F10も選択文節だけへ適用し、元の打鍵が取れないときも文全体へ作用させない。報告アプリでの実機確認は未実施。 |
| [#1094](https://github.com/yksr-melt/Meltype/issues/1094) | [提案] 開発ありがとうございます。何度間違った変換をしても何度間違った変換を修正しても、正しい変換が学習されて上に出てきてくれ… | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1093](https://github.com/yksr-melt/Meltype/issues/1093) | [不具合] コマンドプロンプト、Powershell: コマンドプロンプトにアルファベットしか入らない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1092](https://github.com/yksr-melt/Meltype/issues/1092) | [不具合] Notepads 1.5.6.0: 変換候補のリストが表示されなくなる | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1091](https://github.com/yksr-melt/Meltype/issues/1091) | [誤判定] pa-thisyon → ぱーthisよん (期待: パーティション) | 共通コア修正・回帰テスト確認。パーティション／ドキュメントの読みを英語の断片に分けない。漢字・カタカナ変換器の実機確認は未実施。 |
| [#1090](https://github.com/yksr-melt/Meltype/issues/1090) | [不具合] インストール時: 上記の通り、フリーズする | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1089](https://github.com/yksr-melt/Meltype/issues/1089) | [提案] Windows on ARM64 環境では、現状 Meltype を IME としては利用できず、変換ボックス方式でし… | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1088](https://github.com/yksr-melt/Meltype/issues/1088) | [誤判定] go → go ご (期待: 後) | 補助辞書へ報告の候補を追加し、候補一覧で確認。同音語は複数残す。OSの変換器での順位・実機確認は未実施。 |
| [#1087](https://github.com/yksr-melt/Meltype/issues/1087) | [不具合] Microsoft Excel: 初回の文字が必ずアルファベットで確定されてしまう。 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1086](https://github.com/yksr-melt/Meltype/issues/1086) | [不具合] 全般: 何も表示されない。設定画面が呼び出せず不便でした。 | Windows TSFの右クリックを入力モード切替から設定起動へ修正し、単体テスト確認。実際のタスクバー操作は未検証。 |
| [#1085](https://github.com/yksr-melt/Meltype/issues/1085) | [不具合] どのアプリでも。: こんなkanji | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1084](https://github.com/yksr-melt/Meltype/issues/1084) | [提案] VisualStudio2022(64ビット)17.14.40において、検索小窓(Ctrl+F)を開き半角/全角キーで… | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1079](https://github.com/yksr-melt/Meltype/issues/1079) | [不具合] Windows11 スケーリング150%でタスクトレイからユーザー辞書を開くとウィンドウの右側が見切れる | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1078](https://github.com/yksr-melt/Meltype/issues/1078) | [不具合]ターミナルで利用できない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1077](https://github.com/yksr-melt/Meltype/issues/1077) | [不具合] プレビュー版を利用すると自動アップデートが行われなくなる？ | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1076](https://github.com/yksr-melt/Meltype/issues/1076) | 【辞書】「/」を押したときに「・」の変換が出ないので追加してほしい | 部分対応。/の補助候補に・を追加し、通常のスラッシュ確定を維持。共通コアの候補検査済み。ホストでの明示候補表示は未確認。 |
| [#1075](https://github.com/yksr-melt/Meltype/issues/1075) | [誤判定] | 補助辞書へ報告の候補を追加し、候補一覧で確認。同音語は複数残す。OSの変換器での順位・実機確認は未実施。 |
| [#1073](https://github.com/yksr-melt/Meltype/issues/1073) | [機能提案] config.json の JSON Schema を書き出す | 部分対応。既定値・UI情報からJSON Schemaを生成。CLI管理操作で隣へ書き出して$schemaを保存。型・列挙値・未知の最上位項目に対応。保存とプロファイル切替時の保持を検証。起動時の自動更新・配列内の詳細検証・GUI書き出しは未対応。 |
| [#1047](https://github.com/yksr-melt/Meltype/issues/1047) | 確定した後、前の品詞を確認して自然にする | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1044](https://github.com/yksr-melt/Meltype/issues/1044) | [誤判定] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1043](https://github.com/yksr-melt/Meltype/issues/1043) | [不具合] ATOK: ATOKがIMEとして認識されなくなった。 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1042](https://github.com/yksr-melt/Meltype/issues/1042) | [不具合] メモ帳など全般: Meltypeのインストールはインストーラーを用いました。 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1041](https://github.com/yksr-melt/Meltype/issues/1041) | [誤判定] ryokai → 呂会 (期待: りょかい) | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1039](https://github.com/yksr-melt/Meltype/issues/1039) | [提案] 私は某サッカーゲームをプレイしているのですが、選手名が英字表記のため日本語の文章中に英字表記で選手名を入力することが多… | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1038](https://github.com/yksr-melt/Meltype/issues/1038) | [不具合] Everything 1.4.1.1032 (x64): Meltypeが全く動作しない。IMEモードでも、キーボードモードでも日本語で入力できない。 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1037](https://github.com/yksr-melt/Meltype/issues/1037) | [誤判定] nigakkityuukann → に学期中間 (期待: 二学期中間) | 補助辞書へ報告の候補を追加し、候補一覧で確認。同音語は複数残す。OSの変換器での順位・実機確認は未実施。 |
| [#1036](https://github.com/yksr-melt/Meltype/issues/1036) | [不具合] メモ帳: 1.1文字の読みを登録して書き出すボタンを押して保存 | 共通コアの1文字読みを登録・書き出し・取り込み・再読み込みで確認。報告されたGUIの実機確認は未実施。 |
| [#1033](https://github.com/yksr-melt/Meltype/issues/1033) | [機能提案] Linux (fcitx5・IBus) で Shift+Space を日本語 / 英数の切り替えに使えるようにする | Rust IBus・fcitxイベント処理で既存のToggleKeyboardShortcutsを先に処理。Shift+Space／Ctrl+Spaceで余分な空白を入れず切り替える。WSL IBus・native ABI・fcitxイベント／SDK検査済み。通常デスクトップでの確認は未実施。 |
| [#1032](https://github.com/yksr-melt/Meltype/issues/1032) | [不具合] 全画面表示のGoogleドキュメントにて日本語が打てない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1030](https://github.com/yksr-melt/Meltype/issues/1030) | [誤判定] `notとtte` ではなく `則って` になってほしい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1029](https://github.com/yksr-melt/Meltype/issues/1029) | [機能提案] Spaceで英文入力を途切れさせず、文脈判定とSpace 2回による日本語変換を両立したい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1028](https://github.com/yksr-melt/Meltype/issues/1028) | [誤判定]todo リスト | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1024](https://github.com/yksr-melt/Meltype/issues/1024) | [不具合] Homebrew Cask の SHA-256 不一致の再発とリリース手順の相談 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1023](https://github.com/yksr-melt/Meltype/issues/1023) | [機能提案] 一部ワードを日本語・英語モードによって変換を分ける | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1022](https://github.com/yksr-melt/Meltype/issues/1022) | [不具合]【重要】Linux (IBus): ターミナルのコード入力で、半角/全角 を押しても日本語に切り替えられない | LinuxのRust IBus・共通モードAPIを修正。ターミナルの英数既定から日本語へ切り替え、カーソル・フォーカス移動でコード入力へ戻す。WSLの隔離IBus環境で新規ケースを含む20件成功。通常デスクトップでの確認は未実施。 |
| [#1021](https://github.com/yksr-melt/Meltype/issues/1021) | [機能提案] AZIK | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1020](https://github.com/yksr-melt/Meltype/issues/1020) | [辞書] | いいよこいよ→114514を補助辞書へ追加。候補一覧で検証。実機未確認。 |
| [#1019](https://github.com/yksr-melt/Meltype/issues/1019) | [誤判定] `下ので` ではなく `したので` になってほしい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1018](https://github.com/yksr-melt/Meltype/issues/1018) | [機能提案] 『不具合の報告・提案』画面の「実行環境」テキストを簡単にコピーできるボタン（または機能）を追加してほしい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1017](https://github.com/yksr-melt/Meltype/issues/1017) | [誤判定] `風速6メートル` が `ふうそku6meーとる` になる | 補助辞書へ報告の候補を追加し、候補一覧で確認。同音語は複数残す。OSの変換器での順位・実機確認は未実施。 |
| [#1016](https://github.com/yksr-melt/Meltype/issues/1016) | [機能提案] Page Up/Downにて候補リストのページ送りを実現する。他 | 部分対応。変換中のPage Up／Downを受理し、9候補ずつ循環移動する。末尾の少数候補・単一ページと隔離IBusを検証。Tabによる格子状一覧は未対応。実アプリ未確認。 |
| [#1015](https://github.com/yksr-melt/Meltype/issues/1015) | [誤判定] `go` -> `語` が変換されない | 補助辞書へ報告の候補を追加し、候補一覧で確認。同音語は複数残す。OSの変換器での順位・実機確認は未実施。 |
| [#1014](https://github.com/yksr-melt/Meltype/issues/1014) | [誤判定] `どしたの` が `土下の` になる | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1013](https://github.com/yksr-melt/Meltype/issues/1013) | [誤判定] `yachi8000` が `やち8000` になる | 今回の回帰テストで既存の共通コアの確定結果を確認。報告アプリでの実機確認は未実施。 |
| [#1011](https://github.com/yksr-melt/Meltype/issues/1011) | [誤判定] ライブ変換中にマッチした短縮よみ（ユーザー辞書）が、続けて入力しても解除されず固定されてしまう | 短縮よみ「へん」の登録後、hennkannkouritugayokunaiまで続けると短縮文が残らないことを共通コアの回帰テストで確認。元の漢字変換器とChromeの実機確認は未実施。 |
| [#1009](https://github.com/yksr-melt/Meltype/issues/1009) | [不具合] Minecraftで、MeltypeIME:英語のまま、移動しようとすると操作がほとんどおかしくなる。 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#1008](https://github.com/yksr-melt/Meltype/issues/1008) | [機能提案] z変換機能のオンオフ | DisableZShortcutsを追加。z記号変換を無効化し、Hz・oz・quiz・jazz・zkai等と編集後の入力を共通コアで検証。設定UIにも表示。既定オフ。報告アプリ実機は未確認。 |
| [#1005](https://github.com/yksr-melt/Meltype/issues/1005) | [誤判定] | 共通コア修正・回帰テスト確認。パーティション／ドキュメントの読みを英語の断片に分けない。漢字・カタカナ変換器の実機確認は未実施。 |
| [#1004](https://github.com/yksr-melt/Meltype/issues/1004) | [誤判定] | 補助辞書へ報告の候補を追加し、候補一覧で確認。同音語は複数残す。OSの変換器での順位・実機確認は未実施。 |
| [#1003](https://github.com/yksr-melt/Meltype/issues/1003) | [誤判定] | 共通コア修正・回帰テスト確認。通常／点数方式で報告された英語の綴りを保つ。報告アプリでの実機確認は未実施。 |
| [#1001](https://github.com/yksr-melt/Meltype/issues/1001) | [誤判定] | 共通コア修正・回帰テスト確認。通常／点数方式で報告された英語の綴りを保つ。報告アプリでの実機確認は未実施。 |
| [#997](https://github.com/yksr-melt/Meltype/issues/997) | [提案] Windows 11のアクセシビリティ設定で、マウスポインターを合わせたウィンドウをアクティブにする機能を有効にしてい… | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#996](https://github.com/yksr-melt/Meltype/issues/996) | [不具合] FINAL FANTASY XIV: ゲーム FINAL FANTASY XIV(ウィンドウモード)外に入力中、変換中の項目がタスクバー側に表示されてしまう。 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#995](https://github.com/yksr-melt/Meltype/issues/995) | [不具合] 旧Outlook: アプリが落ちる | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#994](https://github.com/yksr-melt/Meltype/issues/994) | [不具合] Word、Powerpointなど: スペースが半角で入力される。Shift + Spaceでも半角スペースになる | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#993](https://github.com/yksr-melt/Meltype/issues/993) | [提案] Tabでの変換に慣れておらず、今まではスペースキーまたは下（↓）キーで変換していたため、扱いにくい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#992](https://github.com/yksr-melt/Meltype/issues/992) | [不具合] Edge Web版X: 半角でのみでしか入力できない部分に全角で入力されてしまいX側で入力ができない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#991](https://github.com/yksr-melt/Meltype/issues/991) | [不具合] Word、Powerpointなど: カッコの中に移動せず改行されてしまう（設定の「かっこを打ったら中にカーソルを移動」はオンです） | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#990](https://github.com/yksr-melt/Meltype/issues/990) | [提案] 日本語で確定直後に「・」を入力しようとするとスラッシュになってしまう | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#989](https://github.com/yksr-melt/Meltype/issues/989) | [提案] 「図1」と入力しようとして、「zu+スペース+1」と、Enterせずに数字を入力すると、数字キーが変換に使われてしまい… | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#988](https://github.com/yksr-melt/Meltype/issues/988) | [不具合] WPS OFFICE Spreadsheets 11.2.…: ログの一部です | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#987](https://github.com/yksr-melt/Meltype/issues/987) | [提案] MeltypeをPremiere Pro(動画編集ソフト)で使わせていただいています。 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#985](https://github.com/yksr-melt/Meltype/issues/985) | [不具合] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#984](https://github.com/yksr-melt/Meltype/issues/984) | [不具合] Useage.mdの表示崩れ | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#982](https://github.com/yksr-melt/Meltype/issues/982) | [誤判定] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#981](https://github.com/yksr-melt/Meltype/issues/981) | [誤判定] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#979](https://github.com/yksr-melt/Meltype/issues/979) | [機能提案] カスタムローマ字テーブルを最長一致で処理してほしい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#978](https://github.com/yksr-melt/Meltype/issues/978) | [機能提案] Space で変換したとき、Jevに文節の候補を選ばせる(外部に入力を送る) | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#977](https://github.com/yksr-melt/Meltype/issues/977) | [不具合] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#976](https://github.com/yksr-melt/Meltype/issues/976) | [不具合] macOS spotlight検索バーで入力中に、変換候補が検索バーの裏に表示される | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#973](https://github.com/yksr-melt/Meltype/issues/973) | Windows: Zenzai を ON にすると、同梱の .NET に System.Net.Http.dll が無く、入力が止まる (1.3.1-pre.1) | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#972](https://github.com/yksr-melt/Meltype/issues/972) | [機能提案] 変換中の文字列における部分修正機能の追加 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#971](https://github.com/yksr-melt/Meltype/issues/971) | [誤判定] Windows で打ち間違いの中の 3 文字が英字になり、確定しても直らない (henkasnn → へんkasん、sumimasne → すみmasね) | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#970](https://github.com/yksr-melt/Meltype/issues/970) | [不具合] 自動更新でサインイン直後に、説明なしで Microsoft 名義の UAC (powershell.exe -EncodedCommand) が表示される | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#961](https://github.com/yksr-melt/Meltype/issues/961) | [機能提案] Mozc-UT 辞書を任意で使えるようにする (環境変数 MELTYPE_UT_DICTIONARY) | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#960](https://github.com/yksr-melt/Meltype/issues/960) | [誤判定] "くぉ" と期待してたのが、"k を" になった | 共通コア修正・回帰テスト確認。kwo → くぉ（kwa/kwi/kweも確認）。 |
| [#959](https://github.com/yksr-melt/Meltype/issues/959) | [辞書] ローマ字入力で「merutaipu」と入力して「Meltype」を表示して欲しい、他1件 | めるたいぷ→Meltype-flash／Meltypeを補助辞書へ追加。ばーじょんにRustパッケージの実バージョンを候補追加。共通コアで検証。実機未確認。 |
| [#958](https://github.com/yksr-melt/Meltype/issues/958) | [機能提案] 日本語と半角数字の間にスペースを自動挿入するオプション | SpaceAroundNumbersを追加。確定時に日本語と半角数字の境界へ空白を入れる。小数・桁区切り・既存の空白・識別子・URL・ファイル名・前後の文脈を共通コアで検証。既定オフ。設定UIにも表示。実機未確認。 |
| [#955](https://github.com/yksr-melt/Meltype/issues/955) | [提案] 日本語 / 英数を切り替えるキーにCapsLockがない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#954](https://github.com/yksr-melt/Meltype/issues/954) | [不具合] macos golden gate 27.1.0 26A4…: 表示されるファイルピッカーでどこをクリックしてもピッカーが消えてしまい何も選択できない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#953](https://github.com/yksr-melt/Meltype/issues/953) | [不具合] macos golden gate 27.0.1 26A4…: アイコンが小さく下位置に表示される | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#951](https://github.com/yksr-melt/Meltype/issues/951) | [不具合] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#946](https://github.com/yksr-melt/Meltype/issues/946) | ユーザー辞書のショートカットについて | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#944](https://github.com/yksr-melt/Meltype/issues/944) | [提案] macOS版を「更新通知 → クリックで更新」に対応させたい（Sparkle試作あり） | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#940](https://github.com/yksr-melt/Meltype/issues/940) | [機能提案] TSF でも定型文を利用可能に | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#936](https://github.com/yksr-melt/Meltype/issues/936) | [不具合] ユーザー辞書から語を削除しても誤変換が残る (Linux: 語境界の学習が固定される) | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#935](https://github.com/yksr-melt/Meltype/issues/935) | [辞書] 日付入力の強化 | 部分対応。日付候補生成へ10/20・10月20日の解析と曜日付き形式を追加。「きょう」のスラッシュ形式にも曜日を追加。閏年・不正日付・学習抑止を共通コアで検証。各OSで保護入力から候補を開く実機操作は未確認。 |
| [#925](https://github.com/yksr-melt/Meltype/issues/925) | [機能提案] Lang1 / Lang2 キー (VK_IME_ON / VK_IME_OFF) で日本語 / 英数を決めたい | 部分対応。固定ImeOn/ImeOffの共通コア・C ABIを追加し、冪等性と表示どおりの一度だけの確定を検証。Windows TSF／フックの物理Lang1/Lang2受信への接続は未対応。 |
| [#924](https://github.com/yksr-melt/Meltype/issues/924) | [機能提案] IME ON / IME OFF の個別専用キー割り当て機能（一方向切り替え）の追加 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#920](https://github.com/yksr-melt/Meltype/issues/920) | [誤判定] `カレンダー` が `karenだー` になる | 補助辞書へ報告の候補を追加し、候補一覧で確認。同音語は複数残す。OSの変換器での順位・実機確認は未実施。 |
| [#918](https://github.com/yksr-melt/Meltype/issues/918) | [誤判定] `pure` が `ぷれ` になる | 共通コア修正・回帰テスト確認。通常／点数方式で報告された英語の綴りを保つ。報告アプリでの実機確認は未実施。 |
| [#917](https://github.com/yksr-melt/Meltype/issues/917) | [誤判定] `になった` が `担った` になった | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#915](https://github.com/yksr-melt/Meltype/issues/915) | [誤判定] `printenv` が `p輪転v` になった | 共通コア修正・回帰テスト確認。通常／点数方式で報告された英語の綴りを保つ。報告アプリでの実機確認は未実施。 |
| [#914](https://github.com/yksr-melt/Meltype/issues/914) | [誤判定] `mainにpushする` が `マインにpushする` になった | 共通コア修正・回帰テスト確認。mainnipushsuru → mainにpushする。通常方式を検証。 |
| [#882](https://github.com/yksr-melt/Meltype/issues/882) | [不具合] VS Codeの既存タブで日本語入力ができなくなる（タブ再オープンで復旧） | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#876](https://github.com/yksr-melt/Meltype/issues/876) | [誤判定] `doomscroll` が `どおmscroll` になる | 今回の回帰テストで既存の共通コアの確定結果を確認。報告アプリでの実機確認は未実施。 |
| [#872](https://github.com/yksr-melt/Meltype/issues/872) | [誤判定] 今日久しぶり -> 共久しぶり | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#870](https://github.com/yksr-melt/Meltype/issues/870) | [機能提案] 『「」を角括弧 ［］ にする』有効時、左右の括弧をまとめて1つの単位として変換・確定したい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#863](https://github.com/yksr-melt/Meltype/issues/863) | [提案] apt / brew で管理できるようにする | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#862](https://github.com/yksr-melt/Meltype/issues/862) | [不具合] Windowsのシステムメニュー（スタート・Win+X）で日本語が入力できない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#861](https://github.com/yksr-melt/Meltype/issues/861) | [提案] CapsLock の扱いの改善（直接入力・日本語入力中の挙動） | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#860](https://github.com/yksr-melt/Meltype/issues/860) | [不具合] 中点「・」が入力できない（英字キーボード） | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#859](https://github.com/yksr-melt/Meltype/issues/859) | [提案] 再変換機能（変換キーでの再変換・英字からの変換） | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#856](https://github.com/yksr-melt/Meltype/issues/856) | [機能提案] ユーザー辞書管理(追加/編集)簡易化 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#855](https://github.com/yksr-melt/Meltype/issues/855) | [不具合] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#854](https://github.com/yksr-melt/Meltype/issues/854) | [不具合] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#853](https://github.com/yksr-melt/Meltype/issues/853) | [誤判定] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#852](https://github.com/yksr-melt/Meltype/issues/852) | [誤判定] imageがいまげと表示される | 共通コア修正・回帰テスト確認。通常／点数方式で報告された英語の綴りを保つ。報告アプリでの実機確認は未実施。 |
| [#850](https://github.com/yksr-melt/Meltype/issues/850) | [機能提案] 日本語文章向け・コード向けの記号／スペース出力スタイル切り替え | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#846](https://github.com/yksr-melt/Meltype/issues/846) | [不具合] ”Tier2a"のような独自語をF10で変換/英字にしても学習変換辞書に登録されない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#844](https://github.com/yksr-melt/Meltype/issues/844) | [誤判定] .env | 共通コア修正・回帰テスト確認。.envや先頭が小数点の数値を保持。ホストアプリの実機確認は未実施。 |
| [#838](https://github.com/yksr-melt/Meltype/issues/838) | [提案] 日英判定を、日本語として読んだ場合と比べて決める | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#837](https://github.com/yksr-melt/Meltype/issues/837) | タイトル：Caps Lock ONでも大文字が入力されない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#836](https://github.com/yksr-melt/Meltype/issues/836) | [誤判定] 一度打ち切ったものは再変換できないのかな？と思いますが | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#834](https://github.com/yksr-melt/Meltype/issues/834) | [不具合] Webブラウザで、「あれ」と打ったあと、Shiftで英字入力をし、スペースで英字が消える | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#828](https://github.com/yksr-melt/Meltype/issues/828) | [機能提案] ATOK互換キーマップ対応の要望 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#825](https://github.com/yksr-melt/Meltype/issues/825) | [誤判定] なさそう ー 無點そう | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#823](https://github.com/yksr-melt/Meltype/issues/823) | [誤判定] イショクジョウタイ→い食事ょうたい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#818](https://github.com/yksr-melt/Meltype/issues/818) | [不具合] エクスプローラーの一覧で文字キーが横取りされ、先頭一致のファイル選択が効かない (Meltype IME) | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#817](https://github.com/yksr-melt/Meltype/issues/817) | [不具合] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#816](https://github.com/yksr-melt/Meltype/issues/816) | [機能提案] 本当の直接入力：キーを1打ずつアプリへ渡し、シェルやブラウザの対話的な補完を活かす | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#773](https://github.com/yksr-melt/Meltype/issues/773) | [不具合] Powerpointコメント入力時の変換ボックス位置の不良 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#772](https://github.com/yksr-melt/Meltype/issues/772) | [不具合] ウイルス対策ソフトに検知され利用できない | 未解決。ウイルス対策／OSの信頼判定を含む。対象環境、コード署名、検知元での確認が必要。 |
| [#764](https://github.com/yksr-melt/Meltype/issues/764) | [機能提案] 英単語を辞書登録したい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#734](https://github.com/yksr-melt/Meltype/issues/734) | [不具合] ブラウザで入力した1文字目がキーと異なる文字で入力される | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#729](https://github.com/yksr-melt/Meltype/issues/729) | [不具合] タッチキーボードで文字を入力するとき、meltypeが適用されない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#728](https://github.com/yksr-melt/Meltype/issues/728) | [不具合] microsoft copilotのチャット入力欄、clip studio paintのレイヤー名変更、microsoft journalのジャーナルの名前変更などの部分で、meltypeが適用されない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#723](https://github.com/yksr-melt/Meltype/issues/723) | [誤判定] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#721](https://github.com/yksr-melt/Meltype/issues/721) | [不具合] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#714](https://github.com/yksr-melt/Meltype/issues/714) | [誤判定] catとdog派遣化する | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#705](https://github.com/yksr-melt/Meltype/issues/705) | [誤判定] rate limit | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#702](https://github.com/yksr-melt/Meltype/issues/702) | [不具合]ChromeとEdgeで動作しない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#693](https://github.com/yksr-melt/Meltype/issues/693) | [不具合] 不具合再現CIが誤って表示されている | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#688](https://github.com/yksr-melt/Meltype/issues/688) | [不具合] プレリリース版を使用中、GitHubへの不具合報告時にバージョンが安定版（古い方）に固定・上書きされてしまう | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#668](https://github.com/yksr-melt/Meltype/issues/668) | 同梱の .NET を 10.0.13 に更新する (リリースし直す) | 旧C#版の同梱.NET更新。Rust本体は.NETを同梱しない。比較用C#資産の依存は別途確認が必要。 |
| [#667](https://github.com/yksr-melt/Meltype/issues/667) | [不具合]英字キーボード利用時の左右Altを利用時の変換機能 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#664](https://github.com/yksr-melt/Meltype/issues/664) | [機能提案] 変換ボックスや文字の色の種類を増やす | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#663](https://github.com/yksr-melt/Meltype/issues/663) | [不具合] NortonがIME登録処理をふるまい検知し、Meltype IMEが使用できない | 未解決。ウイルス対策／OSの信頼判定を含む。対象環境、コード署名、検知元での確認が必要。 |
| [#661](https://github.com/yksr-melt/Meltype/issues/661) | [不具合] Windowsメニュー入力時に変換ボックスがメニュー背後に隠れる | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#660](https://github.com/yksr-melt/Meltype/issues/660) | [不具合] 変換ボックスが極小になるケースがあり視認性が悪い | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#659](https://github.com/yksr-melt/Meltype/issues/659) | [不具合] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#655](https://github.com/yksr-melt/Meltype/issues/655) | [不具合] Windows起動後、動作が安定しない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#650](https://github.com/yksr-melt/Meltype/issues/650) | [機能提案] Scoop でバケットの公開をしてほしい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#644](https://github.com/yksr-melt/Meltype/issues/644) | [機能提案] Mac: 確定した変換を azooKey に学習させる (Linux 版の Mozc と同じく) | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#641](https://github.com/yksr-melt/Meltype/issues/641) | [誤判定] 改行後の英字の挙動 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#639](https://github.com/yksr-melt/Meltype/issues/639) | [不具合] Alt単独押しでの日英切り替え時、アプリのメニューへフォーカスが移る | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#638](https://github.com/yksr-melt/Meltype/issues/638) | [不具合] 中黒(・)が打てない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#637](https://github.com/yksr-melt/Meltype/issues/637) | [誤判定] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#634](https://github.com/yksr-melt/Meltype/issues/634) | [不具合] MinecraftのMod, EMIの検索欄で変換が機能しない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#627](https://github.com/yksr-melt/Meltype/issues/627) | [機能提案]Meltype IME（入力欄に直接入力)における変換無変換キーでの日本語英語切り替え | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#618](https://github.com/yksr-melt/Meltype/issues/618) | [不具合] Welcome UIでスクロールすると入力欄が残る | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#614](https://github.com/yksr-melt/Meltype/issues/614) | [不具合] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#611](https://github.com/yksr-melt/Meltype/issues/611) | [機能提案] プレリリース版の使用時に「今すぐ更新を確認」を実行した際、プレリリースバージョンを表示してほしい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#610](https://github.com/yksr-melt/Meltype/issues/610) | [不具合] shift+←→で分節を伸縮するときの動作について | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#606](https://github.com/yksr-melt/Meltype/issues/606) | [不具合] プロファイル名の変更画面で直接Meltype入力ができない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#601](https://github.com/yksr-melt/Meltype/issues/601) | GitHubのバージョンと本体のバージョンが違う場合に発生するバグについて | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#598](https://github.com/yksr-melt/Meltype/issues/598) | リファクタリングをする | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#546](https://github.com/yksr-melt/Meltype/issues/546) | [誤判定] 別にそれ(英単語)でもいい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#544](https://github.com/yksr-melt/Meltype/issues/544) | [誤判定] めんどくせえな | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#543](https://github.com/yksr-melt/Meltype/issues/543) | [誤判定] けdomala | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#539](https://github.com/yksr-melt/Meltype/issues/539) | [不具合] VRChat で Meltype IME が読み込まれず、日本語入力できない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#523](https://github.com/yksr-melt/Meltype/issues/523) | [機能提案] MacOS 入力プレビュー付きのGUI設定画面を追加する | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#519](https://github.com/yksr-melt/Meltype/issues/519) | [誤判定] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#499](https://github.com/yksr-melt/Meltype/issues/499) | [誤判定] SBI新生銀行 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#498](https://github.com/yksr-melt/Meltype/issues/498) | [誤判定] 日本語入力で3字以上の時「，．」が半角になる | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#496](https://github.com/yksr-melt/Meltype/issues/496) | [機能提案] ローマ字テーブルを編集して独自の入力変換を設定したい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#494](https://github.com/yksr-melt/Meltype/issues/494) | [不具合] キーボードモードで変換ボックスが不自然な位置に表示される | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#489](https://github.com/yksr-melt/Meltype/issues/489) | [不具合] AlmaLinux 9.8でキーボード・入力ソース・Meltypeの設定が動作しない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#487](https://github.com/yksr-melt/Meltype/issues/487) | [機能提案] ユーザー辞書に品詞情報の登録をできるようにする | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#470](https://github.com/yksr-melt/Meltype/issues/470) | [不具合]アップデートを行ったら、他のIMEが登場しなくなってしまった | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#463](https://github.com/yksr-melt/Meltype/issues/463) | [不具合] v1.2.1で定型文がSpace / Tabで展開されない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#460](https://github.com/yksr-melt/Meltype/issues/460) | [機能提案] テンキー入力の際の直接入力機能 | 部分対応。NumpadDirectを追加。Windows仮想キーの数字・演算記号を既存の文の確定後に直接確定する共通コアを検証。設定UIにも表示。Linux等のテンキー識別接続とExcel・CAD実機確認は未対応。 |
| [#442](https://github.com/yksr-melt/Meltype/issues/442) | Google スプレッドシートで認識されない浮動小数点数 | 共通コア修正・回帰テスト確認。.envや先頭が小数点の数値を保持。ホストアプリの実機確認は未実施。 |
| [#438](https://github.com/yksr-melt/Meltype/issues/438) | [不具合] 日本語での入力ができなくなり、すべてアルファベットになってします | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#436](https://github.com/yksr-melt/Meltype/issues/436) | [不具合] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#432](https://github.com/yksr-melt/Meltype/issues/432) | [機能提案] MacOSでの入力モード切り替えをキーボードショートカットに対応 プルリク可能 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#430](https://github.com/yksr-melt/Meltype/issues/430) | [不具合] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#420](https://github.com/yksr-melt/Meltype/issues/420) | [不具合] YouTubeの検索欄で動作しない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#412](https://github.com/yksr-melt/Meltype/issues/412) | [誤判定] `ヴィべコーディング` になり `vibeコーディング` にならない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#397](https://github.com/yksr-melt/Meltype/issues/397) | [不具合][Linux・改造版で発生] Ctrl+V後の文字入力で貼り付けた文章が消えることがある | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#393](https://github.com/yksr-melt/Meltype/issues/393) | [不具合] | 未解決。ウイルス対策／OSの信頼判定を含む。対象環境、コード署名、検知元での確認が必要。 |
| [#391](https://github.com/yksr-melt/Meltype/issues/391) | [機能提案] Melttype IMEにもキーカスタマイズを | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#365](https://github.com/yksr-melt/Meltype/issues/365) | [誤判定] VRChatが「Vアールチャット」と変換される | 補助辞書へ報告の候補を追加し、候補一覧で確認。同音語は複数残す。OSの変換器での順位・実機確認は未実施。 |
| [#354](https://github.com/yksr-melt/Meltype/issues/354) | [不具合] IME設定欄が勝手に出現する | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#348](https://github.com/yksr-melt/Meltype/issues/348) | [誤判定] 「Mainのコードで確認」が「マインおコードで書くにん」になる | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#347](https://github.com/yksr-melt/Meltype/issues/347) | [誤判定] deepseek が「でえpseek」になる | 今回の回帰テストで既存の共通コアの確定結果を確認。報告アプリでの実機確認は未実施。 |
| [#345](https://github.com/yksr-melt/Meltype/issues/345) | [機能提案] 部首・構成要素の説明から漢字を検索して変換候補に出す | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#343](https://github.com/yksr-melt/Meltype/issues/343) | [機能提案] 郵便番号から住所を変換候補に出す | 未対応。換算レート／郵便番号の外部データ取得、更新方法、オフライン時の挙動の実装が必要。 |
| [#342](https://github.com/yksr-melt/Meltype/issues/342) | [機能提案] 変換で通貨換算を行う | 未対応。換算レート／郵便番号の外部データ取得、更新方法、オフライン時の挙動の実装が必要。 |
| [#316](https://github.com/yksr-melt/Meltype/issues/316) | [誤判定] 英単語 Claude の直後に空白なしで日本語を続けると「Cぁうっえ」になる | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#313](https://github.com/yksr-melt/Meltype/issues/313) | [機能提案] TSF モードでも、確定した文字を後から選択して再変換したい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#298](https://github.com/yksr-melt/Meltype/issues/298) | [誤判定] シリーズが知り-図になる | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#296](https://github.com/yksr-melt/Meltype/issues/296) | [誤判定]issuesが意思ゅーに | 補助辞書へ報告の候補を追加し、候補一覧で確認。同音語は複数残す。OSの変換器での順位・実機確認は未実施。 |
| [#287](https://github.com/yksr-melt/Meltype/issues/287) | [機能提案] 入力欄の外 (スレッド・ページ) の言語を判定の手がかりにする | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#264](https://github.com/yksr-melt/Meltype/issues/264) | [不具合] McAfee LiveSafe によって Meltype が脅威と見なされてしまい隔離されてしまいます | 未解決。ウイルス対策／OSの信頼判定を含む。対象環境、コード署名、検知元での確認が必要。 |
| [#261](https://github.com/yksr-melt/Meltype/issues/261) | [不具合] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#257](https://github.com/yksr-melt/Meltype/issues/257) | [不具合] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#253](https://github.com/yksr-melt/Meltype/issues/253) | [誤判定] URLが虚に | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#240](https://github.com/yksr-melt/Meltype/issues/240) | [機能提案] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#236](https://github.com/yksr-melt/Meltype/issues/236) | 半角カナの入力対応 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#234](https://github.com/yksr-melt/Meltype/issues/234) | [機能提案] Z + H / J / K / L キーでの矢印入力機能の追加 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#229](https://github.com/yksr-melt/Meltype/issues/229) | [提案] Mac 版が正式版になったら Homebrew (cask) で配布したい | 正式版後のHomebrew配布提案。現在はPre5。署名・公証と正式配布の準備は未完了。 |
| [#209](https://github.com/yksr-melt/Meltype/issues/209) | [機能提案] キー選択機能と対応アプリの拡充 | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#197](https://github.com/yksr-melt/Meltype/issues/197) | [辞書] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#162](https://github.com/yksr-melt/Meltype/issues/162) | [不具合] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#157](https://github.com/yksr-melt/Meltype/issues/157) | [不具合] Excel関数PHONETICで変換されない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#155](https://github.com/yksr-melt/Meltype/issues/155) | [不具合] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#145](https://github.com/yksr-melt/Meltype/issues/145) | [誤判定] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#135](https://github.com/yksr-melt/Meltype/issues/135) | [誤判定] かな入力で Shift を押して打った文字が英字のまま残る | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#128](https://github.com/yksr-melt/Meltype/issues/128) | [不具合] Google Workspaceで変換候補が出ない・表示位置が安定しない | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#126](https://github.com/yksr-melt/Meltype/issues/126) | [機能提案] 無変換キーで一部を英字に固定して混在入力したい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#117](https://github.com/yksr-melt/Meltype/issues/117) | [不具合] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#101](https://github.com/yksr-melt/Meltype/issues/101) | [不具合] | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#84](https://github.com/yksr-melt/Meltype/issues/84) | [紹介] Google日本語入力の辞書・学習履歴を使う非公式TSF版を公開しました | 別の非公式TSF版の紹介。本文に新たな修正依頼はない。紹介として記録。 |
| [#73](https://github.com/yksr-melt/Meltype/issues/73) | [提案] 辞書への語の追加を簡単にしたい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#68](https://github.com/yksr-melt/Meltype/issues/68) | [提案] カタカナから英語の綴りへ変換できるようにしたい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#67](https://github.com/yksr-melt/Meltype/issues/67) | [提案] かな入力のオン・オフ設定を追加し、変換精度を高めたい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#66](https://github.com/yksr-melt/Meltype/issues/66) | [提案] 旧JISかな入力でWindowsのレイアウトDLLをどう扱うか明確にしてほしい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#64](https://github.com/yksr-melt/Meltype/issues/64) | [提案] ローマ字以外の入力方式（親指シフト・かな入力）の対応状況を知りたい | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#39](https://github.com/yksr-melt/Meltype/issues/39) | カスタマイズ機能の拡充を行う | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |
| [#34](https://github.com/yksr-melt/Meltype/issues/34) | 誤変換テスト用 | 誤変換テスト用の投稿先。コメントも取得済みだが、全コメントの再現・照合は未完了。 |
| [#28](https://github.com/yksr-melt/Meltype/issues/28) | [不具合] Avast Secure Browserだと入力できない | 未解決。ウイルス対策／OSの信頼判定を含む。対象環境、コード署名、検知元での確認が必要。 |
| [#25](https://github.com/yksr-melt/Meltype/issues/25) | [不具合] googledocsdeno入力について | 未検証。本文・コメントはローカルの取得記録に保存。再現・修正・対象OSでの検証が必要。 |

## 検証と残作業

再現可能な入力例は `rust/core/tests/reported_issues.rs` で、通常・点数方式の判定、空白を含む確定結果、既存の日本語入力を壊さないことを検査します。`rust/core/examples/upstream_issue_audit.rs` は取得した入力例JSONを共通コアへ通し、読み・確定結果を出力します。

共通コア: 最終の全体テスト210件成功。既存の手動性能試験など2件は未実施。Windowsネイティブ: 単体55件成功。切替C ABIはWindowsとLinuxで各7件成功。Windows TSFキー写像・受理検査18件成功。IBusキー写像2件、Python側の写像・切替ルーティング16件も成功。PAL準備スクリプト9件成功。

Linux（WSL）ではRust IBus・ネイティブ共有ライブラリ・fcitx5 SDKアドオンをビルドし、Xvfb／D-Busを隔離した実IBus環境で追加3件を含む23件成功（2回確認）。最初の実行では既存の直接入力自動判定テストが1件失敗し、再実行2回で再現しませんでした。原因の特定・解消とは扱いません。fcitxの101イベント列と追加の候補・フォーカス等のABI検査も成功。Windows／LinuxのClippyは警告をエラー扱いで成功し、filesystemなしの共通コア検査も成功。通常のデスクトップや報告アプリでの実機確認とは区別します。

| OS | 今回のビルド確認 | 実機・報告アプリでの確認 |
| --- | --- | --- |
| Windows | 共通コア・ネイティブTSF DLL・設定アプリ・CLI成功。インストーラー全体は未実施 | 未実施（DLLのCOM・キー受理検査とは区別） |
| macOS | 未実施 | 未実施 |
| Linux | WSL上のRust IBus・native共有ライブラリ・fcitx5アドオン成功。設定UI・配布パッケージ全体は未実施 | 通常のデスクトップ・報告アプリは未実施。隔離IBusの23件は再検査成功、初回の1件失敗は上記参照 |
| Android | 未実施 | 未実施 |
| ChromeOS | 未実施 | 未実施 |
| iOS | 未実施 | 未実施 |
