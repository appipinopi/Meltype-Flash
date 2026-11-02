# Rust macOS key policy validation

<!-- meltype-flash-document-banner:start -->
<p align="center"><img src="../../../assets/branding/readme/documentation.svg" alt="Meltype-flash — 稲妻のように、言葉の壁を駆け抜ける日本語入力。" width="1000"></p>
<!-- meltype-flash-document-banner:end -->

`bash mac/test/key-policy/run.sh /absolute/path/libmeltype_native.dylib`
requires macOS and its Swift/Carbon/AppKit SDK. It builds a temporary owned app,
loads the actual Rust library through the production NativeCore initializer,
and uses production KeyMapping/NativeKeyPolicy with the former SDK implementation
as an independent reference. It compares all UInt16 key codes, a Unicode/control/
punctuation corpus, and modifier combinations including device/ignored bits.
Key-comparison NSEvents are constructed only in the test process. The AppKit
loop fixture posts an application-defined wake event to its own app after stop;
no key event is posted and no input source is registered or activated. The
converter fixture is test-only and is not shipped with the product. This does
not prove actual IMK input or UI parity.

Rust packaging enables MELTYPE_RUST_CORE=1 in mac/build.sh. Explicit legacy
NativeAOT builds retain the original Swift mapping. OS lowercase normalization
and Unicode extraction stay at the SDK boundary, preserving system behavior.
The Rust build rejects a missing key ABI without switching to the legacy mapper.
The focused Rust macOS SDK workflow also compiles the complete Swift adapter,
including its real converter dependency, separately from the SDK fixture.

The same SDK host also compares the Rust editor-range bridge with the former
InputController NSRange arithmetic and Swift canonical String equality. It covers
UTF-16 deletion lengths, marked/selected caret choice, missing expected strings,
20/4000-unit context windows, overflow rejection, composed/decomposed characters,
Hangul, NUL and compatibility-character distinctions. The existing controlled
IMK event/boundary tools accept --rust-core to exercise product Rust paths; their
explicit source lists include the new bridge files for legacy builds as well.

Rust's macOS-specific handle-key ABI also preserves upstream external-scalar
semantics: invalid scalars leave composition unchanged; combining marks commit
without automatic spacing (Latin accents preserve unchosen ASCII raw text), and
supplementary scalars commit before the original event passes through. Other
frontends keep their full-scalar input path. Existing Swift/IMK boundary tests
check this actual production entry point. A portable ABI fixture reproduces the
eight original Swift external-text cases without registering an input source.

Real-converter mode registers the production callbacks before exercising the
IMK adapter. It compares Space and Tab commits with the pinned converter's actual
clause result; prediction candidates are disabled in this fixture so Tab tests
conversion rather than prediction selection. Other settings remain explicit in
the private profile. Candidate Tab/Shift+Tab navigation must not refetch the
backend. macOS external candidates retain the registered converter's order
rather than being displaced by bundled dictionary/context candidates.

The OS English spelling callback is now resolved by Rust before acquiring the
session lock. Resolved evidence has a thread-local scope with restoration on
nested calls/unwind; no host callback runs from dictionary evaluation. Rust caches
positive and negative checks for five minutes (4096 entries), invalidates the
cache on callback registration, and rejects results if the provider was replaced
or the session was destroyed while the callback ran. Private/direct/disabled/
manual/kana paths do not request OS evidence. Swift Rust builds keep only the
NSSpellChecker OS call; the cache and check counter are native. The portable real
ABI test covers classification, commit/Undo, cache behavior, callback replacement,
reentrant session access and destruction. The real-converter SDK host requires
an actual NSSpellChecker invocation through the registered product callback.
This is evidence integration, not proof of full upstream detector parity.

The Rust response-application ABI now owns the edit loop and publishes the view
only after successful edits. Its Swift sink supplies live marked/selected ranges,
UTF-16 substrings, attributed insertion and the existing view renderer. Substring
buffers are explicitly owned through the next callback. Each dependent edit
queries the current client, not a cached initial range; mismatch stops later edits
and rendering. No core/session lock spans SDK callbacks. The old loop remains in
the explicit legacy build. Independent Foundation replay compares traces and final
documents, while controlled IMK tests exercise dependent replacement and mismatch.
Portable ABI tests cover each host-failure point and malformed-response rejection
before any callback. Objective-C exception recovery, actual external client input
and moving the remaining rendering/OS bridge to Rust still require work.

Composition decoration is now planned by Rust: UTF-16 clause offsets, raw/
converted/selected style, valid nonempty attribute ranges, and cursor-at-end
publication. The Swift sink maps styles to actual Carbon marks and sends the
attributed string through IMK; candidate and suggestion widgets still use Swift.
The independent SDK reference compares the old plan across empty/mismatched
clauses, selection boundaries, combining text, NUL and supplementary characters.
The controlled product IMK host verifies attributed UTF-16 payload/selection and
clear. A portable ABI test ensures callback failures stop before publication and
malformed replies do not start callbacks. This does not establish pixel parity.

Candidate update orchestration now runs in Rust. It requests list rebuild only
when contents differ by Swift-compatible canonical equality or the window is
hidden, asks whether the direction selector exists, performs incremental moves,
then records the selected index and requests meaning scheduling. Zero/one-item
lists clear/hide. Swift supplies the actual IMKCandidates calls and always ends
selection-notification suppression, including a failed traversal. The old update
and move loops remain only in legacy builds. The independent SDK host compares
list/selection traces across visibility, content, index and selector support;
portable ABI tests stop at each callback failure and reject malformed input.
Actual candidate-window pixels and complete IMK input still need verification,
along with other remaining direct OS integration.

Meaning reservation keys, duplicate scheduling, canonical matching, delay policy
(1500 ms) and session cleanup now live in Rust. Rust schedules the libdispatch main queue. Swift checks the original session
token/key with Rust, then Rust queries window visibility and sends the annotation
through Objective-C. Clear/hide invalidates the current key; core
session destruction removes it and stale callbacks cannot match a new session.
The A->B->A key behavior deliberately matches the former Swift equality policy.
SDK and portable ABI tests compare original key rules, empty versus absent text,
NUL, independent sessions, malformed UTF-16 and repeated destruction/recreation.
The SDK host checks actual delayed delivery on the main queue, one-shot callback
lifetime, and annotation delivery to a controlled Objective-C object. It checks
the actual IMKCandidates selectors, including Swift update() mapping to the
Objective-C updateCandidates selector. Candidate-window pixels remain unverified.

The temporary app also creates an actual private IMKServer and IMKCandidates
through the Rust initializers and compares panel type, selection keys, automatic
dismissal and visibility with the SDK initializer. No input source is registered.
The AppKit fixture calls the original named Services registration through Rust,
then runs the real NSApplication event loop, delivers a native timer, operates
the private SDK panel, and stops/wakes its own app. A watchdog bounds the loop.
Background loop/registration calls and invalid service names are rejected. The
SDK Services function returns void: the fixture proves the registration call,
not delivery of an installed service shortcut from another application.


The mode Services receiver itself is now an Objective-C class implemented in Rust.
The SDK host invokes its actual changeInputMode:userData:error: implementation
through the production Swift bridge. It exercises all four identifiers, exact
legacy errors, inactive/dead targets, identity-checked old deactivation, weak pure
Swift targets, reentrant deactivation, owner disposal during the source callback,
first-call background rejection and a retained receiver after owner destruction.
A private pasteboard remains unchanged. The Carbon source query is compared with
the SDK current source without selecting or registering any input source. The
remaining Swift callbacks perform mode selection and indicator display; direct
receiver invocation does not prove OS delivery of an installed Services shortcut.


Suggestion panels are now created/updated/hidden through Rust AppKit calls. The
SDK host compares twenty actual panels with the former Swift implementation,
including font, UTF-16 payload, label/content/window geometry, style, screen-edge
placement, long/empty/combining/NUL/emoji text, negative caret width and reuse.
Invalid caret/missing selection hides the panel; background calls are rejected.
These are actual AppKit property comparisons, not screenshot or full app input
verification. The Swift boundary still obtains the IMK selection and caret.


Mode-indicator view generation and layout now live in Rust. A test-only copy of
the former Swift implementation compares six complete AppKit view trees and ten
layout frames. The comparison includes effect material/blending/state, font and
label frames, selected accent colors, layer radii, accessibility radio roles,
labels/values, compact/central caption and caret validation. Deferred presentation, panel management and the one-second NSTimer now live in
Rust too. The lifecycle fixture compares actual panel settings with the former
Swift panel and checks queue supersession, hide-before-presentation, preference
queries at presentation time, timer replacement, panel reuse, missing screens,
and destruction while pending/visible. Both queue and timer target hold only
weak owners. Swift provides the preference read callback and bridge entry point. Full-screen visual parity
and actual editor input are not established by these property comparisons.


The mode/preference bridge uses Rust-owned NSUserDefaults calls with the existing
MacSelectedInputMode and MacShowModeIndicator schema. A private UUID-named suite
checks twenty bool/action comparisons, absent defaults, persistence and restoration
through new selection objects, per-instance cached modes, and invalid persisted
values without rewriting them. The private suite is removed after the fixture.
The fixture additionally launches two fresh copies of its owned executable in
preference-only mode. The parent persists mode/indicator values; child one reads
and updates them, child two reads the updated values and updates again, and the
parent verifies the result while its cached selection stays unchanged. Children
use only the UUID suite and finish within a 30-second termination bound. They do
not create an IMK server or register an input source. Restarting an installed
input method and using it in real editors remain separate integration checks.


The Rust input menu is compared with the former Swift construction for all eight
direct/code/indicator combinations. Actual NSMenu items are compared for title,
order, separator, check state, key-equivalent/modifiers, and explicit versus nil
targets. Five explicitly targeted actions are dispatched only to an owned probe;
folder/report actions with nil targets are inspected without invoking them. The
fixture also checks autorelease ownership and background-thread rejection. It
does not prove presentation or action delivery from the installed OS input menu.


macOSの辞書ブリッジはplistの読込とテキストの統合をRustで行います。
SDK試験は旧Swift実装と比較し、通常の辞書の6形状をXML・バイナリ各形式で12件、
NULを含む元の辞書の6形状をバイナリ形式で6件、計18件を検証します。
新しいFoundationがXML中のNULを拒否するため、NULはバイナリで保持を確認します。
文字列の正規化・除外、Foundationのエラー、32件の統合（結合文字・CRLF・NULを含む）、
別々の専用フォルダーへの繰り返し取込も旧Swiftと比較します。
The production import path now calls Foundation source/read/create/atomic NSData write
operations from Rust too. The IO fixture checks nested Unicode paths, no-op file
mtime, seven failures against the former Swift errors/content, and five UTF-8 decoding cases (BOM, malformed sequences and NUL). Atomic NSData writes preserve the UTF-8 bytes without a converting NSString initializer. A bare word is
a valid OpenStep plist, so malformed-input cases use broken syntax. The open/alert
UI and dictionary-change notification remain separate migration work. No personal dictionary is
read or modified. Portable ABI coverage checks actual merge/malformed requests.


Dictionary IO UTF-8 decoding is checked against the actual original Swift SDK
file initializer, including BOM preservation, invalid-sequence error 259 and
embedded NUL. Newer Swift Foundation source branches can have a different BOM
policy; a source branch alone is not evidence for the SDK/runtime under test.

Verified with both jobs in [8ddae69 macOS SDK CI](https://github.com/appipinopi/Meltype/actions/runs/39103797071).


Dictionary chooser/result workflow
---------------------------------
The Rust production path allocates/configures `NSOpenPanel`, runs it, resolves
its URL before asking the host for the current data directory, imports the
file, posts the original distributed notification (including zero additions),
and runs `NSAlert` with the original success/error text. Swift only commits
composition before calling Rust and supplies the current directory afterward.

`fixtures/DictionaryDialog.swift` compares nine paths with a copy of the former
Swift workflow. Scoped Objective-C intercepts replace the two blocking modal
calls, selected URL and distributed post; actual SDK objects and defaults,
short-circuit/callback ordering, alert text/style, notification parameters and
private destination bytes are compared. No external notification is sent by
these comparisons. An additional owned `NSAlert` modal loop is acknowledged
through its actual default button. After intercept restoration, another case
cancels a real owned `NSOpenPanel` without selecting any file. Both modal tests
have a process watchdog. Background-thread presentation must reject before
creating UI or querying the directory. These cases do not prove OS-wide
notification delivery, an accepted real file chooser, installed IME behavior,
or complete visual parity. Both jobs in [9eb73ed SDK CI](https://github.com/appipinopi/Meltype/actions/runs/39104520853) passed; the saved keys log contains both real modal-loop results and the nine comparisons.


Input-source normalization and registration
-------------------------------------------
Rust now owns the `--register-input-source` workflow: repair the existing
HIToolbox preference array, preserve its original backup key, register the
bundle with Carbon, enumerate all input sources and enable only the exact base
and `.Japanese` identifiers, stopping at the first OSStatus failure. It does
not select an input source. The original prefix-based removal rule during
repair is intentionally preserved. Unknown rows and values survive without JSON
conversion. An already-normalized array leaves the prior backup untouched.

The private Swift SDK fixture compares 324 arrays with the former normalization
and checks eleven stored-value/backup/idempotence cases in unique defaults suites.
It never writes `com.apple.HIToolbox`. A background registration call must return
before touching that suite or Carbon. The Rust fake API checks registration and
enable failures, exact IDs, skipped sources and list release on every exit.
The header-only C fixture checks seven Rust FFI declarations and scalar widths
against the installed SDK; it is never linked into the production library and
never executes a Carbon operation. Actual OS input-source registration/enable
and installed IME input remain unverified. Both jobs in [5a36cf9 SDK CI](https://github.com/appipinopi/Meltype/actions/runs/39105438244) passed, including the 324 comparisons, eleven private settings cases and header ABI checks. The separate `RegisterInputSource` executable still uses Swift.
