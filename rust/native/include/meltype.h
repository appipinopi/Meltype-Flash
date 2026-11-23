/* SPDX-License-Identifier: GPL-3.0-or-later
 * Copyright (C) 2026 appipinopi */
#ifndef MELTYPE_H
#define MELTYPE_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* All strings are NUL-terminated UTF-8. Free results with meltype_free.
 * A session is an opaque token: never dereference it. Serialize calls per
 * session. Callback functions must remain valid until all sessions are gone.
 * Callbacks return malloc-allocated strings; the engine calls free on them. */
typedef char *(*meltype_clauses_callback)(const char *, const char *);
typedef char *(*meltype_candidates_callback)(const char *);
typedef int32_t (*meltype_word_callback)(const char *);
/* Synchronous JSON value events: 0 null,1 bool(number),2 number,3 UTF16 string,
   4/5 array begin/end,6/7/8 object begin/key/end. Text is borrowed only during
   callback. Callback/code/owner stay valid and never throw; return zero to stop.
   Invalid input emits no events. Depth and UTF16 escape semantics match TSF.
   input is readable for length bytes. Returns 1 success,0 failure/cancel. */
typedef int32_t (*meltype_json_emit)(void *,uint32_t,double,const uint16_t *,uint32_t);
int32_t meltype_json_visit(const uint8_t *,uint32_t,void *,meltype_json_emit);
/* Read length UTF16 units; null permitted for empty input. UTF8 quoted JSON
   result belongs to meltype_free. Null result means invalid input/allocation. */
char *meltype_json_quote(const uint16_t *,uint32_t);
int32_t meltype_init(meltype_clauses_callback, meltype_candidates_callback, meltype_word_callback);
/* Optional one-clause conversion; call after meltype_init. Same ownership. */
int32_t meltype_set_converter(meltype_candidates_callback);
int32_t meltype_init_mozc(const char *, const char *);
void *meltype_create(void);
void *meltype_create_at(const char *directory);
char *meltype_control(void *, const char *request_json);
void meltype_destroy(void *);
char *meltype_handle_key(void *, int32_t vk, int32_t scalar, int32_t modifiers, const char *before, const char *after);
char *meltype_handle_key_up(void *, int32_t vk, int32_t scalar, int32_t modifiers);
uint64_t meltype_begin_async_conversion(void *, const char *reading, const char *context);
int32_t meltype_complete_async_conversion(void *, uint64_t id, const char *candidates_json);
int32_t meltype_cancel_async_conversion(void *);
int32_t meltype_timeout_async_conversion(void *, uint64_t now_ms);
int32_t meltype_apply_async_conversion(void *, const char *reading, const char *candidates_json);
int32_t meltype_async_enabled(void *);
uint64_t meltype_begin_async_from_session(void *);
int32_t meltype_async_pending(void *);
int32_t meltype_async_can_begin(void *);
int32_t meltype_async_ready(void *);
char *meltype_poll_async_conversion(void *);
char *meltype_commit(void *);
/* Record the displayed preedit after a host has committed it on focus loss.
 * The host must not apply the returned text again. No backward edit is made. */
char *meltype_commit_preedit(void *);
/* Undo requires a fresh caret snapshot and set_can_delete(1).
 * Hosts MUST compare edit.expect before applying deletion; count is UTF-16. */
char *meltype_undo_commit(void *, const char *before);
/* Verify result.selectionExpect against the current selection before applying
 * view. reading may be NULL; dictionary/history inference can fail safely. */
char *meltype_reconvert_selection(void *, const char *text, const char *reading);
#ifdef _WIN32
/* Read-only MSIME/WinRT comparison diagnostic on an uninitialized or STA
   calling thread. JSON {text:string,context?:string,trace?:bool}, text/context
   each <=8192 UTF-16 units. trace adds a candidateTimedOut boolean.
   Returns {available,converted,clauses,candidates}; no IME activation or document
   access. COM/WinRT ownership ends before return; free with meltype_free. */
char *meltype_windows_converter_probe(const char *request);
#endif
char *meltype_reset(void *);
char *meltype_select_candidate(void *, int32_t index);
char *meltype_select_prediction(void *, int32_t index);
char *meltype_role_key(void *, int32_t vk);
void meltype_set_direct(void *, int32_t);
/* Auto detection additionally requires fresh before text and can_delete(1). */
void meltype_set_direct_detection(void *, int32_t);
void meltype_set_code_input(void *, int32_t);
void meltype_set_private(void *, int32_t);
void meltype_set_can_delete(void *, int32_t);
char *meltype_version(void);
char *meltype_data_directory(void);
char *meltype_report_url(const char *);
void meltype_free(void *);
#ifdef _WIN32
/* Owned TSF COM references, released with IUnknown::Release, NOT meltype_free.
   guid is a pointer to a Windows GUID; unknown/null GUIDs return NULL. */
void *meltype_tsf_create_display_attribute(const void *guid);
void *meltype_tsf_create_display_attribute_enum(void);
/* TSF mode button: COM Release owns the returned interface. Call detach before
   destroying the context or callback code; callbacks run on the creating thread. */
void *meltype_tsf_create_langbar(void *context, int32_t (*read_mode)(void *), void (*toggle)(void *));
void meltype_tsf_langbar_update(void *button);
void meltype_tsf_langbar_detach(void *button);
/* Thread-affine opaque candidate-window token. show takes UTF-8 JSON fields
   candidates/notes/selected/suggestion/meaning/prediction/typed and a Windows RECT.
   Click callbacks may hide/destroy the token. Destroy on its creating thread. */
void *meltype_tsf_candidate_create(void *context, void (*select)(void *, int32_t, int32_t));
int32_t meltype_tsf_candidate_show(void *, const char *json, const void *anchor_rect);
/* TSF core-view projection and placement. Borrowed context/composition, valid
   read cookie and UTF-8 reply view. No COM calls for hidden views/stale tokens. */
int32_t meltype_tsf_candidate_show_tsf(void *token, void *context, void *composition, uint32_t cookie, const char *view);
/* Writes a RECT. Return 1 for TSF text geometry, 0 for caret/cursor fallback,
   -1 for invalid JSON/null output. Never edits the document. */
int32_t meltype_tsf_candidate_anchor(void *context, void *composition, uint32_t cookie, const char *view, void *rect);
void meltype_tsf_candidate_hide(void *);
int32_t meltype_tsf_candidate_visible(void *);
void meltype_tsf_candidate_destroy(void *);
/* COM class factory: GUIDs/out and HRESULT follow the Windows ABI. The shell
   callbacks construct the remaining text service and pin/unpin its DLL. */
int32_t meltype_tsf_get_class_object(const void *clsid, const void *iid, void **out,
    int32_t (*create)(const void *iid, void **out), void (*lock_shell)(int32_t));
/* Explicit regsvr32 operations; mutate COM/profile/category registration.
   path is the TSF shell DLL's UTF-16 path, length excludes a terminator. */
int32_t meltype_tsf_register_server(const uint16_t *path, uint32_t length);
int32_t meltype_tsf_unregister_server(void);
/* Borrowed ITfContext; transfers action_context ownership on every return path.
   destroy is required and called exactly once. Keep callbacks/code alive until
   destroy, including async requests. Use on the creating TSF thread. */
int32_t meltype_tsf_request_edit_session(void *context, uint32_t client, uint32_t flags,
    void *action_context, int32_t (*action)(void *, uint32_t), void (*destroy)(void *));
/* Borrowed TSF COM pointers and edit cookie. UTF-16 callback buffers are borrowed
   only during the callback. Surrounding limits: 4000 before / 20 after. */
int32_t meltype_tsf_read_surrounding(void *context, uint32_t cookie, void *receiver,
    void (*callback)(void *, const uint16_t *, uint32_t, const uint16_t *, uint32_t));
/* Returns 0 unless the range is collapsed and the exact UTF-16 prefix matches.
   Discard the range on failure; never write/delete after a failed check. */
int32_t meltype_tsf_extend_previous(void *range, uint32_t cookie, int32_t count,
    const uint16_t *expected, uint32_t length);
int32_t meltype_tsf_write_range(void *context, void *range, uint32_t cookie,
    const uint16_t *text, uint32_t length, int32_t keep_caret);
/* Start returns an owned ITfComposition. A NULL range uses the current selection.
   All inputs are borrowed COM pointers. Composition views are UTF-8 JSON with
   text/converting/clauses/selectedClause. End retains references across callbacks
   and does not consume the caller's owned COM references. */
void *meltype_tsf_start_composition(void *context, void *sink, void *range, uint32_t cookie);
int32_t meltype_tsf_update_composition(void *context, void *composition, uint32_t cookie,
    const char *view, uint32_t input_atom, uint32_t converted_atom, uint32_t target_atom);
int32_t meltype_tsf_end_composition(void *context, void *composition, uint32_t cookie);
void meltype_tsf_set_attribute(void *context, void *range, uint32_t cookie, uint32_t atom);
/* Synchronous batch; owner/callbacks must remain valid until return. Current
   returns a borrowed composition. Write borrows the range and UTF-16 text.
   Return 0 rejects the remaining batch; caller must reset the core session. */
int32_t meltype_tsf_apply_commits(void *context, uint32_t cookie, const char *commits,
    void *owner, void *(*current)(void *), int32_t (*write)(void *, void *, const uint16_t *, uint32_t));
/* Thread-affine opaque resume snapshot: capture only eligible unconverted input.
   Try consumes on success/failure, returns owned ITfComposition, and never inserts
   text. Destroy unused tokens on the creating thread. Matches compares context. */
void *meltype_tsf_capture_resume(void *context, void *composition, uint32_t cookie);
int32_t meltype_tsf_resume_matches(void *snapshot, void *context);
void *meltype_tsf_try_resume(void *snapshot, void *context, void *sink, uint32_t cookie);
/* Adopt the result before styling, so reentrant termination finds its owner.
   This call retains context/composition across host callbacks. */
void meltype_tsf_resume_attribute(void *context, void *composition, uint32_t cookie, uint32_t input_atom);
void meltype_tsf_destroy_resume(void *snapshot);
/* Thread-affine old-composition queue, max eight retained entries. add borrows
   interfaces and schedules a Rust edit session that pins owner through IUnknown.
   end deactivates/drains and requests synchronous cleanup; queued sessions can
   still finish after refusal. reset starts a new activation; destroy is final.
   All queued sessions must execute/release on their creating TSF thread. */
void *meltype_tsf_abandoned_create(void);
void meltype_tsf_abandoned_reset(void *queue);
void meltype_tsf_abandoned_destroy(void *queue);
int32_t meltype_tsf_abandoned_add(void *queue, void *context, void *composition, void *owner, uint32_t client);
int32_t meltype_tsf_abandoned_forget(void *queue, void *composition);
int32_t meltype_tsf_abandoned_end(void *queue, void *owner, uint32_t client);
void meltype_tsf_clear_composition_attribute(void *composition, uint32_t cookie);
/* Read metadata only: secret=1, private/no-learning=2, unreadable=4;
   property sources (Windows=1, app=2)<<8, unreadable step (QI=1,list=2,
   VARIANT=3)<<16. Missing metadata is unspecified; unreadable values are secret.
   Borrowed context/thread-manager pointers, valid cookie, creating TSF thread. */
uint32_t meltype_tsf_input_scope(void *context, uint32_t cookie);
int32_t meltype_tsf_context_disabled(void *context);
/* Read: 0/1, -1 on unavailable value, -2 for unsupported manager.
   Write: 1 on success, 0 on failure. */
int32_t meltype_tsf_read_open(void *thread_manager);
int32_t meltype_tsf_write_open(void *thread_manager, uint32_t client, int32_t open);
/* Thread-affine subscriptions: 0 thread events, 1 foreground keys, 2 mode
   (initially open), 3 top-context edits. Owner is thread manager except kind 3
   (document manager). Sink implements that event interface. Kind 3 requires
   context_out, returning an owned ITfContext on success; otherwise NULL.
   Unsubscribe releases the retained original source, not a fresh lookup. */
void *meltype_tsf_subscribe(uint32_t kind, void *owner, uint32_t client, void *sink, void **context_out);
void meltype_tsf_unsubscribe(void *token);
/* Synchronous edit-event callbacks. lifetime is a borrowed IUnknown retained
   across dispatch. current returns borrowed composition and writes its context.
   query/moved/commit implement owner actions; commit preserves the app caret.
   All callback code/state must remain valid until return; never throw. */
typedef struct MeltypeTsfEditCallbacks {
    void *owner;
    void *lifetime;
    int32_t (*take_own)(void *);
    void *(*current)(void *, void **context);
    void (*query)(void *, uint32_t cookie, void *context, int32_t changed);
    void (*moved)(void *, int32_t reset_english);
    void (*commit)(void *);
} MeltypeTsfEditCallbacks;
int32_t meltype_tsf_edit_notification(void *context, void *record, uint32_t cookie, const MeltypeTsfEditCallbacks *callbacks);
/* Key flags: modifier=1, Japanese toggle=2, Alt-grave toggle=4, composition
   Ctrl shortcut=8. Modifier bits: Shift=1,Ctrl=2,Alt=4,Win=8. Keyboard reads and
   translation use the caller's thread/layout; translation preserves dead keys. */
uint32_t meltype_tsf_key_flags(uint32_t key, uint64_t message, uint32_t modifiers);
uint32_t meltype_tsf_key_modifiers(void);
int32_t meltype_tsf_key_down(int32_t key);
uint32_t meltype_tsf_key_char(uint32_t key, uint64_t message);
typedef struct MeltypeTsfKeySnapshot {
    void *composition, *context;
    int32_t disabled, open, detection, english;
    uint32_t stale_tries, stale_vk;
    uint64_t stale_time;
} MeltypeTsfKeySnapshot;
typedef struct MeltypeTsfKeyCallbacks {
    void *owner, *lifetime;
    void (*read)(void *, MeltypeTsfKeySnapshot *);
    void (*update)(void *, uint32_t operation, uint64_t a, uint64_t b);
    void (*resume)(void *, void *context);
    int32_t (*commit_now)(void *, int32_t keep_caret);
    void (*commit_async)(void *, int32_t keep_caret);
    void (*abandon)(void *);
    int32_t (*ready)(void *);
    int32_t (*secret)(void *, uint32_t key, void *context, int32_t test);
    void (*pass_reason)(void *, uint32_t reason);
} MeltypeTsfKeyCallbacks;
/* Synchronous acceptance. Borrowed context and lifetime IUnknown on the TSF
   thread, writable UTF-16 output, callbacks/state/code valid until return.
   Callbacks never throw. update: 0 reset flags,1 secret while composing,
   2 clear English,3 stale tries/vk packed high/low32 in a and time b,
   4 checked=a. Pass: 0 clear,1 English,2 modifiers,3 disabled,4 continuing
   English,5 core unavailable,6 password. Secret: 0 ordinary,1 secret,-1 unknown. */
int32_t meltype_tsf_would_eat(void *context, uint32_t key, uint64_t message, int32_t test, const MeltypeTsfKeyCallbacks *callbacks, uint16_t *character);
/* Thread-affine key state: owns cached context and replay queue. Reset on a new
   activation; clear cache on focus/deactivation. Destroy on its creating thread. */
void *meltype_tsf_key_state_create(void);
void meltype_tsf_key_state_reset(void *);
void meltype_tsf_key_state_destroy(void *);
void meltype_tsf_key_cache_clear(void *);
/* Reader receives borrowed owner/context and writes private/no-learning. Owner
   lifetime is a borrowed IUnknown retained across the call. Reuse one test-key
   result for the same context/key within 200ms; never throw from callbacks. */
int32_t meltype_tsf_key_secret(void *state, void *context, uint32_t key, int32_t test,
    void *owner, void *lifetime, int32_t (*reader)(void *, void *, int32_t *private_field), int32_t *private_field);
int32_t meltype_tsf_is_reinjected(void *state, uint32_t key);
/* Return 1 sent, 2 loop suppressed, 0 send failed, -1 invalid. Sender borrows
   two Windows INPUTs; NULL uses SendInput, nonnull substitutes transport.
   Tests use substitutes and inject no keys. Optional error receives last-error.
   All pointers/callbacks valid until return on the token's TSF thread. */
struct tagINPUT;
int32_t meltype_tsf_reinject(void *state, uint32_t key, void *lifetime, void *owner,
    uint32_t (*sender)(void *, const struct tagINPUT *inputs, uint32_t count), uint32_t *error);
typedef struct MeltypeTsfKeyStale {uint32_t tries,key;uint64_t time;} MeltypeTsfKeyStale;
typedef struct MeltypeTsfKeyPassed {int32_t key,control;} MeltypeTsfKeyPassed;
void meltype_tsf_key_foreground(void *state, int32_t foreground);
void meltype_tsf_key_stale_read(void *state, MeltypeTsfKeyStale *out);
void meltype_tsf_key_stale_write(void *state, uint32_t tries, uint32_t key, uint64_t time);
void meltype_tsf_key_stale_clear(void *state);
/* mode 0 consumes pending key metadata once; mode 1 emits the recent key or
   selection/text movement (0/-1), with mouse-button precedence on selection. */
int32_t meltype_tsf_key_passed(void *state, int32_t mode, int32_t selection_changed, MeltypeTsfKeyPassed *out);
typedef struct MeltypeTsfKeyEntry {
    void *owner,*lifetime;
    int32_t (*would)(void *, void *context, uint32_t key, uint64_t message, int32_t test, uint16_t *character);
    void (*mark)(void *, uint32_t key, int32_t kind);
    int32_t (*composing)(void *);
    void (*toggle)(void *, void *context);
    void (*handle)(void *, void *context, uint32_t key, uint16_t character);
} MeltypeTsfKeyEntry;
/* kind: 0 test down,1 down,2 test up,3 up. Returns consumed. mark kind 1 is
   replay passthrough (clear English/mark moved), 2 ordinary passthrough (moved).
   Native state owns consumed keys, replay test/down pairing and passed metadata.
   Down callbacks/lifetime/context are borrowed on their TSF thread and must
   survive until return; never throw. Up events need no callbacks/context. */
int32_t meltype_tsf_key_event(void *state, uint32_t kind, void *context, uint32_t key, uint64_t message, const MeltypeTsfKeyEntry *entry);
typedef struct MeltypeTsfModeCallbacks {
    void *owner,*lifetime;
    uint32_t (*read)(void *); /* open=1, code application=2, code English=4 */
    int32_t (*ready)(void *);
    int32_t (*secret)(void *, uint32_t cookie, void *context);
    void (*set_open)(void *, int32_t open);
    int32_t (*request)(void *, const char *json); /* apply reply, return active && consumed */
    void (*notice)(void *, int32_t japanese);
} MeltypeTsfModeCallbacks;
/* Borrowed TSF interfaces, valid UTF-8 metadata {sid:string,process:string} and
   non-null callback functions on their TSF thread. All callbacks must not throw.
   Pins owner/context across focus, read-session and core callbacks. Toggle
   resolves focus only for a null context. Code-line null context always fails.
   A refused synchronous read uses the core's tracked line; password fields
   never read surrounding text or send toggle requests. */
int32_t meltype_tsf_toggle_mode(void *context, void *thread_manager, void *key_state, uint32_t client, const char *metadata, const MeltypeTsfModeCallbacks *callbacks);
int32_t meltype_tsf_code_line(void *context, void *key_state, uint32_t client, const char *metadata, const MeltypeTsfModeCallbacks *callbacks, int32_t japanese);
typedef struct MeltypeTsfModeQueryCallbacks {
    void *owner,*lifetime;
    uint32_t (*read)(void *); /* code application=2, server active=8 */
    int32_t (*secret)(void *, uint32_t cookie, void *context);
    int32_t (*request)(void *, const char *json, uint32_t timeout_ms, int32_t optional);
} MeltypeTsfModeQueryCallbacks;
/* Uses an existing read cookie; no new edit lock or server hello. Same lifetime,
   metadata and callback safety requirements as toggle_mode. Request receives
   100ms/optional=1 and must apply a successfully parsed reply. */
int32_t meltype_tsf_query_mode(void *context, uint32_t cookie, void *key_state, const char *metadata, const MeltypeTsfModeQueryCallbacks *callbacks, int32_t selection_changed);
typedef struct MeltypeTsfModeApplyCallbacks {
    void *owner,*lifetime;
    uint32_t (*read)(void *);
    void (*write)(void *, uint32_t flags);
    void (*notify)(void *, int32_t english);
} MeltypeTsfModeApplyCallbacks;
/* Reply bits active=1,code=2,English=4. Read/write bits code application=2,
   code English=4. On the TSF thread, with valid non-throwing callbacks. Pins the
   IUnknown owner, publishes both fields before notification, and notifies only
   when English changes. write must publish state without notifying. */
int32_t meltype_tsf_apply_mode(uint32_t reply, const MeltypeTsfModeApplyCallbacks *callbacks);
/* Windows-only private process transport. Lazy worker path must be absolute UTF-8.
   Returned strings use meltype_free. One request at a time per connection;
   concurrent/busy calls fail immediately. Timed-out/failed workers and their
   descendants are terminated and their session state is discarded. Mandatory
   failures back off 3s, optional failures permit immediate reconnection. */
void *meltype_tip_transport_create(const char *worker_path);
char *meltype_tip_transport_request(void *transport, const char *request, uint32_t timeout_ms, int32_t optional);
void meltype_tip_transport_destroy(void *transport);
typedef struct MeltypeTsfKeyEditCallbacks {
    void *owner,*lifetime;
    uint32_t (*read)(void *);
    void (*update)(void *, uint32_t operation, uint32_t value);
    int32_t (*edit)(void *, uint32_t operation, uint32_t cookie, void *context);
    int32_t (*request)(void *, const char *json, uint32_t *reply_flags);
    int32_t (*write)(void *, uint32_t cookie, void *context, void *range, uint16_t character);
    void (*log)(void *, uint32_t kind, int32_t code);
} MeltypeTsfKeyEditCallbacks;
/* TSF thread, live borrowed interfaces and non-null callbacks which never throw.
   metadata: UTF-8 {sid:string,process:string}. Uses a SYNC READWRITE session and
   pins context/owner. Refused or delayed sessions cannot execute key edits later.
   Snapshot bits: composing=1,open=2,secret checked=4,secret while composing=8,
     moved=16,private=32,failed=64,consumed=128.
   update operations: 0 set outcome (failed=1,consumed=2),1 own edit,2 clear moved,
     3 server flags (active=1,detection=2,English word=4),4 server off,
     5 clear last hello,6 set open,7 commit async preserving caret,8 reinject key.
   edit operations: 0 check secret (return bool),1 commit preserving caret,
     2 hide candidates/end composition,3 apply reply mode,4 apply reply edits.
   request stores callback-owned reply until return and returns success. Reply
     flags: active=1,detection=2,English word=4,switch=8,consumed=16,
     empty commits=32,null view=64. write borrows range and returns success.
   log kinds: secret=0,direct write failed=1,edit failed=2,core unavailable=3.
   Delayed SYNC session destruction retains lifetime but never calls owner state
   after this function returns; callback storage need only live until return.
   Explicit Convert over a selection sends {op:reconvert,text}. The captured
   range/text are checked after the request and mode application. Invalidated
   operations are consumed without reinjection; a newer composition is kept. */
int32_t meltype_tsf_handle_key(void *context, uint32_t client, void *key_state, uint32_t key, uint16_t character, const char *metadata, const MeltypeTsfKeyEditCallbacks *callbacks);
typedef struct MeltypeTsfCommitCallbacks {
    void *owner,*lifetime;
    void *(*current)(void *, void **composition_context, void **edit_sink_context);
    void (*update)(void *, uint32_t operation, uint32_t cookie, int32_t value);
    void (*request_apply)(void *, const char *json, uint32_t cookie, void *context, void *target);
} MeltypeTsfCommitCallbacks;
/* mode=0 sync,1 async,2 existing cookie/context. TSF-thread-only borrowed live
   interfaces and non-null callbacks that never throw. metadata={sid:string}.
   current returns borrowed interfaces; they are retained across scheduling.
   update: 0 own edit,1 preserve caret,2 end composition,3 hide candidates,
   4 clear own edit after sync. request_apply must recheck target/context before
   applying a core reply. Async owner/code must survive until callback execution
   or cancellation (lifetime IUnknown is pinned). Incorrectly deferred sync
   sessions are disabled after return. Stale scheduled targets do nothing. */
int32_t meltype_tsf_commit(uint32_t mode, void *context, uint32_t client, uint32_t cookie, const char *metadata, const MeltypeTsfCommitCallbacks *callbacks, int32_t preserve_caret);
/* Same callback lifetime/thread contract as commit. Schedules an async
   READWRITE session and rechecks both context and optional composition before
   selecting. Sets own edit, builds {op:select,sid,index,prediction?}, then invokes
   request_apply. No caret-policy/end/hide operations. Indices are passed to the
   core unchanged, including negative indices. Missing context is a no-op. */
int32_t meltype_tsf_select_candidate(uint32_t client, int32_t index, int32_t prediction, const char *metadata, const MeltypeTsfCommitCallbacks *callbacks);
typedef struct MeltypeTsfApplyReplyCallbacks {
    void *owner,*lifetime;
    void *(*current)(void *);
    int32_t (*write)(void *, void *range, const uint16_t *text, uint32_t length);
    void (*action)(void *, uint32_t operation, uint32_t cookie, void *context, const char *json, int32_t value);
} MeltypeTsfApplyReplyCallbacks;
/* TSF-thread-only live borrowed interfaces, valid READWRITE cookie and non-null
   callbacks which never throw. reply={active,commits,view,selectionExpect?};
   metadata={sid,direct}. Optional selectionExpect must be nonempty text matching
   the selected UTF-16 range, with no existing composition, before any edits.
   Then applies commits. Guard failure calls action 5 with reset JSON and stops.
   action: 0 set converting(value),1 update composition(view JSON),2 show
   candidates(view JSON),3 end composition,4 hide candidates,5 reset core and
   mark key failed/unconsumed. JSON/range/text are borrowed only during callbacks.
   Empty view clears preedit text only if active; inactive replies preserve it.
   Owner/context/composition/range refs survive reentrant callbacks. */
int32_t meltype_tsf_apply_reply(void *context, uint32_t cookie, const char *reply, const char *metadata, const MeltypeTsfApplyReplyCallbacks *callbacks);
typedef struct MeltypeTsfCompositionSlots {
    void *lifetime;
    void **composition,**context;
    void *sink;
} MeltypeTsfCompositionSlots;
/* TSF thread only. Distinct slots hold service-owned COM references and must
   remain writable while lifetime IUnknown is retained. Notifications may replace
   or clear slots using the same ownership convention. sink is a borrowed
   ITfCompositionSink. New composition/context references transfer into slots.
   Apply result: 0 start failed,1 update attempted,-1 invalid. Write flags:
   written=1,ending attempted=2; -1 invalid. End: 0 absent,1 attempted,-1 invalid.
   Ending clears only the captured pair, preserving replacement input even in
   the same context. Commit writing never ends a replacement created mid-write.
   View JSON and UTF-16 text/range are borrowed until return. */
int32_t meltype_tsf_apply_owned_composition(void *context, uint32_t cookie, const char *view, uint32_t input_atom, uint32_t converted_atom, uint32_t target_atom, const MeltypeTsfCompositionSlots *slots);
int32_t meltype_tsf_write_owned_commit(void *context, void *range, uint32_t cookie, const uint16_t *text, uint32_t length, int32_t preserve_caret, const MeltypeTsfCompositionSlots *slots);
int32_t meltype_tsf_end_owned_composition(uint32_t cookie, const MeltypeTsfCompositionSlots *slots);
typedef struct MeltypeTsfTerminationCallbacks {
    void *owner,*lifetime;
    void **composition,**context,**resume;
    void *abandoned;
    uint32_t (*read)(void *);
    void (*update)(void *, uint32_t operation);
    void (*request)(void *, const char *json);
} MeltypeTsfTerminationCallbacks;
/* TSF thread, live borrowed notification composition and valid cookie. Distinct
   slots hold owned COM references and a thread-affine recovery token, remaining
   writable while lifetime is held. Callbacks never throw; metadata={sid:string}.
   read: disabled=1,deactivated=2,converting=4. update: 0 hide,1 reset stale and
   clear converting,2 clear direct word/mark moved. request sends moved commit;
   its reply must not be applied (host already kept the text). Old identities are
   removed from abandoned cleanup without changing current input. */
int32_t meltype_tsf_composition_terminated(uint32_t cookie, void *composition, const char *metadata, const MeltypeTsfTerminationCallbacks *callbacks);
typedef struct MeltypeTsfRecoveryCallbacks {
    void *owner,*lifetime;
    void **composition,**context,**resume;
    void *sink;
    void *(*edit_context)(void *);
    void (*update)(void *, uint32_t operation, int32_t value);
    void (*request)(void *, const char *json);
} MeltypeTsfRecoveryCallbacks;
/* TSF thread, owned refs/token in distinct writable slots, borrowed context and
   sink; storage/code survive while lifetime is retained. Callbacks never throw.
   metadata={sid:string}. update 0 sets own edit(value),1 clear direct word/mark
   moved. request sends moved commit and must not apply its reply. Returns 0 no
   token or reentrant replacement,1 resumed,2 fallback requested,-1 invalid.
   Uses a SYNC READWRITE session; disables incorrectly deferred work on return.
   Publishes owned composition/context before styling, so notifications can
   release that pair and preserve a newly captured recovery token safely. */
int32_t meltype_tsf_resume_or_commit(void *context, uint32_t client, uint32_t input_atom, const char *metadata, const MeltypeTsfRecoveryCallbacks *callbacks);
typedef struct MeltypeTsfAbandonCallbacks {
    void *owner,*lifetime;
    void **composition,**context,**abandoned;
    void (*update)(void *, uint32_t operation);
    void (*request)(void *, const char *json);
} MeltypeTsfAbandonCallbacks;
/* TSF thread, distinct writable owned-reference/token slots. Lifetime pins
   callback storage/code. metadata={sid:string}; callbacks never throw.
   update 0 resets stale state,1 hides candidates,2 clears direct word/marks moved.
   request must ignore the commit reply. Detaches both references before old-field
   cleanup; preserves replacement input created during cleanup/UI notifications.
   Returns 1 notified,0 replacement preserved,-1 invalid. Does not insert text. */
int32_t meltype_tsf_abandon_composition(uint32_t client, const char *metadata, const MeltypeTsfAbandonCallbacks *callbacks);
typedef struct MeltypeTsfFocusCallbacks {
    void *owner,*lifetime;
    uint64_t *epoch;
    uint32_t (*read)(void *);
    int32_t (*commit)(void *, int32_t mode);
    void (*action)(void *, uint32_t operation, void *document);
} MeltypeTsfFocusCallbacks;
/* TSF thread, borrowed interfaces; writable epoch/storage/code valid while
   lifetime is retained. Callbacks never throw. read bits composition=1,
   disabled=2,deactivated=4. commit 0=sync/1=async, both keep application caret;
   nonzero return is success. action 0=resume/commit(null context),1=clear direct
   word/mark moved,2=clear field cache,3=hide,4=advise document,5=hello.
   Nested document-focus notification supersedes the older transition. */
int32_t meltype_tsf_document_focus(void *document, const MeltypeTsfFocusCallbacks *callbacks);
/* Foreground loss hides candidates then forgets consumed keys. Foreground gain
   is inert. Same owner/callback lifetime contract; key_state is a native token. */
int32_t meltype_tsf_foreground_focus(int32_t foreground, void *key_state, const MeltypeTsfFocusCallbacks *callbacks);
typedef struct MeltypeTsfShutdownCallbacks {
    void *owner,*lifetime;
    uint64_t *epoch;
    uint32_t *phase;
    void **composition,**context,**manager;
    void **mode_subscription,**key_subscription,**thread_subscription;
    uint32_t (*read)(void *);
    void (*action)(void *, uint32_t operation, uint32_t cookie, void *manager);
    void (*request)(void *, const char *json, uint32_t cookie, void *context, void *target);
} MeltypeTsfShutdownCallbacks;
/* TSF thread, distinct writable owned COM/token slots; epoch/phase/storage/code
   survive while lifetime is retained. Callbacks never throw. metadata={sid:string}.
   phase=0 active,1 stopping,2 stopped; activation resets phase and advances epoch.
   read bit0 disabled. action 0 mark deactivated,1 clear resume,2 clear field cache,
   3 end abandoned,4 close transport,5 drop candidates,6 unadvise edit,7 remove and
   detach language bar(manager),8 reset client,9 end composition(cookie).
   request applies commit reply only to matching context/target; ignores close.
   Deactivation/phase!=0 must make document-focus read report ended.
   Reentrant/repeated shutdown is inert; reactivation supersedes remaining cleanup.
   A delayed SYNC edit is disabled before return. Returns HRESULT. */
int32_t meltype_tsf_shutdown(uint32_t client, const char *metadata, const MeltypeTsfShutdownCallbacks *callbacks);
typedef struct MeltypeTsfStartupCallbacks {
    void *owner,*lifetime;
    uint64_t *epoch;
    uint32_t *phase;
    void **manager,**keys,**abandoned,**resume;
    void **subscriptions[3];
    uint32_t *atoms[3];
    void *sinks[3];
    void (*initialize)(void *, uint32_t client, int32_t disabled, int32_t own_process, const char *sid);
    void *(*action)(void *, uint32_t operation, int32_t value, void *document);
    void *(*category)(void *);
} MeltypeTsfStartupCallbacks;
/* TSF thread, borrowed manager/owner/sinks; distinct writable owned COM/token
   slots, atom slots and epoch/phase/storage/code valid while lifetime is held.
   Callbacks never throw. Subscription/sink order thread,key,mode. initialize
   resets shell primitive state and stores supplied client/disabled/own/sid.
   action 0 sets open(value),1 creates candidates,2 creates language bar and
   returns borrowed ITfLangBarItem,3 advises document,4 hello. category optionally
   returns owned ITfCategoryMgr; null callback uses native CoCreateInstance.
   Nested activation/shutdown supersedes old work; late tokens are unsubscribed.
   Does not register a COM server or input profile. Returns HRESULT. */
int32_t meltype_tsf_startup(void *manager, uint32_t client, uint32_t flags, const MeltypeTsfStartupCallbacks *callbacks);
typedef struct MeltypeTsfOpenChangeCallbacks {
    void *owner,*lifetime;
    uint64_t *epoch;
    uint32_t *phase;
    uint32_t (*current)(void *,void **manager);
    void (*update)(void *,uint32_t operation,int32_t value);
} MeltypeTsfOpenChangeCallbacks;
/* TSF thread, borrowed GUID/owner; writable mode epoch/phase and callback code /
   storage survive while lifetime is held. Callbacks never throw. current returns
   borrowed manager plus flags open=1,composing=2,ended=4. update 0 publishes
   open(value)/clears direct word,1 async commit without caret preservation,
   2 refresh bar,3 log mode. Nested notices supersede old work; phase!=0 skips.
   Activation must advance the mode epoch. Returns HRESULT. */
int32_t meltype_tsf_open_changed(const void *guid,const MeltypeTsfOpenChangeCallbacks *callbacks);
typedef struct MeltypeTsfCoreStateCallbacks {
    void *owner,*lifetime;
    const uint64_t *epoch;
    uint32_t (*read)(void *,uint64_t *last_hello);
    void (*update)(void *,uint32_t operation,uint64_t value);
    void (*hello)(void *);
    int32_t (*request)(void *,const char *,uint32_t *flags);
    uint64_t (*clock)(void *);
} MeltypeTsfCoreStateCallbacks;
/* TSF thread; epoch/code/storage valid while lifetime is retained. Callbacks
   never throw. read flags disabled=1,active=2,ended=4 plus last hello time.
   Optional clock substitutes Windows uptime. hello performs the refresh.
   Ready refreshes inactive state after strictly more than 2000ms.
   Hello process is readable UTF16 (null only for empty). request returns success
   plus active=1,detection=2,code=4. update 0 stores timestamp,1 publishes normalized
   flags/clears direct word,2 logs active(value). Stale epochs do not publish. */
int32_t meltype_tsf_core_ready(const MeltypeTsfCoreStateCallbacks *callbacks);
int32_t meltype_tsf_core_hello(const uint16_t *process,uint32_t length,const MeltypeTsfCoreStateCallbacks *callbacks);
typedef struct MeltypeTsfCoreRequestCallbacks {
    void *owner,*lifetime;
    const uint64_t *epoch;
    uint32_t (*read)(void *);
    int32_t (*transact)(void *,const uint8_t *,uint32_t,uint32_t timeout,int32_t optional,const uint8_t **reply,uint32_t *length);
    int32_t (*parse)(void *,const uint8_t *,uint32_t);
    void (*notice)(void *);
} MeltypeTsfCoreRequestCallbacks;
/* Same thread/lifetime/no-throw contract. Input readable for length bytes, null
   only for zero length. read flags ended=4,own process=8. transact forwards
   timeout/optional; returned reply bytes stay borrowed through parse. parse copies
   the reply into caller's value and returns success. notice logs parse failure.
   Own-process requests retain the original thread-id byte insertion rule.
   Stale/ended replies are skipped. No borrowed stack survives this call. */
int32_t meltype_tsf_core_request(const uint8_t *input,uint32_t length,uint32_t timeout,int32_t optional,const MeltypeTsfCoreRequestCallbacks *callbacks);
/* Native TSF COM owner. Final Release on creator thread destroys the adapter.
   Dispatch never unwinds or retains Call; callbacks remain valid until destroy.
   Interface lookup is borrowed and does not AddRef. */
typedef struct MeltypeTsfServiceCall {
    uint32_t op;void *a;void *b;void *c;uintptr_t n1;intptr_t n2;uint8_t is_up;
} MeltypeTsfServiceCall;
void *meltype_tsf_service_com_create(void *owner,int32_t (*dispatch)(void *,const MeltypeTsfServiceCall *),void (*destroy)(void *));
void *meltype_tsf_service_com_interface(void *root,const void *iid);
/* TSF-thread state allocation with upstream defaults. Opaque except to a shell
   that first checks layout version,size,alignment and every field offset.
   Destroy once, on its TSF thread, after aliases/callbacks/UI have ceased and
   lifecycle cleanup has released raw COM references and subscription tokens.
   Rust-owned key/resume/abandoned/SID/transport/UI state is released by destroy. */
void *meltype_tsf_service_state_create(void);
void meltype_tsf_service_state_destroy(void *);
/* Return required u32 count. Write metadata only for nonnull output with enough
   capacity: version,size,alignment, then private header field offsets. */
uint32_t meltype_tsf_service_state_layout(uint32_t *output,uint32_t capacity);
uint32_t meltype_tsf_service_state_live_count(void);
/* Same TSF thread, no concurrent use. SID is copied; getter borrows until set/
   destroy. Request returns owned meltype_free text; worker/request are UTF-8.
   Transport closes on explicit close and on state destruction. */
/* Reactivation defaults, no COM callbacks; preserves other fields/tokens.
   Null state/SID returns 0 with no change. Clear-resume detaches before release.
   State destruction also owns cleanup of key/resume/abandoned tokens. */
int32_t meltype_tsf_service_initialize(void *state,uint32_t client,int32_t disabled,int32_t own,const char *sid);
void meltype_tsf_service_clear_resume(void *state);
/* TSF-thread edit subscription replacement. Lifetime pins the service/state;
   document and edit sink are borrowed SDK interfaces. Null document detaches.
   Reentrant replacement/end invalidates and cleans up a late subscription. */
/* Native key acceptance/cache/event state. Commit mode0=sync,1=async, with keep
   caret. Handle receives a borrowed context/key/UTF-16 character. Notice follows
   mode-action codes plus op6=changed pass reason,op7=resumed. Lifetime pins adapters/code.
   Sync reads left pending remain unconfirmed (-1) and cannot execute later. */
int32_t meltype_tsf_service_key_secret(void *state,void *lifetime,void *context,uint32_t key,int32_t test,void *adapter,void (*notice)(void *,uint32_t,int32_t));
int32_t meltype_tsf_service_would_eat(void *state,void *lifetime,void *sink,const char *worker,void *context,uint32_t key,uint64_t message,int32_t test,void *adapter,int32_t (*commit)(void *,uint32_t,int32_t),void (*notice)(void *,uint32_t,int32_t),uint16_t *out);
int32_t meltype_tsf_service_key_event(void *state,void *lifetime,void *sink,const char *worker,uint32_t kind,void *context,uint32_t key,uint64_t message,void *adapter,int32_t (*commit)(void *,uint32_t,int32_t),void (*handle)(void *,void *,uint32_t,uint16_t),void (*notice)(void *,uint32_t,int32_t));
/* Native service commit/selection: state, SID, exchange, caret and ending remain
   native. mode0=sync,1=async,2=existing context/cookie. Worker path is copied;
   lifetime pins state/adapter/code until delayed execution or cancellation.
   Apply receives borrowed compatible JSON at a valid edit cookie, after target
   and context checks. Apply/notice never unwind or retain bytes. Notice reports
   invalid replies. Returns HRESULT. No concurrent calls on the service. */
int32_t meltype_tsf_service_commit(void *state,void *lifetime,const char *worker,uint32_t mode,void *context,uint32_t cookie,int32_t keep,void *adapter,int32_t (*apply)(void *,uint32_t,void *,const uint8_t *,uint32_t),void (*notice)(void *));
int32_t meltype_tsf_service_select_candidate(void *state,void *lifetime,const char *worker,int32_t index,int32_t prediction,void *adapter,int32_t (*apply)(void *,uint32_t,void *,const uint8_t *,uint32_t),void (*notice)(void *));
/* Native key editing: borrowed live service/root/context/composition sink on its
   TSF thread. Root pins state/adapter/code; retained SYNC work is invalidated at
   return. Apply borrows compatible reply JSON and returns nonzero on success.
   Optional sender replaces SendInput and borrows INPUT[count]; NULL uses real
   OS input. Notice0..3=key logs,4=invalid reply,5=English display,6=scope source,
   7=scope unreadable,8=reinjection guard,9=SendInput error. All callbacks must not
   unwind or retain arguments. Async commits independently retain real adapters. */
int32_t meltype_tsf_service_handle_key(void *state,void *lifetime,void *sink,const char *worker,void *context,uint32_t key,uint16_t ch,void *adapter,int32_t (*apply)(void *,uint32_t,void *,const uint8_t *,uint32_t),void (*notice)(void *,uint32_t,int32_t),uint32_t (*sender)(void *,const void *,uint32_t));
/* Apply raw compatible reply bytes through native service document/UI state.
   Live root/context/composition sink on its TSF thread, valid read/write cookie.
   Worker is trusted absolute UTF-8. Root pins state/notice through reentry.
   Notice reports invalid reset replies, never unwinds or retains arguments.
   Returns -1 invalid input,0 guarded application failure,1 applied. */
int32_t meltype_tsf_service_apply_reply(void *state,void *lifetime,void *sink,const char *worker,void *context,uint32_t cookie,const uint8_t *reply,uint32_t length,void *adapter,void (*notice)(void *));
/* Complete native shutdown: owns request mediation, matching commit application,
   close-reply validation and all service teardown. Live service/root/sink on its
   TSF thread; worker trusted absolute UTF-8. Root pins state/code through reentry.
   Notice op0=abandoned cleanup HRESULT,op1=invalid reply; never unwinds. Pending
   SYNC sessions are invalidated before stack adapters expire. Returns HRESULT. */
int32_t meltype_tsf_service_shutdown(void *state,void *lifetime,void *sink,const char *worker,void *adapter,void (*notice)(void *,uint32_t,int32_t));
/* Complete native key dispatch: kind0..3=test-down/down/test-up/up. Live service,
   root/context/composition sink on its TSF thread; worker trusted absolute UTF8.
   Root pins state/notice/sender code through reentry. Mode/key notices follow
   service_key_event/service_handle_key codes and never unwind. Optional sender
   substitutes SendInput (NULL uses OS). Async commits independently own worker,
   adapter and notices. No external commit/edit/reply callback is used. */
int32_t meltype_tsf_service_dispatch_key(void *state,void *lifetime,void *sink,const char *worker,uint32_t kind,void *context,uint32_t key,uint64_t message,void *adapter,void (*mode_notice)(void *,uint32_t,int32_t),void (*key_notice)(void *,uint32_t,int32_t),uint32_t (*sender)(void *,const void *,uint32_t));
/* Native notification follow-ups. Live service/root/sinks on the TSF thread;
   worker trusted absolute UTF8. Root pins state/code; callbacks never unwind.
   Notice uses mode-action codes plus7=resumed,10=caret-outside commit,11=open.
   Deferred commit failures report op1; async work owns adapter/notice/path.
   Focus event -1=document,0=lost,1=gained. Record may be null. */
int32_t meltype_tsf_service_focus_native(void *state,void *lifetime,void *sink,void *edit_sink,const char *worker,void *document,int32_t event,void *adapter,void (*notice)(void *,uint32_t,int32_t));
int32_t meltype_tsf_service_edited_native(void *state,void *lifetime,void *sink,const char *worker,void *context,void *record,uint32_t cookie,void *adapter,void (*notice)(void *,uint32_t,int32_t));
int32_t meltype_tsf_service_opened_native(void *state,void *lifetime,void *sink,const char *worker,const void *guid,void *adapter,void (*notice)(void *,uint32_t,int32_t));
/* Native activation with owned candidate/language-bar callbacks. sinks points to
   five borrowed interfaces: thread,key,compartment,edit,composition. Live root,
   manager on TSF thread. Worker copied into persistent UI adapters. No root/UI
   reference cycle; detach/destroy UI before freeing state/notice code. Activation
   notice0=initialized,1=hello,2=invalid; UI notices use mode-action codes. Category
   is an optional owned ITfCategoryMgr factory for controlled hosts. */
int32_t meltype_tsf_service_activate_native(void *state,void *lifetime,const char *worker,void *manager,uint32_t client,uint32_t flags,void *const *sinks,void *adapter,void (*notice)(void *,uint32_t,int32_t),void (*ui_notice)(void *,uint32_t,int32_t),void *(*category)(void *));
/* Route service COM calls. root is a live native service COM interface, state
   its storage, worker trusted UTF8; all on TSF thread. call follows the public
   service COM Call layout; root pins state/code through reentry. Notice callbacks
   follow mode,key,activation,shutdown codes and survive UI/pending work. Unknown
   op=E_NOTIMPL, invalid/null key output=E_INVALIDARG. */
int32_t meltype_tsf_service_route(void *state,void *root,const char *worker,const void *call,void *adapter,void (*mode_notice)(void *,uint32_t,int32_t),void (*key_notice)(void *,uint32_t,int32_t),void (*activation_notice)(void *,uint32_t,int32_t),void (*shutdown_notice)(void *,uint32_t,int32_t));
/* Create a native-owned service interface. iid/out follow QueryInterface ABI;
   worker trusted absolute UTF8 and copied. Called on its TSF thread. Optional
   lock(1)/lock(0) pins external callback code until state/UI/transport teardown.
   Adapter/notices remain valid until final Release; callbacks never unwind.
   Failure clears out and balances state/pin. OS must Deactivate active service
   subscriptions before final Release. Returns HRESULT and one owned interface. */
int32_t meltype_tsf_service_object_create(const char *worker,const void *iid,void **out,void *adapter,void (*mode_notice)(void *,uint32_t,int32_t),void (*key_notice)(void *,uint32_t,int32_t),void (*activation_notice)(void *,uint32_t,int32_t),void (*shutdown_notice)(void *,uint32_t,int32_t),void (*lock)(int32_t));
/* Service mode actions op0=toggle,op1=code line(value Japanese),op2=query(cookie,
   value selection-changed). Notice 0=hello,1=invalid JSON,2=English display,
   3=code-line Japanese,4=scope source,5=scope unreadable reason. Query retains
   its 100ms optional policy. TSF thread, lifetime pins state/notice code. */
int32_t meltype_tsf_service_mode_action(void *state,void *lifetime,const char *worker,void *context,uint32_t cookie,uint32_t op,int32_t value,void *adapter,void (*notice)(void *,uint32_t,int32_t));
/* Same lifetime/thread contract and notice ops4/5; invalid owner is secret. */
int32_t meltype_tsf_service_input_scope(void *state,void *lifetime,void *context,uint32_t cookie,void *adapter,void (*notice)(void *,uint32_t,int32_t));
/* Native host-termination/recovery state and reply-discarding commit traffic.
   Borrowed TSF interfaces/cookie; lifetime pins state/adapter/notice code. Worker
   is a trusted absolute UTF-8 path. Optional notice logs parse failure and never
   unwinds. Replies use the compatible parser and are never inserted. */
int32_t meltype_tsf_service_terminated(void *state,void *lifetime,const char *worker,uint32_t cookie,void *composition,void *adapter,void (*notice)(void *));
int32_t meltype_tsf_service_resume(void *state,void *lifetime,const char *worker,void *sink,void *context,void *adapter,void (*notice)(void *));
/* Same ignored-reply/lifetime contract; detaches uneditable old input and queues
   cleanup without changing its document. Returns 1=notified,0=replaced,-1=invalid. */
int32_t meltype_tsf_service_abandon(void *state,void *lifetime,const char *worker,void *adapter,void (*notice)(void *));
/* Native service composition slots, attributes, keep-caret and stale-key cleanup.
   Borrowed TSF interfaces/cookie; service IUnknown pins state through COM and
   post-end cleanup. UTF-8 view is NUL-terminated; UTF-16 text null only if empty. */
int32_t meltype_tsf_service_apply_composition(void *state,void *lifetime,void *sink,void *context,uint32_t cookie,const char *view);
int32_t meltype_tsf_service_write_commit(void *state,void *lifetime,void *sink,void *context,void *range,uint32_t cookie,const uint16_t *text,uint32_t length);
int32_t meltype_tsf_service_end_composition(void *state,void *lifetime,uint32_t cookie);
/* Service shutdown with native composition ending; same callback/slot contract
   as service_deactivate, with no end-operation adapter. */
int32_t meltype_tsf_service_stop(void *state,void *lifetime,void *context,void (*request)(void *,const char *,uint32_t,void *,void *),void (*notice)(void *,int32_t));
/* Native activation state/resources. Sinks points to four borrowed interfaces:
   thread,key,compartment,edit. Select/read/toggle/context must remain live until
   UI retirement. Notice ops initialized=0(disabled),hello=1(active),invalid JSON=2.
   Optional category factory returns owned ITfCategoryMgr (fixtures; null uses COM).
   Lifetime pins state/context/code on the TSF thread; no unwinding. */
int32_t meltype_tsf_service_activate(void *state,void *lifetime,const char *worker,void *manager,uint32_t client,uint32_t flags,void *const *sinks,void *context,void (*select)(void *,int32_t,int32_t),int32_t (*read)(void *),void (*toggle)(void *),void (*notice)(void *,uint32_t,int32_t),void *(*category)(void *));
/* Native service shutdown owns state/COM slots and resource cleanup. Request
   borrows UTF-8 JSON and applies only to matching context/target. End receives
   the valid edit cookie. Optional notice reports failed abandoned cleanup HRESULT.
   TSF thread; lifetime pins state/adapter/code; callbacks never unwind. */
int32_t meltype_tsf_service_deactivate(void *state,void *lifetime,void *context,void (*request)(void *,const char *,uint32_t,void *,void *),void (*end)(void *,uint32_t),void (*notice)(void *,int32_t));
/* Service-owned open/close notification state and language-bar update. Commit
   requests async commit without caret preservation. Optional notice gets mode.
   TSF thread; lifetime pins state/context/callback code; no unwinding. */
int32_t meltype_tsf_service_opened(void *state,void *lifetime,const void *guid,void *context,void (*commit)(void *),void (*notice)(void *,int32_t));
/* Native edit-notification state/actions. Cookie and TSF context/record are
   borrowed; null record preserves own-edit. Lifetime IUnknown pins state/adapter
   and callback code. Query/commit are synchronous adapters and never unwind. */
int32_t meltype_tsf_service_edited(void *state,void *lifetime,void *context,void *record,uint32_t cookie,void *adapter,void (*query)(void *,uint32_t,void *,int32_t),void (*commit)(void *));
/* Native focus actions. Event -1=document,0=foreground loss,1=foreground gain.
   Commit mode 0=sync keep caret,1=async keep caret; resume uses null context.
   Borrowed lifetime pins state/context/callbacks. Sink is required for a nonnull
   document; optional notice follows service_core. TSF thread, no unwinding. */
int32_t meltype_tsf_service_focus(void *state,const char *worker,void *lifetime,void *sink,void *document,int32_t event,void *context,int32_t (*commit)(void *,int32_t),void (*resume)(void *),void (*notice)(void *,uint32_t,int32_t));
/* Native request mediation with service state, thread metadata and owned transport.
   Reply bytes live only during callback. Zero callback result reports parse failure;
   notice is optional. Service IUnknown pins state/context/code through callbacks.
   Input is length-delimited UTF-8 without embedded NUL (null only for length zero). */
int32_t meltype_tsf_service_exchange(void *state,const char *worker,void *lifetime,const uint8_t *input,uint32_t length,uint32_t timeout,int32_t optional,void *context,int32_t (*reply)(void *,const uint8_t *,uint32_t),void (*notice)(void *));
/* Native readiness/hello, owned transport, UTF-16 process identity and response
   publication. Notice op0=active,op1=invalid JSON. Optional clock is a host fixture.
   TSF-thread lifetime pins state/context; worker is a trusted absolute UTF-8 path. */
int32_t meltype_tsf_service_core(void *state,const char *worker,void *lifetime,void *context,void (*notice)(void *,uint32_t,int32_t),uint64_t (*clock)(void *),int32_t explicit_hello);
/* Mode flags open=1,code app=2,code English=4,server active=8; TSF thread.
   Write-open captures manager/client before COM. Apply-mode pins lifetime and
   publishes normalized code flags before optional synchronous notice and UI. */
uint32_t meltype_tsf_service_mode(const void *state);
int32_t meltype_tsf_service_write_open(void *state,int32_t open);
int32_t meltype_tsf_service_apply_mode(void *state,uint32_t reply,void *lifetime,void *context,void (*notice)(void *,int32_t));
int32_t meltype_tsf_service_edit_sink(void *state,void *document,void *lifetime,void *sink);
int32_t meltype_tsf_service_set_sid(void *state,const char *sid);
const char *meltype_tsf_service_sid(const void *state);
char *meltype_tsf_service_request(void *state,const char *worker,const char *request,uint32_t timeout,int32_t optional);
void meltype_tsf_service_close(void *state);
/* TSF-thread service owns returned bar; borrowed until replacement/removal.
   Keep state/context alive across create (old sink release may reenter).
   Remove detaches before querying the optional borrowed TSF manager. */
void *meltype_tsf_service_create_langbar(void *state,void *owner,int32_t (*read)(void *),void (*toggle)(void *));
void meltype_tsf_service_update_langbar(void *state);
void meltype_tsf_service_remove_langbar(void *state,void *manager);
/* Same TSF-thread state/context lifetime as create_langbar. Service owns token.
   Show/hide/drop snapshot or detach the token before reentrant COM/window calls. */
void *meltype_tsf_service_create_candidates(void *state,void *owner,void (*select)(void *,int32_t,int32_t));
int32_t meltype_tsf_service_show_candidates(void *state,void *context,void *composition,uint32_t cookie,const char *json);
void meltype_tsf_service_hide_candidates(void *state);
void meltype_tsf_service_drop_candidates(void *state);
/* UTF-16 host executable basename, .exe removed; required capacity includes NUL.
   Null/short output is untouched; zero means acquisition failure. */
uint32_t meltype_tsf_process_name(uint16_t *out, uint32_t capacity);
#endif
#ifdef __cplusplus
}
#endif
#endif
