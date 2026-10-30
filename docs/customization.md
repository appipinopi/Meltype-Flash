# カスタマイズの進め方

2026年10月23日に確認した上流の低優先Issueをもとに、まず日常的な設定を画面から操作できるようにする。変換処理へ新しい依存関係は追加せず、保存済みの設定を共通Coreへ渡す。

## 今回の変更

ChromeOSの設定画面へ、日英判定・句読点・タイプミス補正・Tab変換・英語前後の空白・学習の6項目を追加した。これまでの詳細JSONも残し、双方で同じ編集内容を扱う。保存後はCoreから設定を読み直し、正規化された値を表示する。非表示のアプリ別設定と未知の設定項目は保持する。

Android・iOSには、システム・ライト・ダーク・ブルー・ピンク・ミントの6種類のテーマと背景画像の選択・削除・プレビューを追加した。キーと候補の文字・背景色もテーマに合わせる。画像は選択時に長辺1280ピクセル以下へ縮小し、中央を基準に表示する。画像の再読込みは表示開始・設定変更時だけで、キー入力時やレイアウト変更ごとには行わない。

AndroidはフリックとQWERTYに同じ配色を使う。背景は専用の設定画面から変更し、Coreの辞書・学習・バックアップとは分けて保存する。iOSは本体アプリがApp Groupへ保存し、キーボードは読み取るだけとする。フルアクセスは要求しない。iOSでの共有設定と署名の手順は [iOS README](../ios/README.md) に記載した。

Androidの背景設定・画像の縮小・EXIF回転・保存・キャッシュ・中央基準の描画はRustへ移した。Android APIはJNI経由で呼び、OS側のクラス・リスナーはRustの定義から生成する。フリック判定・入力欄の開始終了・設定画面の通知処理もRustで管理する。Android本体に手書きのJava/Kotlinソースは残していない。

これは [#39 カスタマイズ機能](https://github.com/yksr-melt/Meltype/issues/39)・[#664 配色](https://github.com/yksr-melt/Meltype/issues/664) の段階的な改善であり、Issue全体の完了ではない。ChromeOSの候補ウィンドウの色・フォント・位置・非表示、任意のキーバインド、候補順の固定は今回追加していない。

## 次に追加する設定の案

| 分類 | 操作 | 進め方 |
| --- | --- | --- |
| 入力操作 | 変換・次候補・前候補・確定キー | まず既定の操作セットを選択できる形にし、その後に個別編集を追加する。重複キーを保存前に検出する |
| 候補表示 | 入力位置の上・下、表示する候補数、予測候補の表示 | 入力欄に重なる [#240](https://github.com/yksr-melt/Meltype/issues/240) を優先する。候補を隠しても変換中の状態は確認できるようにする |
| 見た目 | 文字サイズ、自由な配色、デスクトップへの展開 | モバイルのテーマ・画像対応に続いて [#664](https://github.com/yksr-melt/Meltype/issues/664) を進める。デスクトップは候補ウィンドウ、モバイルはキーの大きさと文字サイズを中心に調整する |
| 辞書・候補順 | 読みと単語の登録、登録語の優先 | [#73](https://github.com/yksr-melt/Meltype/issues/73) の登録手順を短くする。学習順と固定した優先順位は別の操作として示す |

各項目に「保存済みへ戻す」を用意する。操作の初期化、見た目の初期化、学習の削除を一緒にしない。プレビューは設定画面を開いたときだけ動かし、入力中の処理に負荷を加えない。

## OSごとの適用

設定名があることと、各入力方式がその設定を表示へ反映することは区別する。C#版には変換ボックスのテーマ・位置などの設定があるが、Rust版へ同じ設定名を保存するだけでは表示の対応にならない。RustのWindowsキーボード方式も、簡易候補表示で未対応の表示設定を案内している。

ChromeOS版は標準の候補ウィンドウを使う。現状の実装では独自の色・フォント・位置設定を反映していないため、動かない設定を通常の画面へ追加しない。Windows・macOS・Linux・Android・ChromeOS・iOSそれぞれの表示実装で反映を確認してから有効にする。

## 低優先Issueの残り

今回確認した未解決Issueは12件。上の4件以外は [#1103 helpの権限表記](https://github.com/yksr-melt/Meltype/issues/1103)、[#688 報告時の版情報](https://github.com/yksr-melt/Meltype/issues/688)、[#953 Macアイコン](https://github.com/yksr-melt/Meltype/issues/953)、[#863 apt/brew](https://github.com/yksr-melt/Meltype/issues/863)、[#655 Windows起動直後](https://github.com/yksr-melt/Meltype/issues/655)、[#393 Smart App Control](https://github.com/yksr-melt/Meltype/issues/393)、[#264 McAfee](https://github.com/yksr-melt/Meltype/issues/264)、[#145 discodeの判定](https://github.com/yksr-melt/Meltype/issues/145)。今回の変更でこれらが解決したとは扱わない。
