# Windows変換器のRust移植と元版の比較

<!-- meltype-flash-document-banner:start -->
<p align="center"><img src="../../../assets/branding/readme/documentation.svg" alt="Meltype-FLASH — 稲妻のように、言葉の壁を駆け抜ける日本語入力。" width="1000"></p>
<!-- meltype-flash-document-banner:end -->

基準は `45e14f86558aa6ea34f2c2ecd038ba95ca8c3c46`。`KanjiConverter.cs` と `WinRtCandidates.cs` の元の実装を変更せず、STAのC#ホストから実行します。Rust側も同じPCのMicrosoft IME／Windows候補APIを呼びます。実OSの結果はOS・辞書の版によって変わるため、Rustの出力を固定期待値にせず、その環境で採ったC#の応答と照合します。

```powershell
dotnet run --project tools/parity/windows-converter/WindowsConverter.csproj -- tools/parity/windows-converter/cases.json > reference.jsonl
python rust/native/tests/windows_converter_smoke.py <meltype_native.dll> reference.jsonl
rustc --edition=2021 -O rust/native/tests/fixtures/windows_mozc.rs -o helper.exe
python rust/native/tests/windows_worker_converter_smoke.py <meltype-tip-worker.exe> helper.exe reference.jsonl
```

変換、文脈付きの文節解析、Windows候補の一覧・順序を26入力で比較します。空入力、英語、句読点、数字、前後の文脈、89／90／99／100／101／190／256文字、長い文脈、サロゲートペアを含みます。Microsoft IMEが無い環境では、元版と同じ未利用時の応答を検査します。利用できない環境での成功を実変換の成功と扱いません。

候補APIは元版と同じ `IAsyncInfo` を1度取得し、500msで取消します。時計も[.NET 10のEnvironment.TickCount64](https://github.com/dotnet/runtime/blob/v10.0.0/src/libraries/System.Private.CoreLib/src/System/Environment.Windows.cs)が使う `GetTickCount64` に合わせています。非同期の期限切れと候補の内容を区別するため、参照ホストは空の結果だけ最大3回照会し、元版が返した空でない一覧を優先します。Rustの比較側は明示的に期限切れと報告されたときだけ最大3試行し、変換・文節の差は再試行しません。最後には全候補の一致を要求し、候補の不一致を成功扱いしません。元版の空応答とRustの完了結果が異なった場合も、その実行は失敗として記録します。OS候補の応答期限は実行負荷に左右されるため、最終照合では他のビルドを終了してから両方を実行します。空応答や長文を比較対象から除外しません。測定時間の比較用ホストではありません。

`synthetic.json` の27ケースは元版の `SplitForEngine` と `ParseClauses` を反射で呼んで採取します。結果とホスト／元ソースのSHA-256は `rust/native/tests/data/upstream-windows-converter.json` に固定。文節の品詞による結合、文脈の読み直し、境界をまたぐ語、読みに空白がある場合、範囲外・欠落データ、1バイトpackの構造体、UTF-16の分割をRust単体テストでも照合します。無効なポインターを作る試験ではありません。

## workerへの接続

Windowsのworkerは専用STAでMicrosoft IMEとWinRT候補を所有し、終了前にClose／Releaseします。`ConversionEngine` の実際の設定名は `Hybrid`（両方）、`Mozc`、`System` です。TIPリクエストの有効なアプリ設定ごとに切り替え、単独変換・文節変換・追加候補の既存ABIへ接続します。診断だけを呼ぶ場合は `meltype_windows_converter_probe` が同じ変換器を利用します。

Mozcヘルパーはworkerの隣の `mozc/meltype_mozc_helper.exe`、学習プロファイルはMeltypeデータフォルダー内の `mozc/` です。起動は非同期で、READYを入力処理で待ちません。実行フォルダーはヘルパーのフォルダー、ウィンドウは非表示とし、元版同様にDLL読込みエラー画面を抑止する設定を子へ渡し、起動直後に親の設定を戻します。変換・候補の512件境界のキャッシュ、候補控えの復元、空の成功応答と失敗の区別、3回失敗後の停止を扱います。Mozcが使えない場合はMozc指定でもSystemへ戻り、使えるMozcから空応答が返った場合にはMozc指定のまま読みを保持します。

workerの試験は独立した一時フォルダーで実際のフレーム通信を使います。Systemでは実APIの結果をC#と照合し、MozcはRust製の試験ヘルパーで起動・問い合わせ・別候補選択・キャッシュ・空応答・失敗・設定切替・プライベート欄での変換と学習抑止・無効設定の無問い合わせを検査します。試験用ヘルパーの成功を実Mozcの辞書・学習の検証と扱いません。IMEの登録・有効化や利用者の文書への入力は行いません。

既存のSDKホストは `MELTYPE_TEST_TIP_NO_CONVERTERS=1` により実辞書を使わない条件を維持します。公開TSF入口の実System接続を比較するときだけ、C#で採った「にほんご」の変換結果を `MELTYPE_TEST_SYSTEM_NIHONGO` に渡します。CPP所有者／Rust所有者、実Windows SDK配置の範囲を用いる条件は[再変換ホスト](../reconversion/README.md)を参照してください。

[学習通知とキャッシュ破棄・正常終了保存](../mozc-lifecycle/README.md)は追加済みです。実Mozcの配布物同梱・永続学習の実動確認、DLL欠落の細かい再試行分類、全候補源・補助辞書・翻訳・意味注釈との最終的な順序、実アプリの入力は継続作業です。これらを含む完全移植の完了条件はまだ満たしていません。

## このPCでの検証（2026-10-19）

- 最終Releaseのx64／x86で、26入力の変換・文脈付き文節・全候補一覧が一致。25入力は利用可能なMicrosoft IMEを実際に使用。WinRT期限切れによる規定内の再試行はx64で2回、x86で1回。
- 両ABIの実workerでSystemのC#照合、Mozcの起動・作業フォルダー・子のエラーモード、キャッシュ、別候補確定、空応答、モード／無効設定、プライベート欄の学習抑止と通常欄での学習再開、欠落／3回失敗後の復帰を確認。
- ワークスペース172テスト、最終native単体41テストを各ABI、Rust 1.99の全体／TSF-server Clippyが成功。外部変換器を使わないSDKホスト14種類も各ABIで成功。
- ビルドと並行した先行実行は、長文256の候補がC#では空・Rustでは5件で不一致だった。変換・文節の差はなく、この失敗ログを保持した。最終照合はビルド終了後に参照も採り直したもので、負荷に依存する候補取得の期限境界まで常に同時に完了することは保証しない。

元版の候補応答はOSの辞書を含むためコミットせず、ローカルの`dist/windows-converter-reference-idle.jsonl`と`dist/windows-converter-idle-*.log`に保存。先行失敗は`dist/windows-converter-private-x64-live.log`。SDKの保存先は`dist/windows-converter-sdk-verified/`と同名のログ。実Systemを使う公開TSF SDKホストの追加CIは、このローカルの外部変換器なしの検証と区別する。


`corpus.py <出力フォルダー> --sample 128` は20,181件の同梱読みから均等に128件を選び、既存の境界26入力と合わせ154入力を生成する。参照ホストへ生成した cases.json を渡し、`windows_converter_smoke.py <DLL> <参照JSONL> <生成cases.json>` で同じ実OS応答を照合する。これは有限のサンプルであり、OS辞書の全入力・全応答を網羅するものではない。


`corpus.py <出力フォルダー> --all` はサンプル抽出をせず、同梱の全20,181読みと境界26入力を生成する。生成完了と比較成功を区別する。`batch_compare.py windows` の使い方と再開・失敗保持は [BATCHES.md](../BATCHES.md) を参照。`windows_converter_smoke.py` の `--keep-going --report <ローカルJSON>` は差を記録して残りも比較するが、不一致・未実施があれば終了コードは失敗になる。

## 同梱読み全件の実OS比較（2026-10-20）

Rust `e09eadf`、元版C#基準 `45e14f8`、Windows 11 build 26300で、同梱20,181読みと境界26入力の全20,207要求を比較した。4分割・バッチサイズ128、バッチ内で変換器を保持し、C# x64の実API応答を各Rust ABIと照合した。全候補と順序を比較し、候補の先頭だけで成功判定していない。

| 初回の全件比較 | x64 | x86 |
|---|---:|---:|
| 確認済み | 20,207 | 20,207 |
| 一致 | 20,203 | 20,205 |
| 不一致 | 4 | 2 |
| Microsoft IME利用 | 20,206 | 20,206 |
| Rustの規定内タイムアウト再試行 | 5 | 4 |

変換結果と文節は全件一致した。候補一覧の差は「ああいう」「あんどろいど」が両ABI、「emoji-boundary」「long:190」がx64。初回実行の終了コードは不一致により失敗で、記録を保持している。

並行照会終了後、この4入力だけC#参照から採り直した。毎回初期化する条件とバッチ内保持の条件で別々に比較し、両条件・両ABIとも4/4一致、Rustのタイムアウト再試行0回。差は再現しなかったが原因は未確定で、再照合で初回の失敗を上書きしない。全20,207要求が一度に一致した結果とはしない。

[公開集計・証跡ハッシュ](full-corpus-verification.json)は分割の重複・欠落・改変と個別結果を検査して生成した。初回の生応答はローカル `C:/opencode-rust/meltype-verification-20261020/os-shards/`、単独再照合は `dist/os-all-20261020/isolated-fresh/` と `isolated-shared/`。公開集計に生のOS候補は含めない。任意の読み・文脈・OS版・辞書版・学習状態や他OSの全応答を網羅するものではなく、速度・CPU・RAMは未再測定。
