# AGENTS.md

AI エージェント (Claude Code・Codex など) が Meltype で作業するときの決まりです。
詳しい開発の手順は [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)、貢献の決まりは [CONTRIBUTING.md](CONTRIBUTING.md) にあります。

## 構成

| 場所 | 中身 |
| --- | --- |
| `src/Meltype.Core` | OS によらない判定・変換のコア (Composition / Detection / Input / Config / Diagnostics)。判定・変換を直すときはまずここ |
| `src/Meltype.Core.Tests` | コアのテスト (どの OS でも動く) |
| `src/Meltype` | Windows 版 (TSF・UI・更新) |
| `src/Meltype.Tests` | Windows 版のテスト (Windows でしか動かない) |
| `src/Meltype.Mac.Native` | Mac 版から呼ぶコアの出口 |
| `native/tip`・`native/mozc` | ネイティブの部分 (TSF の TIP・Mozc) |
| `mac/` | Mac 版 (Swift) |
| `linux/` | Linux 版 (fcitx5・ibus) とパッケージの作成 |
| `web/` | Web (Cloudflare Workers・報告フォーム) |
| `dictionaries/` | 辞書 (英単語・固有名詞・同音異義語の候補・文脈の手がかり) |
| `tools/` | 辞書の生成・検査、ベンチマーク、テスト用のスクリプト |
| `docs/` | 使い方・開発・設計の文書 |

## ビルドとテスト

```
dotnet build Meltype.sln
dotnet run --project src/Meltype.Core.Tests                 # すべてのテスト (Linux / Mac でもこれ)
dotnet run --project src/Meltype.Core.Tests -- --eval       # 品質テスト
dotnet run --project src/Meltype.Core.Tests -- --repro kyouhagoogle enter   # 打ったときの見え方と確定の結果
dotnet run --project src/Meltype.Tests                      # すべてのテスト (Windows だけ)
dotnet run --project src/Meltype.Tests -- CompositionTests  # 名前に一致するテストだけ
dotnet run --project src/Meltype.Tests -- --explain konnichiwa hello   # 1 文字ずつの判定理由
dotnet run --project src/Meltype.Tests -- --convert きょうはいいてんき  # 変換の結果と文節の区切り
node tools/check-dictionaries.mjs                           # 辞書を変えたとき
```

Linux のクラウド環境では `src/Meltype.Tests` は動きません。`src/Meltype.Core.Tests` で確かめてください。

## 必ず守ること

- **新しいソースファイルの先頭** にライセンスと著作権の行を入れる。名前は既存のファイルをまねて `Yukishiro` にせず、変更を頼んだ人 (PR を出す人) の名前にする。分からなければ聞く。
  ```
  // SPDX-License-Identifier: GPL-3.0-or-later
  // Copyright (C) 2026 あなたの名前
  ```
- **PR は [テンプレート](.github/PULL_REQUEST_TEMPLATE.md) どおりに書く**。「確かめること」は `入力 → 期待` の形で書く (bot が PR のコードで再現を確かめます)。
- **AI を使ったことを PR に書く**。テンプレートの「AI を使った」にチェックを入れ、使った部分を書く。
- **判定・変換を変えたら**、報告された例をテストに足す。
- **説明は日本語で書く**。PR の本文・コメント・文書は、既存の文書に合わせて平易な日本語で書く。
- **ビルドを報告するときは全OSを書く**。Windows・macOS・Linux・Android・ChromeOS・iOSを毎回明記し、それぞれビルド成功・失敗・未実施を区別する。ビルドと実機での動作確認も区別し、未実施を成功として扱わない。
- **ビルドの配布時は全OS用のファイルを添付する**。Windows・macOS・Linux・Android・ChromeOS・iOSの配布物を揃えて添付する。状況表だけで代用せず、不足したまま全OS版として配布しない。
- **リリースの添付を整理する**。同一内容の重複ZIP、BUILD-PROVENANCE.json、単独のRELEASE-NOTES.md・README-iOS.mdは添付しない。説明はリリース本文やソース内の文書へまとめ、各OSの配布物・ライセンス・SHA256SUMS.txtを残す。
- **画像をダウンロード用アセットとして配布しない**。スクリーンショット・バナーなどの画像をリリースのAssetsや配布アーカイブへ追加しない。ベンチマークの箱ひげ図はリリース本文への画像埋め込みで表示する。検証用の画像はローカルに保持する。
- **ベンチマーク結果はリリース本文に掲載する**。結果・比較条件・測定範囲を本文の表・文章・箱ひげ図で示す。手動時はGitHubの本文画像、自動リリース時は画像専用のrelease-mediaブランチのコミットSHA付きURLを本文へ埋め込む。リリースAssetsへの画像・レポート・生データ・ZIPの添付や、それらへのダウンロードリンクは追加しない。自動処理の受け渡し用CI成果物は短期間で削除し、公開リリースへ添付しない。
- **変更はできるだけ小さくする**。頼まれたことに必要な分だけ直し、ついでの整理や書き換えはしない。

## してはいけないこと

- テストを消す・飛ばす・ゆるめることで通す。
- 生成されたファイル (辞書から作るもの、`tools/make-*` で作るものなど) を手で直す。生成のスクリプトを使う。
- 頼まれていないのに、リリース・Homebrew cask・winget などの配布まわりのワークフロー (`.github/workflows`・`Casks/`・`packaging/`) を変える。
