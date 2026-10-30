# CIの入口

普段使う入口は次の4つです。OS別の手順は全OSビルドのジョブにまとめています。

|入口|定義|実行すること|
|---|---|---|
|PRチェック|`pr-checks.yml`|辞書形式、C#品質比較、報告の再現、Flash固有IDの保護|
|全OSビルド・テスト|`rust.yml`|Windows・macOS・LinuxのCore/Native/設定画面、Android JNI/APK/エミュレーター、ChromeOS WASM、fcitx ABI、macOS SDK/Swift、iOSシミュレーター|
|ベンチマーク|`platform-benchmarks.yml`|手動でWindows・macOS・LinuxのCoreとNative C ABIを比較|
|リリース|`rust-release.yml`|明示したタグに対する全OS配布物の作成と公開。公開は手動操作のみ|

このほか、`bot.yml`（リポジトリ管理・CLA・著作権表記）、`web.yml`、`sponsors.yml`を独立して残しています。実行可能なワークフローは計7本です。移動前の実行履歴は消していないため、GitHubの一覧にはしばらく古い名前も残る場合があります。

## 全OSの検証

PRと手動操作で`rust.yml`を実行します。Coreだけでなく、mac・ios・chromeosの変更も起動条件に含めます。デスクトップの共通検証はWindows・macOS・Linuxのマトリックスで行います。OS固有のSDK・UI・エミュレーター検証は独立したジョブとして維持しています。

AndroidとChromeOSの手順もこのファイルだけで管理します。リリースからは`workflow_call`の`packages_only: true`でこの2ジョブを再利用し、同じデスクトップ検証を重ねて実行しません。リリース自身のiOS配布ビルドは維持します。

PR更新時は同じPRの古いデスクトップ・Android実行を中止します。手動実行は完了まで待機します。ビルド成功、シミュレーター確認、実機IME入力の確認は別のものとして報告してください。

## ベンチマーク

通常のビルドでは性能測定を重ねて実行しません。手動のベンチマーク入口と公開用CIで、本家C# 1.4.0（`46db6ca2f941eb9580f4d5ed103ee0bee71111c7`）・前のRust公開版・現在のRustを比較します。本家はこのコミットに固定します。前版は現在のCargoの版より古い公開タグを数値順に選びます。Pre7ならPre6、0.1.1 Pre1なら0.1.0です。`benchmark_baseline`でタグやコミットを指定することもできます。

CoreはRustのみの5枚とC#を含む3者比較5枚、Native C ABIはRustのみの5枚です。入力時間・CPU・RAMを分け、各パネルの表示範囲を調整します。極端な外れ値は図の説明と表へ残し、元データも保持します。同じ版内の比較はbefore/afterを付けます。

21種類の入力を同じコンパイラで比較し、ソース・バイナリ・辞書のハッシュと状態一致を記録します。状態が異なるケースを同等動作の速度比較に使いません。前版と現在版の品質評価もそれぞれ保存し、現在版は既存の品質基準で検査します。異なる公開版には修正差分があるため、両版の品質JSONが完全一致することは要求しません。

Windows・macOS・Linuxの観測はOSごとに分け、異なるランナーの値を合算しません。Android・iOS・ChromeOSは実機がない場合は未測定です。別OSのCore値やクロスビルドを実測として扱いません。成果物は3日間保持し、リリースAssetsには画像や生データを追加しません。

## 権限と保管した定義

`PRチェック`は読み取り権限でPRのコードを実行します。書き込み権限を持つCLA・著作権Botは、引き続き既定ブランチのBotだけをcheckoutし、PRのコードを実行しません。必須チェック名`Flash identity`は維持します。

停止済みの上流C#向け定義、上流専用IME検証、Pre3専用の追加配布定義は`docs/ci-legacy/`へ保管しました。テスト本体や過去の実行履歴は削除していません。現行の全OS配布は`rust-release.yml`を使います。

変更時はactionlint、Flash識別情報の検査、ベンチマークのPythonテストを実行します。各OSのビルドと統合検査はGitHub Actionsで確認します。
