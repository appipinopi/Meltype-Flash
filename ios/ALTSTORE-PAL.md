# AltStore PALへの配布準備（Pre6以降）

対象は `flash-v0.1.0-preview.6` 以降です。この準備でCargoの版番号・iOSのビルド番号・公開タグは変更しません。Pre5の未署名IPAはPALへ登録できません。

## 初回の登録と公証

1. 有料のApple Developerアカウントを用意し、App Store Connectで対象地域の代替マーケットプレイス配布に必要な契約・条件を確認します。
2. [AltStoreの登録API](https://faq.altstore.io/developers/rest-api) でDeveloper IDを登録し、返されたセキュリティトークンを使ってApp Store ConnectにAltStore PALを追加します。トークン・Appleの認証情報・署名証明書はリポジトリに保存しません。
3. 本体 `com.appipinopi.meltypeflash` と拡張 `com.appipinopi.meltypeflash.keyboard` を同じ開発チームに登録し、双方で同じApp Groupを有効にします。登録済みの識別子に変更する場合は `ios/project.yml` の各Bundle Identifierと `MELTYPE_APP_GROUP` を揃えます。
4. Macで `bash ios/build.sh` を実行してRustライブラリとXcodeプロジェクトを生成します。これは未署名ビルド用なので、この時点のIPAをPALへ渡さないでください。
5. Xcodeで `ios/MeltypeFlash.xcodeproj` を開き、本体と拡張の両方の署名を設定します。`CODE_SIGNING_ALLOWED` はプロジェクトで `NO` のため、配布アーカイブでは `YES` に上書きします。提出時のApp Store Connectの案内に従い、配布証明書と双方のプロビジョニングを用意してください。iOSの最低対応バージョンは、PAL自体の導入条件とは別です。
6. リリース時に本体・拡張のビルド番号を一致させ、以前提出したビルドより増やします。XcodeのArchiveからApp Store Connectへアップロードし、代替配布向けの公証に提出します。公証への提出・完了はこのツールでは行いません。
7. 公証後のADP IDを取得し、[AltStoreのADP API](https://faq.altstore.io/developers/rest-api#download-adp) からパッケージを取得・展開します。`manifest.json`・`signature`・`variant/` 以下の構造をそのままHTTPSで公開します。未署名IPAやSimulator ZIPへのURLで代用できません。

アーカイブをコマンドで作る場合の例です。`APPLE_TEAM_ID` は自分の実際のTeam IDに設定し、Xcodeで本体と拡張の署名を設定してから実行します。署名方式によって追加のプロビジョニング設定が必要です。

```sh
xcodebuild -project ios/MeltypeFlash.xcodeproj -scheme MeltypeFlash \
  -configuration Release -destination 'generic/platform=iOS' \
  -archivePath "$PWD/output/pal/MeltypeFlash.xcarchive" \
  CODE_SIGNING_ALLOWED=YES DEVELOPMENT_TEAM="$APPLE_TEAM_ID" archive
```

## ソースJSONの生成

macOS上で、公証へ提出したものと同じ署名済み `.app` と公証後のADPを指定します。次の環境変数には実際のファイルの場所・App Store ConnectのApple ID・開発者表示名・公開URL・公開日を設定してください。`PAL_ADP_MANIFEST_URL` は公開したADPの `manifest.json` へのURL、`PAL_ICON_URL` は公開したPNGアイコンへのURLです。画像をGitHub ReleaseのAssetsへ添付する必要はありません。

```sh
python3 ios/altstore-pal.py \
  --app "$PAL_SUBMITTED_APP" \
  --adp "$PAL_EXTRACTED_ADP" \
  --tag flash-v0.1.0-preview.7 \
  --marketplace-id "$PAL_APPLE_ID" \
  --developer-name "$PAL_DEVELOPER_NAME" \
  --icon-url "$PAL_ICON_URL" \
  --download-url "$PAL_ADP_MANIFEST_URL" \
  --release-date "$PAL_RELEASE_DATE" \
  --output output/pal/altstore-source.json
```

本体・キーボード拡張の署名、チーム・App Group・版番号の一致、実機向けビルド、ADPの主要ファイルを検査します。バージョン・ビルド番号・最低OS・権限は実際のアプリから読み、上の例では表示名にだけ `Pre7` を付けます。`version` を `0.1.0-pre.7` に書き換えないでください。サイズはADP内の1つのvariant IPAのバイト数です。

この検査はAppleの公証ステータス、ADPの暗号署名、提出アプリとADPの対応、HTTPS先の公開状態を保証しません。公証済みの同じビルドを指定し、公開先へADPを欠けなく配置したことを確認してください。

## 公開と更新

生成したJSONを確認して、一定のHTTPS URLで公開します。そのURLをAltStore PALのソース追加に使います。実際にPALから本体とキーボード拡張をインストールし、iOSの設定でキーボードを追加できること、かな・英字の入力、背景の共有、更新を実機で確認してください。

この生成ツールは初回ソース用です。既存ファイルは上書きしません。次回以降は別ファイルへ生成し、既存ソースの `versions` の先頭に新しい版を追加して、過去の配布履歴を残してください。Bundle IdentifierとmarketplaceIDは同じアプリの値を保ちます。公開・Exploreへの登録・GitHub Release作成は自動実行しません。

Exploreへの掲載は配布できることを確認した後の任意の手順です。[ソースの公開仕様](https://faq.altstore.io/developers/make-a-source) に従い、変更できない `fediUsername` を決めてソースに追加してから、[Federate Source API](https://faq.altstore.io/developers/rest-api#federate-source) を呼びます。

## ローカル検査

```sh
python3 -m unittest discover -s ios -p 'test_altstore_pal.py' -v
```

テストは模擬アプリと模擬ADPでメタデータと拒否条件を確認します。Appleによる公証や実機への配布を確認するものではありません。

公式資料: [配布の流れ](https://developer.altstore.io/#how-it-works)・[ソース仕様](https://faq.altstore.io/developers/make-a-source)・[API](https://faq.altstore.io/developers/rest-api)
