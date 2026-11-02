# Issueの報告先と確認範囲

[Meltype-flashのIssue](https://github.com/appipinopi/Meltype-Flash/issues)では、Rust版のWindows・macOS・Linux・Android・ChromeOS・iOSを扱います。[本家MeltypeのIssue](https://github.com/yksr-melt/Meltype/issues)では、本家のC#版と、その公式サイト・botなどを扱います。

| 起きている場所 | 報告先・扱い |
| --- | --- |
| Meltype-flashで起きる | [Flashの報告フォーム](https://github.com/appipinopi/Meltype-Flash/issues/new/choose)で、OS・Flashの版・アプリ・入力と結果を書いてください。 |
| Flashと本家の両方で起きる | Flash側でも受け付けます。両方の版と再現結果、既存の本家IssueがあればそのURLを書いてください。 |
| 本家のC#版だけの画面・機能、公式サイト・botの問題 | [本家の報告フォーム](https://github.com/yksr-melt/Meltype/issues/new/choose)へ報告してください。Flash側で本家のIssueを完了にはしません。 |
| どちらか分からない | Flashのフォームで「分からない」を選び、使っている実行ファイル名やダウンロード元を添えてください。確認して分類します。 |

本家だけの報告かどうかは、再現した製品・版・機能を確認して判断します。Flashで未対応の機能や未検証の問題を、理由なく「本家専用」に分類しません。

## 本家の報告を参照するとき

- 元のリポジトリとIssue URLを記録し、Flashで確認する対象OS・機能・入力例を分けます。
- [上流Issueの対応記録](ISSUE-AUDIT.md)は、取得時点の本家Issueを参照した台帳です。Flashの未解決Issue一覧や、Flash 1.0の完了件数として数えません。
- 本家の状態とFlashの状態は別々に記録します。共通コアのテスト成功だけで、本家アプリや報告アプリで直ったとは扱いません。
- 再現できていない報告は「未検証」、一部だけ直した場合は「部分対応」とし、未確認のOS・実機操作を明記します。
- 修正を確認したときも、終了するのは確認できたFlash側のIssueです。本家Issueの終了・移動・転載は自動で行いません。本家側の報告は元の窓口で扱います。

Flash側の修正を本家にも提案する場合は、元Issueへの参照と確認範囲を添えて、別途本家へPull Requestを送ります。

## 自動再現・公開コマンド

本家から継承したbotの`/repro`・`/explain`・`/ca`・`/jht`や、`/test`・`/pack`・`/release`等はC#版用です。FlashではこれらをC#版へ送らず、Rust版の[全OS検証](https://github.com/appipinopi/Meltype-Flash/actions/workflows/rust.yml)と[公開用ワークフロー](https://github.com/appipinopi/Meltype-Flash/actions/workflows/rust-release.yml)を案内します。

Flashの誤判定フォームからC#版の自動再現は起動せず、C#版の結果に基づく「再現済み」ラベルも付けません。Flashの再現確認はRustコードで行い、PRの「確かめること」にある入力例や対象OSの検査結果を記録します。本家リポジトリでのC#版botの動作は維持します。
