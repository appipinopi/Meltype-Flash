# 元版の書き間違い候補との比較

<!-- meltype-flash-document-banner:start -->
<p align="center"><img src="../../../assets/branding/readme/documentation.svg" alt="Meltype-FLASH — 稲妻のように、言葉の壁を駆け抜ける日本語入力。" width="1000"></p>
<!-- meltype-flash-document-banner:end -->

変更していない元の`MisspellingDictionary.Find`を実行し、正しい形への置換全文か、候補なしをJSONLへ採取します。元版のUTF-16範囲と`CompositionText.ToKatakana`で置換を組み立て、Rustの探索結果と比較します。

```powershell
dotnet run --project tools/parity/misspellings/Misspellings.csproj -- tools/parity/misspellings/cases.json > misspelling-reference.jsonl
python rust/native/tests/misspelling_reference.py misspelling-reference.jsonl
cargo test -p meltype-core --test upstream_misspellings --locked
```

同梱辞書の全ての誤記・正記を単独、ASCIIの前後、絵文字の前後に置き、長いURL・ASCII・結合文字・空入力に、濁点除去・促音の挿入・長音の除去／挿入・結合文字の並び・誤記との隣接を加えた2281入力です。元版が正常に返した2144入力は候補の全文と候補なしが一致します。137入力では元版の曖昧探索がサロゲートペアの途中を切り、FormD正規化で`ArgumentException`になります。これらは期待値から削除せず、例外名も固定します。Rustでは全2281入力を実行してパニックしないことを検査し、元版の137例外を正常応答の一致件数へ加えません。

固定期待値は`rust/core/tests/data/upstream-misspellings.json`で、元ソース・辞書・ホストのSHA-256と基準コミットを含みます。Windows CIでも元版の596結果を再採取して比較します。先行の例外終了を含むログはローカル`dist/misspellings-original-*.jsonl`へ保持しています。

RustのASCII探索省略は、読み込み済み辞書の誤記と正規化した形にASCIIだけで一致し得るキーがない場合に限ります。辞書内容から条件を導き、ASCIIの誤記や正規化キーが含まれる場合は通常の探索を使います。日本語を含む読みの探索順・誤記優先・正記内の抑止条件は変更していません。

この比較は同梱の書き間違い辞書だけを対象とし、ユーザー辞書の追加・画面・全候補源・全設定や入力メソッド全体の一致を証明しません。性能は[Windows共通Coreの固定測定](../../performance/windows/README.md)で別に確認します。

重なる部分文字列の正規化を、各文字の正規化結果の連結で再利用します。残る文字の結合クラスが全て0の場合だけこの経路を使い、それ以外は部分文字列全体を正規化します。元の長い候補を優先する順序と、正しい形に当たったときに短い候補を止める条件を維持します。最初の2182入力は元の期待値が不変なことも採取時に確認しました。

## 元版のUnicode分類

元版のShapeはUTF-16のcharごとにNonSpacingMark（Mn）だけを取り除きます。Rust側の旧処理はMark全体を取り除いていたため、SpacingCombiningMark（Mc）やEnclosingMark（Me）まで誤字候補から消していました。`ःブレスレット`で元版は候補なし、修正前Rustは結合文字を消した候補を出すことを回帰で確認しました。修正後はMnだけを除き、補助平面の文字も元版のchar列の扱いに合わせて保持します。

```powershell
dotnet run --project tools/parity/misspellings/Misspellings.csproj -- --categories > unicode-mark-reference.json
dotnet run --no-build --project tools/parity/misspellings/Misspellings.csproj -- --shapes tools/parity/misspellings/shape-cases.json > unicode-shape-reference.jsonl
python tools/parity/misspellings/generate-marks.py --categories unicode-mark-reference.json --shapes unicode-shape-reference.jsonl --check
```

`upstream-unicode-marks.json`に元版を実行した.NETランタイム情報、BMP全65,536値のビットマップと212範囲、正規化112例を保持します。範囲表はRustソースへ生成するため、製品の実行時に.NETや新しいICU依存は必要ありません。`--check`を外すと固定データと表を再生成します。CIでは比較だけを行い、ランタイムの分類が変わったときに自動で期待値を受け入れません。

濁点のようなMn、占有結合文字、囲み結合文字、結合クラス0のMn、ハングルの声調記号、補助平面の音楽用結合文字・異体字セレクターを含めます。文字ごとの正規化結果に非0の結合クラスが残る場合は、部分文字列全体の正規化へ戻します。これは分類と誤字探索の照合であり、全Unicode文字列の表示や全入力操作の一致を証明するものではありません。修正前の失敗は`dist/mn-parity-before-fix.log`へ保持しています。

この分類修正後は全186テストとRust 1.99のワークスペース／TSF-server Clippyが成功。最終Releaseのx64／x86で`misspelling_marks_smoke.py`の文字入力→候補表示8ケースと、既存ABI・文節伸縮20入力列・選択再変換28操作・学習ワーカーも成功しました。ログは`dist/mn-*.log`へ保存。今回の分類修正後の速度は未測定です。
