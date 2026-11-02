# Mozcの学習通知・保存・終了の比較

<!-- meltype-flash-document-banner:start -->
<p align="center"><img src="../../../assets/branding/readme/documentation.svg" alt="Meltype-flash — 稲妻のように、言葉の壁を駆け抜ける日本語入力。" width="1000"></p>
<!-- meltype-flash-document-banner:end -->

元版の基準は`45e14f86558aa6ea34f2c2ecd038ba95ca8c3c46`です。元の`MozcConverter.cs`と`CompositionController.cs`は変更せず、元プロジェクトを参照したC#ホストから実行します。

CIでは`prepare_reference.py`でこのコミットのCoreと辞書を隔離展開し、`CoreProject`を指定して比較ホストをビルドします。凍結した13系列とMisspellingsの基準SHAが異なる場合は停止します。通常のチェックアウト内の新しいC#を、古い凍結期待値の比較元には使いません。本家1.4.0の比較・測定は別の固定コミットを使います。

```powershell
python tools/parity/mozc-lifecycle/prepare_reference.py dist/frozen-csharp-reference
$frozenCore = Join-Path $PWD 'dist/frozen-csharp-reference/src/Meltype.Core/Meltype.Core.csproj'
dotnet build tools/parity/mozc-lifecycle/MozcLifecycle.csproj "-p:CoreProject=$frozenCore"
```

展開先は既存フォルダーを上書きしません。`reference-provenance.json`はローカル・CI専用の証跡で、リリースAssetsへ添付しません。

## 通信・保存

`cases.json`の13操作（空の文節列1件、学習要求12件）を、元版とRustの`Mozc`へ同じ順で渡します。タブ／改行／USの置換、文節間のRS、文脈、学習拒否`NG`、成功5回ごとの`S`を全通信ログで比較します。保存を拒否するヘルパーと、終了通知`Q`に応答しないヘルパーも別プロセスで比較します。正常時は`Q`でヘルパー自身の最終保存へ進み、応答しなければ元版と同じ1秒の猶予後に終了させます。失敗した通信・不正な応答では強制終了し、正常保存の経路へ戻しません。

```powershell
rustc --edition=2021 -O rust/native/tests/fixtures/windows_mozc.rs -o helper.exe
dotnet build tools/parity/mozc-lifecycle/MozcLifecycle.csproj "-p:CoreProject=$frozenCore"
cargo build -p meltype-core --example mozc_lifecycle --locked
python rust/native/tests/mozc_lifecycle_smoke.py helper.exe tools/parity/mozc-lifecycle/bin/Debug/net10.0/MozcLifecycle.dll target/debug/examples/mozc_lifecycle.exe
```

終了を無視する試験では、元版の`Dispose`はKill直後に戻るため、C#比較ホストは共有読取りでログを開きます。製品の終了待機や保存処理は変更しません。試験ヘルパーは専用一時フォルダー内の記録だけを更新し、実Mozcの学習プロファイルには触れません。

## 学習対象の選別

`filter-cases.json`の22ケースでは、元版Controllerの文節・文脈を制御し、実際の`Learn`から非同期の`ILearningConverter.Learn`へ渡った文節列を採取します。日本語の連続区間、英語、入力した英字そのもの、翻訳、日時・計算記号、読みの英数字、学習無効、プライベート欄、10 UTF-16単位の文脈切出しと文末境界を含みます。

```powershell
dotnet run --project tools/parity/mozc-lifecycle/MozcLifecycle.csproj "-p:CoreProject=$frozenCore" -- --filter tools/parity/mozc-lifecycle/filter-cases.json > filter-reference.jsonl
python rust/native/tests/mozc_learning_reference.py filter-reference.jsonl
cargo test -p meltype-core --lib original_controller_learning_runs_and_context_match --locked
```

追加した2ケースは、隣の文節のRawや翻訳候補を現在の文節の情報として扱わないことを確認します。元版の結果とソース／ホストのSHA-256は`rust/core/tests/data/upstream-engine-learning.json`へ固定しています。これは選別・文脈の比較です。Rawの生成は下記の別の照合でも確認し、全キー操作・全設定・全候補源の一致とは区別します。

## Windows worker

Coreの確定処理から通知を取り出し、ネイティブのセッションロックを解放してからworkerへ渡します。学習時に候補・変換キャッシュを破棄し、MozcのIPCだけをバックグラウンドへ移します。次のMozc問合せはその処理と直列化し、System／WinRTのCOMオブジェクトは専用STAから移動しません。終了時にも保留中の学習を回収し、正常なヘルパーへ`Q`を送ります。

`windows_worker_converter_smoke.py`は実際のフレーム通信で、学習通知・プライベート／無効設定・通常欄での再開、別候補確定後の先頭候補変更、キャッシュ再照会、750msかかる学習を確定応答で待たないこと、終了保存を確認します。学習後の候補変更は試験ヘルパーによる制御結果で、実Mozcの辞書品質・永続学習の証明ではありません。

残る確認は、Raw／Translations生成の全条件との照合、全キー・編集・独自表・全候補源の組合せ、TSFで選択失効や書込み失敗が起きた後の学習との整合、各OSの学習コールバック、実Mozcの配布と保存データを再起動して使う実動検証です。限定された比較だけで全Core移行完了とは扱いません。

## ローカル検証（2026-10-19）

ワークスペース175テスト、元版Controllerの20ケース、native単体のx86 41テストとRust 1.99の全体／TSF-server Clippyが成功。正常／保存拒否／終了無応答の元版との全通信比較はx64／x86のRust実行ファイルで成功。学習を止める設定、別候補の学習後の再問合せと750msの遅い学習、終了保存・学習中のSystem設定への切替・学習通信中にヘルパーが終了したときの復帰は両ABIの最終Release workerで確認済み。既存ABIと選択再変換の回帰も成功。

結果はローカルの`dist/mozc-learning-*.log`へ保存。元版通信の期待値をRustから作ることはなく、同じ試験ヘルパーを元版とRustから独立に起動して比較する。今回の変更の速度ベンチマークはまだ行っていない。

## 文節ごとの元入力と翻訳情報

`Clause`はRawとTranslationsを個別に保持します。通常の文節生成では、保持している音単位と最終読みに対応する元入力を確認し、音単位の途中で切った場合や対応を証明できない場合はRawを付けません。かな入力・選択範囲からの再変換では通常入力のRawを流用せず、文節伸縮後は古いRaw／Translationsを消します。翻訳を選んだ文節はエンジン学習から除外し、永続履歴にも翻訳として記録します。元の英字を選んだ文節を通常の変換履歴へ登録せず、複数文節を別々に記録した後で文全体を再び通常変換として登録する重複も避けます。文全体の翻訳候補一覧で隣の文節まで除外しません。

```powershell
dotnet run --project tools/parity/mozc-lifecycle/MozcLifecycle.csproj "-p:CoreProject=$frozenCore" -- --raw tools/parity/mozc-lifecycle/raw-cases.json > raw-reference.jsonl
python rust/native/tests/mozc_learning_reference.py filter-reference.jsonl raw-reference.jsonl
```

元の`CompositionText.RawForReading`を、Balanced／Manualの36入力・全346 UTF-16範囲について採取した値は`rust/core/tests/data/upstream-clause-raw.json`です。Rustの文節読みは有効なUTF-8文字列なので、サロゲートペアの途中を切る12範囲は構成できません。元期待値は削除せず保存し、Rustで構成できる334範囲をすべて照合します。拗音の途中、促音、保留n、英語の前後、大文字、句読点、絵文字を含みます。この試験は元版の音単位との対応を比較するもので、同じ入力の全てのUI表示が一致したことを意味しません。

文節のない変換の日英学習と全文節編集経路、全レベル・独自ローマ字表・誤字修正・JISかな・編集履歴・全候補源の組合せ、元入力／全角英字候補を追加する全経路と候補順、TSF適用失敗後の学習の整合と実Mozcの永続学習は継続作業です。

今回の文節情報の変更後は、ワークスペース178テストとRust 1.99の全体／TSF-server Clippyが成功。最終Releaseのx64／x86でABI、元版の文節伸縮20入力列、選択再変換28操作、実フレーム通信の学習ワーカー試験も再実行して成功しました。結果は`dist/clause-metadata-shipping-{x64,x86}-*.log`に保存しています。

## 文節選択による日英学習

通常の文節にChangedを保持し、候補選択API・候補送りで操作した文節だけを変更済みにします。境界伸縮と再変換では初期化します。変更済みの日本語文節でRawと同じ英字を選ぶと英語として、英語文節で日本語候補を選ぶと日本語として学習します。候補からの選択はF6／F10の明示切替とは区別して、既存の回数学習へ渡します。

```powershell
dotnet run --project tools/parity/mozc-lifecycle/MozcLifecycle.csproj "-p:CoreProject=$frozenCore" -- --language tools/parity/mozc-lifecycle/language-cases.json > clause-language-reference.jsonl
python rust/native/tests/mozc_learning_reference.py filter-reference.jsonl raw-reference.jsonl clause-language-reference.jsonl
```

変更していない元ControllerとLanguageMemoryを一時メモリで実行し、通常と同じ組込みローマ字辞書を接続した16ケースを`upstream-clause-language.json`へ採取します。選択の有無、2回必要な短語・日本語、大小文字、元入力不在、英語から日本語、対象外文字、学習無効・プライベートを比較します。Rustは学習方向・回数・有効状態を照合し、実際の文節選択から確定までと伸縮時の状態破棄も別に検査します。全候補生成経路、文節を持たない単一変換、言語判定表示、実アプリ内での入力と永続保存の全組合せはこの比較だけでは証明しません。

この日英学習の追加後は全180テストとRust 1.99の全体／TSF-server Clippyが成功。最終Releaseのx64／x86でABI・文節伸縮20入力列・選択再変換28操作・学習ワーカーも成功しました。ローカル結果は`dist/clause-language-*.log`に保存しています。

## 通常変換履歴と翻訳履歴の条件

元ControllerのLearnから、ConversionHistoryとTranslationHistoryへ実際に渡された結果を25ケースで採取します。選び直していない文節、英語文節、1 UTF-16単位の読み、ひらがな・カタカナ・元入力と同じ候補、日時・計算記号は通常の優先候補へ登録しません。翻訳候補の所属は別の条件で扱い、選び直し状態や読みの長さで選択回数を捨てません。元と同じく読みの長さはUTF-16単位で数えます。

```powershell
dotnet run --project tools/parity/mozc-lifecycle/MozcLifecycle.csproj "-p:CoreProject=$frozenCore" -- --history tools/parity/mozc-lifecycle/history-cases.json > clause-history-reference.jsonl
python rust/native/tests/mozc_learning_reference.py filter-reference.jsonl raw-reference.jsonl clause-language-reference.jsonl clause-history-reference.jsonl
```

期待値は変更していない元Controller／ConversionHistory／TranslationHistoryから取得し、`upstream-clause-history.json`へ固定します。翻訳の回数、1文字の翻訳、変更していない候補、英語文節、日時と計算記号、学習無効・プライベート、サロゲートペアの読みを含みます。Rustでは通常の候補を選ばず確定した場合と、同じ先頭候補を明示的に選んでから確定した場合も分けて検査します。短い翻訳とUTF-16の読みは一時プロファイルへの保存・読込みでも検査します。

通常入力のライブ変換から通常履歴を自動登録する処理を除き、文節のない変換でも選び直し状態を通常履歴の条件へ適用します。これは履歴の選別の照合であり、文節のない変換の日英学習、全候補源・全文節編集経路・保存制限・設定画面の一致や、TSF適用失敗時の学習整合まで証明するものではありません。

今回の履歴修正後は全183テストとRust 1.99の全体／TSF-server Clippyが成功。最終Releaseのx64／x86でABI・文節伸縮20入力列・選択再変換28操作・学習ワーカーも再実行して成功しました。結果は`dist/clause-history-workspace-verified.log`と`dist/clause-history-shipping-{x64,x86}-*.log`へ保存しています。先行試験では古いカタカナ学習前提の回帰が1件失敗し、そのログも保持しています。通常候補の選択学習を検査する試験へ修正し、カタカナを通常履歴へ登録しない元版の期待値は比較データ側で維持しています。

## 単一変換と go の入力中候補

`Choice.cs`は元の公開`MeltypeSession.HandleKey`／`SelectCandidate`を使用し、文字入力→Space（またはgoの入力中候補）→候補選択→Enterを実行します。元版の組込み辞書・LanguageMemoryを接続したManualの16操作について、確定文字列と日英学習の方向・回数・有効状態を`upstream-single-choice.json`へ採取します。Rust側も公開のキー／候補選択入口を通し、通常の単一変換が文節リストを持たない経路であることを明示的に検査します。候補一覧の全順序は比較対象にしていません。

```powershell
dotnet run --project tools/parity/mozc-lifecycle/MozcLifecycle.csproj "-p:CoreProject=$frozenCore" -- --choice tools/parity/mozc-lifecycle/choice-cases.json > single-choice-reference.jsonl
python rust/native/tests/mozc_learning_reference.py filter-reference.jsonl raw-reference.jsonl clause-language-reference.jsonl clause-history-reference.jsonl single-choice-reference.jsonl
```

hashi・go・sushiの元入力への選び直しと2回待ち、goの入力中候補、学習無効とプライベート欄を比較します。全大文字は自動のSpace処理とは分け、F6のあとSpaceで候補を開く操作として固定しています。表示字種を変えても変換用の区間が英語であれば、英字候補の選択は日英の訂正として覚えません。同じ英語区間で日本語候補を選んだ場合は日本語として記録します。Fキーで表示を変えてから変換した場合は、変換候補の学習へ同時に別の明示Fキー学習を混ぜません。

Rustは文節のない変換に同じ日英学習の選別を適用し、goの入力中候補へもRawを渡します。比較対象の16操作では、入力どおりの英字などをエンジンへ学習通知しないことも検査します。全レベル・各文脈・途中編集・全大文字の自動Space操作・全候補源の分類一致、Fキー後の全エンジン通知、TSF適用失敗時の学習整合、実アプリ上の操作は継続確認です。

今回の単一変換修正後は全184テストとRust 1.99の全体／TSF-server Clippyが成功。最終Releaseのx64／x86でABI・文節伸縮20入力列・選択再変換28操作・学習ワーカーも成功しました。結果は`dist/single-choice-workspace-first.log`と`dist/single-choice-shipping-{x64,x86}-*.log`に保存しています。元版で全大文字のSpace時に候補が開かない先行観測や、F6後の誤った英語学習を検出した先行失敗も保存し、上記の固定した操作と区別します。

## Fキー後の変換とエンジン学習

元の公開MeltypeSessionで文字入力→F6〜F10→Enter／Space／変換キー→Enterを24操作採取し、変換状態・確定全文・エンジン学習通知を`upstream-engine-keys.json`へ固定しています。元版Controllerとセッションは変更せず、試験変換器の応答と学習通知だけを観測します。非同期学習は専用の単一スレッドと末尾通知で完了を待ちます。

```powershell
dotnet run --project tools/parity/mozc-lifecycle/MozcLifecycle.csproj "-p:CoreProject=$frozenCore" -- --engine-keys tools/parity/mozc-lifecycle/engine-key-cases.json > engine-keys-reference.jsonl
python rust/native/tests/mozc_learning_reference.py filter-reference.jsonl raw-reference.jsonl clause-language-reference.jsonl clause-history-reference.jsonl single-choice-reference.jsonl engine-keys-reference.jsonl
```

F6〜F8で表示字種を変えた後でも、実際に変換した日本語はエンジンへ学習通知します。SeparateKeyRolesがOFFの場合、F9／F10の後のSpaceは表示中の英字に空白を付けて確定し、変換やエンジン学習を始めません。変換キーを使えば変換でき、通常の学習選別を適用します。学習無効とプライベート欄では通知せず、英語文節の日本語候補も元のエンジン学習対象から除きます。

Rustでは単一変換の文節情報を通常履歴・日英学習・エンジン通知で共有します。Coreの比較は仮の外部変換応答をキー分岐後に設定する試験であり、OSへの配送の証明とは分けます。文節ありと文節なしの経路を両方検査し、全大文字の英語文節2操作は単一変換経路で比較します。Windowsの実フレーム通信試験にもF6〜F8後の学習通知とF9／F10のSpace確定を加えています。

残る確認は全判定レベル・途中編集・候補源と全キーの組合せ、TSF適用失敗時の学習整合、実Mozcの永続学習、各OSの実アプリでの入力です。この限定比較でCore全体やUI一致の完了とはしません。修正前の学習通知欠落は`dist/engine-keys-before-fix.log`へ保存しています。今回の変更後の速度は未測定です。

先行のworker検査では、ネイティブABIがCoreのキー処理より前に外部変換を始め、F9のSpaceが英字確定にならない差も検出しました（`dist/engine-keys-shipping-x64-worker.log`）。ネイティブの変換開始条件もCoreと同じ判定を参照するよう修正し、最終worker検査では英字確定時の変換問合せが増えないことまで調べます。

今回の全187テストが成功。最後のnative先行変換修正後もCore／sessionの関連36テスト・native単体41テストとRust 1.99の全体／TSF-server Clippyが成功しました。最終Releaseのx64／x86で追加したFキー後の実フレーム通信、既存ABI・文節伸縮20入力列・選択再変換28操作・結合文字8入力が成功。ログは`dist/engine-keys-final-*.log`へ保存。試験ヘルパーによる通知検証であり、実Mozcの永続辞書学習は未確認です。

## キーの役割を分ける設定とF9／F10

比較を30操作へ拡張しました。元版ではSeparateKeyRolesがONの場合、F9／F10の後でもSpaceは候補を開きます。OFFの場合の英字＋空白の確定とは別の動作です。直近の英字確定の修正がONにも適用されていた差を`dist/separate-engine-keys-before-fix.log`へ保存し、Coreとnativeが参照する共通の条件へ設定判定を追加しました。

元版から採取した最初の24操作は不変です。追加6操作は通常表示とF6〜F10のあと、SeparateKeyRolesをONにしたSpaceでの変換状態・確定全文・学習通知を比較します。Windowsの実フレーム通信でも設定OFFの英字確定／問合せ抑止と、ONの候補開始／確定後の学習通知を同じworkerで検査します。全設定や全キー操作の組合せを証明するものではなく、全候補源・途中編集・実Mozcの永続学習・TSF適用失敗時の学習整合は継続確認です。今回の修正後の速度は未測定です。

SeparateKeyRolesの修正後は関連Core／session 36テストとRust 1.99の全体／TSF-server Clippyが成功。最終Release x64／x86で、設定ONのF9／F10後の候補開始と学習通知、OFFの英字確定と外部問合せ抑止、既存ABI・文節伸縮20入力列・選択再変換28操作が成功しました。ログは`dist/separate-engine-keys-*.log`へ保存しています。全187テストは直前版の実行結果で、今回の修正では関連範囲と最終実行ファイルの回帰を確認しています。

## Shift＋Space

比較を50操作へ拡張し、先行30操作は不変です。通常の日本語と英語cameraについてF6／F9／F10後のShift＋Spaceを元の公開キー入口から採取します。ManualとBalanced（cameraが英語になる条件）を区別し、変換状態・確定全文・エンジン通知を比較します。Core単体ではホストが供給するスペル事実を同じ条件で渡します。

入力中のShift＋Spaceは日本語候補を優先して変換を開始し、変換中は前の候補へ、入力が空のときだけ全角スペースを挿入します。Rustの以前の処理は入力中でも全角スペースを確定していたため修正しました。native側でもShiftを外部変換へ接続し、選択再変換の英語候補展開処理を使って日本語候補を先頭へ出します。

実DLLの`shift_space_smoke.py`は元版の20操作で確定全文と変換表示を比較し、各入力で前の候補への循環・Spaceで先頭へ戻る動作と未入力時の全角スペースも検査します。Windows workerのフレーム試験にも変換開始・候補戻し・確定後の学習通知を追加しています。

cameraの比較で、既知のc行の綴りをかな辞書より先に英語とする元のIsEnglishSpanの条件も移植しました。Fキーで見せる字種と、変換用の区間分類を分離します。比較は限定された語・設定・試験変換器の条件であり、全ての英語区間・保護範囲・スコア判定・カスタム表・実エンジンの候補順を証明しません。全機能とUIの移行は継続中で、今回の変更後の速度も未測定です。

Shift＋Space修正後は全187テスト・Rust 1.99の全体／TSF-server Clippyが成功。最終Releaseのx64／x86で専用20操作の変換表示と確定・候補戻し／進め・空入力、workerの日本語通知と英語cameraの通知除外、既存ABI・文節伸縮20入力列・選択再変換28操作が成功しました。ローカル結果は`dist/shift-space-*.log`へ保存。先行ABI試験の古い全角スペース挿入前提による失敗も保持し、元版から採取した操作に合わせた期待値で再検査しています。

## F9／F10の見た目が変わるケース切替

元の公開MeltypeSessionでF9連打・F10連打・両キーの切替を96入力列（各6操作）実行し、各キーのconsumed／表示全文／変換状態を`upstream-case-keys.json`へ採取します。Manual／Balanced、ひらがな表示のhashi、すでに英語表示のhello、大文字HASHI、単字、数字、GitHub・apiV2を含みます。

```powershell
dotnet run --project tools/parity/mozc-lifecycle/MozcLifecycle.csproj "-p:CoreProject=$frozenCore" -- --case-keys tools/parity/mozc-lifecycle/case-key-cases.json > case-keys-reference.jsonl
python rust/native/tests/mozc_learning_reference.py filter-reference.jsonl raw-reference.jsonl clause-language-reference.jsonl clause-history-reference.jsonl single-choice-reference.jsonl engine-keys-reference.jsonl case-keys-reference.jsonl
cargo test -p meltype-core --test upstream_case_keys --locked
```

元版は同じ見た目の段階を飛ばします。HASHIを英字表示中のF10はHashiへ進み、すでに英字表示のhelloの最初のF10はHELLOになります。F9／F10で幅を変えるときは、幅を変えた結果が現在表示と異なればまず打ったままの綴りを表示します。最大3段階の探索で、数字や単字も無限ループしません。変換候補の表示中は自動表示と区別し、打ったままから切替を開始します。

Rustは切替前の表示と次の字種を比較し、同じ表示を飛ばします。専用C ABI試験`case_keys_smoke.py`も全96入力列で表示を照合し、最後にEnterで表示どおり確定することを検査します。元版ソースは変更せず、ホスト・入力・ソースのハッシュを固定データへ保持します。先行の全大文字での不一致は`dist/case-keys-before-fix.log`へ保存しています。

この比較はASCII英字・数字のケース切替と全角化が中心で、Unicode全般の大文字変換、全編集履歴・JISかな・全候補源・変換中の全操作は引き続き確認が必要です。全Core・UIの移行完了や高速化を示す比較ではなく、今回の変更後の速度は未測定です。

回帰で見つかったreviewと@nameも元版から追加採取し、先行78入力列は不変でした。@nameはSigilWordsDirectをOFFにして明示的に組成へ入れる条件です。元版のreview→REVIEW→Review→review、@name→@NAME→@nameの切替順に古い回帰試験の期待値を合わせています。先行の2件の失敗ログは保持しています。

Mac向け共通Core回帰のseeyouagainも元版へ追加し、先行90入力列は不変でした。初回F10でSEEYOUAGAINへ変わる結果を記録し、明示字種を選んだ後には自動の単語間スペースを加えない回帰条件を維持しています。

ケース切替の最終修正後は全188テストとRust 1.99の全体／TSF-server Clippyが成功。最終Release x64／x86で全96入力列の表示と最終確定、学習worker、既存ABI・Shift＋Space20操作・文節伸縮20入力列・選択再変換28操作も成功しました。結果は`dist/case-keys-*.log`へ保存。先行の古い期待値による失敗も保持しています。

## 数字キーによる候補選択

`NumberKeys.cs`は元のControllerに既に展開した候補を設定し、公開HandleKeyで39状態を採取します。1〜9とテンキー、9件単位のページ、複数文節の次文節への移動と最後の確定、候補不足の番号の通常入力、かな入力時のテンキーを含みます。これは候補生成を省いた状態遷移の検査です。

別途`PublicNumberKeys.cs`は内部状態を書き換えず、文字入力からSpace・文節選択・候補移動・番号・Enterまで48入力列を元の公開Sessionへ送ります。固定変換器は各読みへ20候補を返し、元版の同梱候補辞書も読み込みます。`upstream-public-number-keys.json`に全返信を保持し、`number_keys_smoke.py`が実DLLの同じキーと変換コールバックで、消費・確定編集・表示・全文節・変換中の選択位置と候補一覧を比較します。1〜4ページ目、ローマ字候補と絵文字を含む末尾、番号が無いときの入力継続を検査します。非変換中の選択位置（元版-1／Rust 0）、ヒントや意味注釈、実OSの描画はこの照合の対象外です。

比較で数字キー処理の欠落に加え、読みが辞書候補より前へ入ること、打ったローマ字と全角形の欠落、意味注釈用辞書の逆引き語が余分な候補になることを発見しました。外部変換文節の候補組立てを修正し、ローマ字の後ろへ絵文字を配置します。Clauseの生成は入力された候補順を維持し、空の応答だけ読みへフォールバックします。変換器が無いローカル変換経路と全候補源の順序、候補の取得タイミング、全設定・編集履歴の組合せは継続確認です。

```powershell
dotnet run --project tools/parity/mozc-lifecycle/MozcLifecycle.csproj "-p:CoreProject=$frozenCore" -- --number-keys tools/parity/mozc-lifecycle/number-key-cases.json
dotnet run --no-build --project tools/parity/mozc-lifecycle/MozcLifecycle.csproj -- --public-number-keys tools/parity/mozc-lifecycle/public-number-key-cases.json
python rust/native/tests/number_keys_smoke.py target/release/meltype_native.dll
```

CIは元版から両固定データを再採取して照合し、DLL試験を各OSとWindows x86へ追加しています。今回の修正後の速度は未測定で、全Core・UIの移行完了を示すものではありません。

最終修正後は全189テストとRust 1.99のワークスペース／TSF-server Clippyが成功。最終Releaseのx64／x86で公開キー48入力列の消費・確定・表示・文節・変換中の選択位置と候補一覧が一致しました。既存ABI、ケース切替96列、Shift＋Space20操作、文節伸縮20列、選択再変換28操作、結合文字8入力も両ABIで成功。最終Release workerの実フレーム通信も成功しています。元版から39状態と48入力列を再採取する参照検査も成功し、先行の候補順の不一致ログは保存しています。結果は`dist/number-keys-*.log`へ記録しました。

## 変換中の取消とTabの役割

`ConversionKeys.cs`は元の公開MeltypeSessionへ文字入力からキーを送り、内部状態を変更せず84入力列を採取します。Manual・固定20候補・元版の同梱候補辞書で、ライブ変換・予測・学習はOFF。TabConversionのON/OFF、1／2文節、Backspace・Ctrl＋H・Esc・変換キー・Shift付き上下、Tab／Shift＋Tab、取消・字種変更後の再変換、F6〜F10から変換して取消を含みます。`upstream-conversion-keys.json`へ全返信と参照ソースのハッシュを固定し、数字選択と共通のDLL試験で消費・編集・表示・文節・変換中の選択位置と候補一覧を照合します。

Rustでは最初のBackspaceは変換だけを取り消し、次のBackspaceで入力を削除します。変換前に選んだ字種もBackspace／Escで復元します。変換キーはキー役割分離設定にかかわらず候補を進め、Shift＋Downも前ではなく次の候補です。Tabで始めた変換だけTab／Shift＋Tabで候補を巡回し、Spaceなどで始めた変換のTabは確定してアプリへ通します。Shift＋Tabでは新たな変換を始めず、取消・Fキー・新しい入力・確定でTab起点の状態を解除します。

nativeはCoreがTabの変換開始を受理した後で外部変換器へ問い合わせます。これにより提案・予測・保護入力の優先処理へ先回りしません。Windows workerのフレーム試験にもTab開始・候補巡回・取消・SpaceからのTab通過と、F7後に変換してEscで字種を戻す操作を追加しています。変換キーの日本語優先経路でも既知英単語と未知の綴りの境界を保ち、既存の文節伸縮ケースを回帰に使います。

```powershell
dotnet run --project tools/parity/mozc-lifecycle/MozcLifecycle.csproj "-p:CoreProject=$frozenCore" -- --conversion-keys tools/parity/mozc-lifecycle/conversion-key-cases.json
python rust/native/tests/number_keys_smoke.py target/release/meltype_native.dll rust/core/tests/data/upstream-conversion-keys.json
```

この比較は全設定・全候補源・JISかな・予測の選択解除・変換中の誤字修正・実OSの描画までを証明しません。全Core移行とUIの照合は継続中で、今回の修正後の速度は未測定です。

字種復元までの全189テストが成功し、最後のnative英語区間修正後もnative単体41テストとRust 1.99の全体／TSF-server Clippyが成功しました。最終Releaseのx64／x86で84入力列、数字選択48列、既存ABI・ケース切替96列・Shift＋Space20操作・文節伸縮20列・選択再変換28操作・結合文字8入力が成功。最終Release workerでは実フレーム通信で追加のTab操作と字種復元も成功。元版84列の再採取照合も成功し、先行の取消・字種・英語区切りの不一致ログは保持しています。結果は`dist/conversion-keys-*.log`へ保存しました。

## 予測候補の表示・操作・学習

`PredictionKeys.cs`は元版の公開Sessionと実際のPhraseHistory／Predictorを使い、110入力列を採取します。学習済み日本語3語句と英語予測、Manual／Balanced、小文字・先頭大文字・全大文字、保留子音、7回のTab／Shift＋Tab、Escの解除と全取消、追加入力・Backspace・F7、Enterとクリック確定を含みます。日本語予測4ケースでは学習を有効にし、確定後の語句の読み・文字列・回数も採取します。履歴とプロファイルは各ケースの一時ディレクトリだけを使い、ユーザーの入力欄へ配送しません。

RustのTab巡回は候補の末尾と先頭の間に未選択（-1）を挟み、選択した予測を途中表示へ反映します。最初のEscは選択だけを解除し、次のEscで入力を消します。Fキーや編集では予測選択を解除します。日本語予測には未確定の子音を含めず、ひらがなの打ち終わった部分を使います。英語予測は英字表示の3文字以上か、かな表示でも英字原文4文字以上の場合に追加。手動の英字提案や誤字提案、明示字種では予測を隠します。Manualの提案をTabで受けた場合はF10を選ばず、その組成の判定だけBalancedへ切り替えます。

予測を確定した学習は、入力途中の読みではなく元の語句の完全な読みに戻して回数を増やします。通常確定の語句学習は予測設定ON・漢字を含む日本語に限定し、英字だけや予測設定OFFの確定を語句履歴へ追加しません。予測一覧を作る際の条件と、明示候補を学習する条件は分けます。

公開キー比較は入力途中のcame、cameraへ文字を足す場合のc行英語区間、全大文字略語にF7を使う場合の差も検出しました。参照ホストへローマ字訂正辞書を接続すると4列の確定結果が変わり、未接続時の採取ログも保持しています。Rustの訂正器から広い英単語一覧による一律除外を取り除き、Session側で同梱の一般語と混在英語を除外します。訂正後の綴りを再判定し、camer→cameなどを無条件にかな表示しません。

`upstream-prediction-keys.json`へ元の全返信とソースハッシュを固定し、Coreの公開入口とC ABIで消費・編集・表示・予測一覧・選択位置・保存後の語句を比較します。DLLでは変換中の候補一覧と文節も比較。ヒント・意味注釈・非変換中の候補選択sentinel・使用日時そのもの・実OSの描画は比較対象外です。ライブ変換はOFFで、ユーザー辞書／変換履歴を加えた予測順位、全判定レベル・かな入力・全誤字と全編集履歴は継続確認です。変換中のTabによる誤字修正の移行は残っています。

```powershell
dotnet run --project tools/parity/mozc-lifecycle/MozcLifecycle.csproj "-p:CoreProject=$frozenCore" -- --prediction-keys tools/parity/mozc-lifecycle/prediction-key-cases.json
cargo test -p meltype-core --test upstream_prediction_keys --locked
python rust/native/tests/prediction_keys_smoke.py target/release/meltype_native.dll
```

今回の修正後の性能は未測定です。参照ホストの採取と再照合、両ABI試験をCIへ追加しています。全Core・UIの受入完了を示すものではありません。

訂正器単体も`DirectTypos.cs`で20入力を採取し、正常な修復案・候補なしを`upstream-direct-typos.json`へ固定しました。元版は単体のgoogle→gogoleを返すため、古い「単体でgoogleは常に候補なし」のテストを置き換えました。Sessionでgoogleを入力して確定する場合は英語のまま保つ別の検査を維持します。入力欄が途中表示を確定済みにした場合も、漢字を含まない未完成かなを語句履歴へ追加しない条件へ既存回帰を合わせています。これらは元版のRememberPhraseと公開操作で確認した条件です。

最終Release x64／x86では予測110入力列と保存後の語句、数字48列・変換操作84列、既存ABI・ケース切替96列・Shift＋Space20操作・文節伸縮20列・選択再変換28操作・結合文字8入力が成功しました。Windows workerでも予測Enter／クリック4列の表示・確定・完全な読みの学習と、既存のSystem／Mozc／Hybrid通信が成功。元版の110列と訂正器20入力の再採取照合、Rust 1.99の全体／TSF-server Clippyも成功しています。先行失敗ログは保持し、ローカル結果は`dist/prediction-keys-*.log`へ記録しています。

予測修正後の最終ワークスペース全190テストと、Rust 1.99の全体／TSF-server Clippyが成功しました。途中の古い単体・学習期待値による失敗ログも保持しています。
# 誤字修正の公開キー比較（追加）

`--misspelling-keys misspelling-key-cases.json` は、入力中・変換中のTab修正、Shift+Tab、修正後の削除・字種切替・確定、Manual/Balanced、CorrectTypos/TabConversionの設定、JISかな、URL・英語の除外を104入力列で採取します。Unicodeの8列は通常の `a` キーで組成を開始してから続けます。元版の公開Sessionは空入力で直接Unicode文字を開始しないためです。先行96列の期待値は不変です。

このホストはTranslationDictionaryを接続しないため、native比較側もTranslationCandidatesをOFFにします。その他の候補源を含む全設定の比較ではありません。`rust/core/tests/data/upstream-misspelling-keys.json` が元版の採取結果とソースハッシュを保持し、非変換の75列をCore、全104列の編集・表示・誤字案内・変換中の候補一覧をnative ABIで検査します。`mozc_learning_reference.py` の13番目の引数に再採取したJSONLを渡すと全行を照合できます。

```
dotnet run --project tools/parity/mozc-lifecycle/MozcLifecycle.csproj "-p:CoreProject=$frozenCore" -- --misspelling-keys tools/parity/mozc-lifecycle/misspelling-key-cases.json
python rust/native/tests/number_keys_smoke.py target/release/meltype_native.dll rust/core/tests/data/upstream-misspelling-keys.json
```
