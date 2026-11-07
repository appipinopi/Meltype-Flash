# AIモデルの任意導入

|OS|選択肢|
|---|---|
|Windows・macOS・Linux|使用しない / Zenz / Jinen / kanakanji-edge / shuntelligence|
|Android・ChromeOS・iOS|使用しない / shuntelligence|

## バージョンとモデルサイズの選択

モデル系列を選び、「バージョン・モデルサイズ・量子化」で使う配布版を選びます。モデルはそれぞれ別のフォルダーへ導入するため、切り替えても以前のモデルを保持します。導入と検証が終わるとボタンが「インストール済み」に変わります。設定を閉じて開き直した場合も導入状態を読み取り、再ダウンロードせず選択できます。バージョンの選択だけでは未導入のモデルを起動しません。

|系列|選択できる配布版|対象OS|
|---|---|---|
|Zenz|v1 Q8_0、v2 Q5_K_M、v3 small Q5_K_M、v3.1/v3.2 small・xsmall Q5_K_M|Windows・macOS・Linux|
|Jinen|v1 small・xsmall Q5_K_M/F16、v1.1 beta・v2 small・xsmall Q4_K_M/Q5_K_M/Q8_0/F16|Windows・macOS・Linux|
|kanakanji-edge|33m Q8_0/F16|Windows・macOS・Linux|
|shuntelligence|v5 S7/XS5、v6 S8P/XS5、v7 S8P/XS5|Windows・macOS・Linux・Android・ChromeOS・iOS|

Jinen v1.1 betaは配布元のモデルカードにライセンス表記がないため、導入画面でもその旨と配布元リンクを示します。重みは他のモデルと同様、本体に同梱せず配布元から任意取得します。Zenz v2.5などGGUFを配布していないモデルの自動変換や、未確認の実行コードの導入は行いません。

配布元・固定リビジョン・SHA-256・サイズ・ライセンスは [`ai/model-catalog.json`](ai/model-catalog.json) にまとめています。Zenz v1/v2/v3の制御文字の順序を区別し、JinenはNFKC正規化、kanakanji-edgeは特殊トークンの解析に対応します。GGUF実行器のトークナイザー互換指定はZenzにだけ適用します。

モデルの重みやトークナイザーのライセンスはアプリ本体のGPLv3とは別です。Zenzは各配布元のCC BY-SA 4.0またはApache-2.0、Jinen v1/v2はCC BY-SA 4.0、kanakanji-edgeの重みはApache-2.0・埋め込みトークナイザーは京都大学黒橋研究室由来のCC BY-SA 4.0です。作者はそれぞれKeita Miwa、Hitoshi Togasaki、yuuki14202028です。Jinen v2は国立国会図書館の書誌データを加工したコーパスを含み、国立国会図書館がモデルを提供・保証するものではありません。shuntelligenceのデータ出典と条件は後述の文書を参照してください。

配布元: [Zenz](https://huggingface.co/Miwa-Keita)、[Jinen](https://huggingface.co/togatogah)、[kanakanji-edge](https://huggingface.co/yuuki14202028/kanakanji-edge-33m)、[shuntelligence](https://github.com/shuntilettuce/Japanese-IME-for-HarmonyOS-next/releases)。配布形式やライセンスが変わった場合は、確認した配布版を一覧へ追加します。

最初はAIを使用しません。モデルを選ぶと、そのモデルの導入・パス設定だけを表示します。「インストール」を押して環境作成に同意すると、専用ランタイムとモデルデータを取得します。OllamaやDockerは導入しません。導入済みの環境は再利用します。「使用しない」へ戻してもモデルやパスを削除しません。

デスクトップでは自分で取得した実行ファイルやモデルのパスも選べます。モバイル・ChromeOSではアプリが管理する専用領域を使い、パス入力は不要です。ChromeOSは導入時だけダウンロード先へのアクセスと保存容量の権限を要求します。iOSはApp Groupを含む署名と、共有モデルを読むためのフルアクセス許可が必要です。通常のキーボード入力はフルアクセスを許可せず利用できます。

モデルは本体の配布物に含みません。導入時に固定したSHA-256を検証し、失敗した途中の環境は有効にしません。shuntiの実行コードは固定した上流コミットからビルドし、Android・iOS・ChromeOSではアプリへ同梱します。ChromeOSで外部の実行コードは取得しません。

入力内容は外部へ送信しません。秘密入力・英字・数値の変換はAIへ渡さず、入力内容が変わった後の応答は適用しません。AIが準備中・未導入・停止中の場合や応答の待機期限を超えた場合は通常の変換を使います。

shunti: https://github.com/shuntilettuce/Japanese-IME-for-HarmonyOS-next/tree/14801bffb90c9ea5f2dd0f594943a97ea35c8a1a 。エンジンのMIT表記は本体の第三者ライセンスに含みます。任意データの辞書・モデルには配布元の条件（CC BY-SA 4.0・CC BY 4.0など）が適用されます。詳細は `ai/vendor/shunti/THIRD_PARTY_NOTICES.md` と `DATA_SOURCES.md` に記録しています。

## 上流1.4.1系の取り込み

照合先は `yksr-melt/Meltype` の `v1.4.1-pre.5`（`7813249178fcf71a03b4c11d38211007aa58939f`）です。正式な1.4.1と同一の機能を保証する表記ではありません。

- PR #1505のZenz文脈混入修正：プロンプトの区切り順を修正し、出力区切り8種類と正当な変換結果を回帰テストで確認。
- 1.4.0以降の辞書変更を既存のFlash独自語彙と照合して統合。候補・英単語・ブランド・読みなどを対象とし、辞書形式検査を実施。
- 既存の公開Issue回帰テストを維持。キー数集計・Discord表示・新かな配列など、独立した大きな機能の互換実装は今回のAI導入変更には含めません。

ビルド成功と物理端末での動作確認は分けてリリース本文へ記載します。
