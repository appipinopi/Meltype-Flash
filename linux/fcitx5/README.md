# fcitx5 adapter migration

<!-- meltype-flash-document-banner:start -->
<p align="center"><img src="../../assets/branding/readme/documentation.svg" alt="Meltype-FLASH — 稲妻のように、言葉の壁を駆け抜ける日本語入力。" width="1000"></p>
<!-- meltype-flash-document-banner:end -->

The default `MELTYPE_RUST_CORE=ON` build requires the Rust native library,
including `meltype_json_visit_utf8`, `meltype_fcitx_key` and
`meltype_fcitx_modifiers` and `meltype_fcitx_mode`. JSON bytes, escapes and numbers are parsed
in Rust. The C++ event adapter receives copied UTF-8 values for the existing
fcitx5 widgets; it does not parse JSON. Keycode/modifier translation and Japanese mode/role decisions also run in Rust. The SDK supplies Unicode
key-symbol conversion and raw modifier bits. The framework input-context wrapper,
editor application and widget integration still contain C++ and remain migration
work. This is not a standalone Rust fcitx5 addon yet.

Build on Linux with the fcitx5 development packages installed:

```sh
cargo build -p meltype-native --release --locked
cmake -S linux/fcitx5 -B linux/build/fcitx5-rust -DMELTYPE_RUST_CORE=ON -DMELTYPE_RUST_ABI_TESTS=ON
cmake --build linux/build/fcitx5-rust --parallel 2
python3 linux/fcitx5/test/json_bridge.py target/release/libmeltype_native.so
linux/build/fcitx5-rust/testkeymapping "$PWD/target/release/libmeltype_native.so"
```

The addon loads `libMeltypeNative.so` under `MELTYPE_DIR`, retaining the existing
frontend filename. The Cargo output is `libmeltype_native.so`; packaging must
copy it under the frontend filename. These commands do not install/activate an
input method. The Rust Linux bundle currently packages IBus, not this addon.

`MELTYPE_RUST_CORE=OFF` explicitly selects the legacy NativeAOT compatibility
reader in `compat/json_reader.hpp`. The old `linux/build.sh` selects that option
for its C# library. It is not included in the default Rust adapter. Existing
upstream history and GPL notices are retained.

The UTF-8 visitor validates the complete document before callbacks, rejects
malformed JSON/UTF-8 and unpaired surrogate escapes, and uses the last duplicate
object value. Embedded escaped NUL is length-delimited. Callback text is borrowed
only for the synchronous callback. A zero callback return cancels traversal, and
C++ exceptions are caught at the callback boundary. The Windows TSF compatibility
visitor retains its existing numeric and UTF-16 behavior separately.

The standalone test compiles the production C++ event adapter, loads the real
Rust library and compares 217 valid reference/core projections. It additionally
checks malformed-input rejection, unchanged output on failure, cancellation,
Unicode/NUL and duplicate keys. Temporary profiles are private; it does not run a
fcitx daemon or type into any application. The SDK addon builds and real fcitx5
input require separate evidence. CI builds both Rust and compatibility variants
and runs the standalone ABI test against its release Rust library.

The SDK key test compares the original virtual-key policy for 131,076 key
symbols and 512 modifier combinations through the real Rust ABI. It preserves
the former fcitx policy (including ignoring lock bits and Super2), independently
of the IBus frontend’s mapping. Japanese mode/role decisions run before ordinary key translation. Rust returns
a plan for the role key and target mode; the adapter keeps the existing commit,
editor application and mode-update order. The framework operations still contain C++; direct/preedit state is owned by
the Rust session registry. See the [SDK modifier definitions](https://github.com/fcitx/fcitx5/blob/master/src/lib/fcitx-utils/keysym.h)
and [raw flag accessor](https://github.com/fcitx/fcitx5/blob/master/src/lib/fcitx-utils/flags.h).

The key test also compares both Japanese/direct mode states for every symbol
(262,152 mode/role cases). A focused Rust fcitx ABI workflow builds both adapter
variants and runs the JSON/key/mode comparisons alongside the full-platform
workflow. Neither comparison registers an input method nor operates a daemon.

Rust/default contexts now use meltype_fcitx_create/destroy and native
direct/preedit state, including cleanup through generic meltype_destroy. The C++
input-context property retains only a Rust handle; legacy NativeAOT state is
kept behind its explicit compatibility build. The state ABI test uses private
profiles, real core mode transitions, stale/double destruction and 100 fresh
contexts. It does not create a daemon or activate an input method.

The [aeddba2 SDK run](https://github.com/appipinopi/Meltype/actions/runs/39036802956)
passed both adapter builds, 217 JSON projections, 131,076 SDK key symbols, 262,152
mode/role states and 512 modifier cases on fcitx5 5.1.7. The subsequent
[e377fa1 SDK run](https://github.com/appipinopi/Meltype/actions/runs/39037192682)
also passed both builds and the independent context/state ABI test. Its
[four-platform CI](https://github.com/appipinopi/Meltype/actions/runs/39037192728)
passed Windows, macOS, Linux and Android jobs. These checks do not prove complete
UI parity or actual desktop input-method activation.

Surrounding-text byte ranges and guarded deletion counts now come from Rust
(`meltype_fcitx_context_offsets` / `meltype_fcitx_delete_count`). The SDK bridge
passes UTF-8 text and character cursor positions; Rust verifies the expected
suffix and its UTF-16 deletion length, returning a character count for the SDK.
Invalid UTF-8/cursors are rejected, and failed range queries leave output
untouched. The old helpers live only in the explicit legacy build and reference
tests. The standalone comparison covers Japanese, supplementary characters,
embedded NUL, window limits and expected-text mismatches: 2,225 comparisons
passed locally on WSL with the actual Rust library. A range-loop copy warning
in the test was fixed before that successful run. SDK editor callbacks
and widgets remain C++ and still need migration.

The default adapter now requests panel operations from Rust through
`meltype_fcitx_view_utf8`: formatted preedit segments, selected-clause highlight,
visibility, suggestion/hint composition, candidate-list eligibility/page size,
items and selected index, followed by publish or hide. The full response is
validated and the operation list is built before the first callback. UTF-8 text
is length-delimited, including embedded NUL; returning zero stops callbacks.
C++ catches SDK exceptions at the callback boundary and hides the panel after a
failed traversal. SDK object allocation, text submission, labels, notifications
and input-context/editor bridges still use C++. The legacy build retains the
former `show()` implementation. The standalone ABI test compares Rust plans to
that policy, including wrong field types, missing views, candidate boundaries,
Unicode, cancellation and actual core replies. This establishes policy evidence,
not rendered pixels or real application input compatibility.

Editor response application now uses `meltype_fcitx_apply_utf8`. Rust owns the
commit loop, current-suffix query, hide/delete/commit ordering, mismatch-triggered
core reset and stopping later edits/view updates. The SDK callback fetches current
surrounding text separately for each deletion; it does not cache the context from
the first edit. Host failures stop traversal, and the C++ boundary hides the panel.
Panel planning completes before any editor mutation. The legacy build keeps the
original loop. A controlled editor compares both operation traces and final text
against that loop, including capabilities, cursor positions, Japanese/emoji/NUL,
multiple dependent edits, mismatches, malformed replies and callback failures.
This fixture does not establish real application input compatibility. SDK calls,
framework property/candidate objects and remaining key-event orchestration still
need migration.

Key-event dispatch, role-key fallback, direct-mode commit/update/label ordering,
candidate selection and focus/reset commit now run in Rust. The C++ SDK bridge
passes symbols/scalars/modifier state and provides synchronous capability/context
queries plus editor/widget operations. The default build no longer constructs a
C++ JSON projection; the former frontend orchestration is legacy-only. Context
buffers live through dispatch and preserve former c_str NUL truncation. No mutex
is held across host queries; failed queries and destruction during getters stop
dispatch. The event ABI test compares independent actual cores with the former
frontend sequence, building on separate SDK key/mode and old editor comparisons.
SDK classes/property lifetime, labels and framework callbacks remain C++.
