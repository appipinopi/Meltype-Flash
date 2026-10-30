# 配布ファイルの選び方

[Meltype-flash v0.1.0 Pre7](https://github.com/appipinopi/Meltype-Flash/releases/tag/flash-v0.1.0-preview.7)の開発プレビューです。

|ファイル|用途|
|---|---|
|`Meltype-flash-Windows-x64.zip`|Windows x64向け。64bit・32bitアプリ用IME、設定画面、登録・ポータブル試用スクリプトを同梱。[導入手順](WINDOWS-PACKAGE.md)|
|`Meltype-flash-Linux-x64.tar.gz`|Linux x64向け。展開先の`Meltype-flash-linux`で`bash install.sh`を実行。[導入手順](../linux/README.md)|
|`Meltype-flash-macOS-arm64.zip`|Apple Silicon Mac向け。[導入手順](../mac/README.md)|
|`Meltype-flash-macOS-x64.zip`|Intel Mac向け。|
|`Meltype-flash-Android-debug.apk`|Android検証用。debug署名です。[導入手順](../android/README.md)|
|`Meltype-flash-ChromeOS.zip`|ChromeOS専用の拡張機能。[導入手順](../chromeos/README.md)|
|`Meltype-flash-iOS-Simulator-arm64.zip`|Apple Silicon MacのiOS Simulator向け。実機へは導入できません。|
|`Meltype-flash-iOS-unsigned.ipa`|自分のApple IDで署名するための未署名IPA。[署名と制限](../ios/README.md)|
|`Meltype-flash-iOS-source.zip`|Xcodeで自分の署名を設定して実機ビルドするためのソース。|

Pre7では単独のWindowsセットアップEXE・portable ZIP、Linuxのdeb・ARM64版は添付していません。OSとCPUに合う配布物を選び、拡張子を変更して他のOSへ流用しないでください。

ライセンスと`SHA256SUMS.txt`も添付しています。画像・ベンチマークのレポートや生データはダウンロード用アセットに含めません。全OSのビルド状況と実機確認の範囲、Windowsの比較テストの残る不一致はリリース本文を確認してください。
