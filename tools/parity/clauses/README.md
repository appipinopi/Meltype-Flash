# 文節伸縮と変換器候補の元版比較

<!-- meltype-flash-document-banner:start -->
<p align="center"><img src="../../../assets/branding/readme/documentation.svg" alt="Meltype-FLASH — 稲妻のように、言葉の壁を駆け抜ける日本語入力。" width="1000"></p>
<!-- meltype-flash-document-banner:end -->

基準は元版`45e14f86558aa6ea34f2c2ecd038ba95ca8c3c46`の実際のMeltypeSession/CompositionControllerです。`Program.cs`から基準コミットを展開したCoreを参照し、20入力列を実行します。Rustの出力から期待値を生成しません。

```powershell
dotnet run --project tools/parity/clauses/Clauses.csproj -p:CoreProject=<元版Meltype.Core.csprojの絶対パス> -- tools/parity/clauses/cases.json
python rust/native/tests/clause_resize_smoke.py <target>/debug/meltype_native.dll
cargo +stable test -p meltype-core --test clause_refresh --locked
```

元版の応答は`rust/core/tests/data/upstream-clause-resize.json`に固定。ホストソースのSHA-256も記録します。比較は全キーの消費・確定編集・表示文字・変換状態・文節文字列、変換中の候補／文節選択位置と、試験用変換器の候補／別候補の順序です。入力中の未選択位置の表現（元版-1／Rust 0）、全補助辞書の候補一覧、意味注釈、画面配置は比較対象に含めません。

文節の伸縮、最短／最後の文節の無操作、結合、次文節選択、別候補の選択、取消、変更のない後続文節の候補保持、単独変換の失敗を含みます。英語文節の音による読み直し、英語の手前の短縮、読み直せない英語や保護URLの無操作、英語終わりのSpace確定と変換キーの違いも含みます。変換器は入力を固定の2／3文節に区切り、単独変換とは意図的に異なる結果を返します。元版のConvertとConvertClausesの使い分けを検査します。ライブ変換・学習・予測は無効、同梱スペル辞書と製品で使うローマ字訂正語彙は有効です。参照ホストに訂正語彙を接続して再採取した際も、既存12件の応答が変わっていないことを照合しました。

## ABI

既存`meltype_init`に加え、`meltype_set_converter`で単独変換を任意登録できます。`meltype_init`の後に登録し、戻り値は既存の候補／文節コールバックと同じmalloc所有権のUTF-8文字列です。NULL／空応答では読みを保持します。登録しないフロントエンドは既存の分割変換結果の連結へフォールバックします。単独変換と分割変換が異なる実エンジンは登録が必要です。各OSフロントエンドからの登録は今後の統合作業として残ります。

伸縮で変更された文節だけ候補を取り直します。コールバックはセッションレジストリのロック外で呼び、結果を適用する直前に読みの一覧を照合します。同一セッションへの呼び出しの直列化は既存ABI契約どおり必要です。実IMEの登録・利用者の入力・辞書を使わず、私用の一時プロファイルで検査します。

全候補源の順序、独自ローマ字表・編集履歴との全組合せ、ライブ変換のキャッシュ、実OSの変換器・TSF操作を含む全文節互換性は、この20件だけでは証明しません。
