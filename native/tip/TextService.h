// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 lnkiai
//
// Meltype IME のテキストサービス本体。キーを Meltype.exe に渡し、返ってきた結果
// (確定する文字・未確定の文字・候補) を入力欄に反映する。

#pragma once

#include <wrl/client.h>

#include <memory>
#include <vector>

#include "CandidateWindow.h"
#include "Globals.h"
#include "Json.h"
#include "Pipe.h"
#include "RustServiceState.h"

namespace meltype {

class LangBarButton;

#ifdef MELTYPE_RUST_CORE
#define MELTYPE_SERVICE_OVERRIDE
#else
#define MELTYPE_SERVICE_OVERRIDE override
#endif
class TextService final
#ifndef MELTYPE_RUST_CORE
                        : public ITfTextInputProcessorEx,
                          public ITfThreadMgrEventSink,
                          public ITfKeyEventSink,
                          public ITfCompositionSink,
                          public ITfDisplayAttributeProvider,
                          public ITfCompartmentEventSink,
                          public ITfTextEditSink
#endif
{
public:
    TextService();
    template<class T> T* ServiceInterface() const {
#ifdef MELTYPE_RUST_CORE
        return static_cast<T*>(RustServiceComInterface(comEntry_,__uuidof(T)));
#else
        return static_cast<T*>(const_cast<TextService*>(this));
#endif
    }

    // IUnknown
    STDMETHODIMP QueryInterface(REFIID riid, void** ppv) MELTYPE_SERVICE_OVERRIDE;
    STDMETHODIMP_(ULONG) AddRef() MELTYPE_SERVICE_OVERRIDE;
    STDMETHODIMP_(ULONG) Release() MELTYPE_SERVICE_OVERRIDE;

    // ITfTextInputProcessor(Ex)
    STDMETHODIMP Activate(ITfThreadMgr* threadMgr, TfClientId clientId) MELTYPE_SERVICE_OVERRIDE;
    STDMETHODIMP ActivateEx(ITfThreadMgr* threadMgr, TfClientId clientId, DWORD flags) MELTYPE_SERVICE_OVERRIDE;
    STDMETHODIMP Deactivate() MELTYPE_SERVICE_OVERRIDE;

    // ITfThreadMgrEventSink
#ifndef MELTYPE_RUST_CORE
    STDMETHODIMP OnInitDocumentMgr(ITfDocumentMgr*) MELTYPE_SERVICE_OVERRIDE { return S_OK; }
#endif
#ifndef MELTYPE_RUST_CORE
    STDMETHODIMP OnUninitDocumentMgr(ITfDocumentMgr*) MELTYPE_SERVICE_OVERRIDE { return S_OK; }
#endif
    STDMETHODIMP OnSetFocus(ITfDocumentMgr* focus, ITfDocumentMgr* previous) MELTYPE_SERVICE_OVERRIDE;
#ifndef MELTYPE_RUST_CORE
    STDMETHODIMP OnPushContext(ITfContext*) MELTYPE_SERVICE_OVERRIDE { return S_OK; }
#endif
#ifndef MELTYPE_RUST_CORE
    STDMETHODIMP OnPopContext(ITfContext*) MELTYPE_SERVICE_OVERRIDE { return S_OK; }
#endif

    // ITfKeyEventSink
    STDMETHODIMP OnSetFocus(BOOL foreground) MELTYPE_SERVICE_OVERRIDE;
    STDMETHODIMP OnTestKeyDown(ITfContext* context, WPARAM wParam, LPARAM lParam, BOOL* eaten) MELTYPE_SERVICE_OVERRIDE;
    STDMETHODIMP OnKeyDown(ITfContext* context, WPARAM wParam, LPARAM lParam, BOOL* eaten) MELTYPE_SERVICE_OVERRIDE;
    STDMETHODIMP OnTestKeyUp(ITfContext* context, WPARAM wParam, LPARAM lParam, BOOL* eaten) MELTYPE_SERVICE_OVERRIDE;
    STDMETHODIMP OnKeyUp(ITfContext* context, WPARAM wParam, LPARAM lParam, BOOL* eaten) MELTYPE_SERVICE_OVERRIDE;
#ifndef MELTYPE_RUST_CORE
    STDMETHODIMP OnPreservedKey(ITfContext*, REFGUID, BOOL* eaten) MELTYPE_SERVICE_OVERRIDE {
        *eaten = FALSE;
        return S_OK;
    }
#endif

    // ITfCompositionSink
    STDMETHODIMP OnCompositionTerminated(TfEditCookie ec, ITfComposition* composition) MELTYPE_SERVICE_OVERRIDE;

#ifndef MELTYPE_RUST_CORE
    // ITfDisplayAttributeProvider
    STDMETHODIMP EnumDisplayAttributeInfo(IEnumTfDisplayAttributeInfo** ppEnum) MELTYPE_SERVICE_OVERRIDE;
    STDMETHODIMP GetDisplayAttributeInfo(REFGUID guid, ITfDisplayAttributeInfo** ppInfo) MELTYPE_SERVICE_OVERRIDE;

#endif

    // ITfCompartmentEventSink
    STDMETHODIMP OnChange(REFGUID guid) MELTYPE_SERVICE_OVERRIDE;

    // ITfTextEditSink
    STDMETHODIMP OnEndEdit(ITfContext* context, TfEditCookie ec, ITfEditRecord* record) MELTYPE_SERVICE_OVERRIDE;

    // 入力モード (日本語 = 開いている / 英数 = 閉じている)。タスクバーのボタンから使う。
    // コードエディターのコードの行では、IME は開いたまま、次に打つキーが英数のままアプリに通るので「A」と出す
    bool IsOpen() const {
#ifdef MELTYPE_RUST_CORE
        return (RustServiceMode(state_.value)&5)==1;
#else
        return open_ && !codeEnglish_;
#endif
    }
    void SetOpen(bool open);
    // 半角/全角・タスクバーのボタン: 表示している 日本語 ⇔ 英数 を切り替える (context が null ならフォーカスのある入力欄)
    void ToggleInputMode(ITfContext* context);

private:
    ~TextService();

    // test: OnTestKeyDown から (入力欄の種類を読んだ結果を、続く OnKeyDown で使うために覚える)
#ifndef MELTYPE_RUST_CORE
    bool WouldEat(ITfContext* context, UINT vk, LPARAM lParam, wchar_t& ch, bool test);
#endif
    void ForgetSecretField();
    bool IsReinjected(UINT vk);
    bool ServerReady();
    void Hello();
#ifndef MELTYPE_RUST_CORE
    void HandleKey(ITfContext* context, UINT vk, wchar_t ch);
#endif
    // コードの行を日本語にする / 英数に戻すよう Meltype.exe に頼む。そうしたなら true
    bool SetCodeLine(ITfContext* context, bool japanese);
    // キャレットが動いたときに、次に打つキーが英数か Meltype.exe に聞く (コードエディターのとき。編集の通知の中で呼ぶ)
    void QueryMode(TfEditCookie ec, ITfContext* context, bool selectionChanged);
    // アプリに通したキーを覚える (QueryMode で伝える)
#ifndef MELTYPE_RUST_CORE
    void RememberPassed(UINT vk);
#endif
    // まだ伝えていない、アプリに通したキー (要求に足す JSON。無ければ空)
    std::string PendingPassed();
    // 応答の code / english を入力モードの表示に反映する
    void ApplyMode(const JsonValue& reply);
    // 要求を送って応答を読む。失敗したら false (optional は PipeClient::Transact と同じ)
    bool Request(const std::string& json, JsonValue& reply, DWORD timeoutMs = 800, bool optional = false);
    // 応答を入力欄に反映する (編集セッションの中で呼ぶ)
#ifndef MELTYPE_RUST_CORE
    void Apply(TfEditCookie ec, ITfContext* context, const JsonValue& reply);
#endif
    // range の前の count 文字が expect (前に確定した文字) なら、range をそこまで広げる。違えば false
#ifndef MELTYPE_RUST_CORE
    bool ExtendOverPrevious(TfEditCookie ec, ITfRange* range, LONG count, const JsonValue& expect);
#endif
    // range を text で置き換えて確定する (アプリによっては変換中の文字としてでないと入らないので、いったん変換中にする)
    bool WriteCommit(TfEditCookie ec, ITfContext* context, ITfRange* range, const std::wstring& text);
    void ApplyComposition(TfEditCookie ec, ITfContext* context, const JsonValue& view);
    void HideCandidates();
    void ShowCandidates(TfEditCookie ec, ITfContext* context, const JsonValue& view);
    void EndComposition(TfEditCookie ec);
    void SetAttribute(TfEditCookie ec, ITfContext* context, ITfRange* range, TfGuidAtom atom);
    // 未確定の内容を確定する (フォーカスが移った・英数にしたなど)。非同期の編集セッションで行う
    void CommitAsync(bool keepAppCaret);
#ifdef MELTYPE_RUST_CORE
    // Non-blocking conversion: Space/Convert starts a worker request and a
    // thread timer applies the result when it is ready.
    bool BeginAsync(ITfContext* context);
    void PollAsync();
    void CancelAsync();
    static VOID CALLBACK AsyncTimerProc(HWND, UINT, UINT_PTR, DWORD);
    bool RequestRaw(const std::string& json, std::string& raw, JsonValue& reply, DWORD timeoutMs = 800);
    UINT_PTR asyncTimer_ = 0;
    bool asyncActive_ = false;
    ITfContext* asyncContext_ = nullptr;
#endif
    // 同期で確定する。できなければ false。keepAppCaret: キャレットを動かさず、前の語も確定し直さない
    bool CommitNow(bool keepAppCaret = false);
    // 確定できない前の入力欄の変換中の文字を手放す
    void AbandonComposition();
    // アプリが確定した打ちかけの文字を、変換中に戻す (戻せなければ確定したことにする)。context が null なら確定したことにする
    void ResumeOrCommit(ITfContext* context);
    void ClearResume();
    bool HasResume() const;
    void CommitInSession(TfEditCookie ec, ITfContext* context, bool keepAppCaret);
    void SelectCandidate(int index, bool prediction);
    void Reinject(UINT vk);
    void SurroundingText(TfEditCookie ec, ITfContext* context, std::wstring& before, std::wstring& after);
    bool ContextDisabled(ITfContext* context);
    void AdviseTextEditSink(ITfDocumentMgr* documentMgr);
    bool IsSecretField(TfEditCookie ec, ITfContext* context);
    // 入力欄の種類を読む読み取りの編集セッション。1 = パスワード欄、0 = 違う、-1 = 読めない (キーの編集セッションの中で読む)
#ifndef MELTYPE_RUST_CORE
    int ReadSecretField(ITfContext* context);
#endif
#ifdef MELTYPE_RUST_CORE
    HRESULT DispatchKey(unsigned int,ITfContext*,UINT,LPARAM,BOOL*);
    HRESULT ServiceCommit(unsigned int,ITfContext*,TfEditCookie,bool);
    static int __cdecl ApplyServiceReply(void*,unsigned int,void*,const unsigned char*,unsigned int);
#endif
    void ResetStaleCommit();
    std::string SessionId() const;
    std::string CommitRequest(bool moved) const;
    void UnadviseTextEditSink();
    void UpdateLangBar();

#ifdef MELTYPE_RUST_CORE
    // First member: freed after UI/data members; pins the shell until then.
    RustServiceStateStorage state_;
    void* comEntry_ = nullptr; // Borrowed from the Rust COM owner that deletes this adapter.
    ITfThreadMgr*& threadMgr_ = state_.value->manager;
    TfClientId& clientId_ = state_.value->client;
    void*& threadSubscription_ = state_.value->thread_subscription;
    void*& keySubscription_ = state_.value->key_subscription;
    void*& modeSubscription_ = state_.value->mode_subscription;
    void*& editSubscription_ = state_.value->edit_subscription;
    ITfContext*& editSinkContext_ = state_.value->edit_context;
    ITfComposition*& composition_ = state_.value->composition;
    ITfContext*& compositionContext_ = state_.value->composition_context;
    TfGuidAtom& atomInput_ = state_.value->atom_input;
    TfGuidAtom& atomConverted_ = state_.value->atom_converted;
    TfGuidAtom& atomTarget_ = state_.value->atom_target;
    void*& keyState_ = state_.value->keys;
    void*& resume_ = state_.value->resume;
    void*& abandoned_ = state_.value->abandoned;
    ULONGLONG& lastHello_ = state_.value->last_hello;
    unsigned long long& focusEpoch_ = state_.value->focus_epoch;
    unsigned long long& modeEpoch_ = state_.value->mode_epoch;
    unsigned int& shutdownPhase_ = state_.value->shutdown_phase;
    bool& serverActive_ = state_.value->server_active;
    bool& directDetection_ = state_.value->direct_detection;
    bool& directEnglishWord_ = state_.value->direct_word;
    bool& open_ = state_.value->open;
    bool& codeApp_ = state_.value->code_app;
    bool& codeEnglish_ = state_.value->code_english;
    bool& disabled_ = state_.value->disabled;
    bool& ownProcess_ = state_.value->own_process;
    bool& lastConverting_ = state_.value->last_converting;
    bool& caretMoved_ = state_.value->caret_moved;
    bool& ownEdit_ = state_.value->own_edit;
    bool& secretChecked_ = state_.value->secret_checked;
    bool& secretWhileComposing_ = state_.value->secret_composing;
    bool& privateField_ = state_.value->private_field;
    bool& keepAppCaret_ = state_.value->keep_caret;
    bool& deactivated_ = state_.value->deactivated;
    bool& keyFailed_ = state_.value->key_failed;
    bool& consumed_ = state_.value->consumed;
    const wchar_t*& scopeSource_ = state_.value->scope_source;
    const wchar_t*& passReason_ = state_.value->pass_reason;
    HRESULT ServiceFocus(ITfDocumentMgr*,int);
    bool ForgetAbandoned(ITfComposition*);
    void EndAbandonedNow();
#else
    LONG refs_ = 1;
    ITfThreadMgr* threadMgr_ = nullptr;
    TfClientId clientId_ = TF_CLIENTID_NULL;
#ifdef MELTYPE_RUST_CORE
    void* threadSubscription_ = nullptr;
    void* keySubscription_ = nullptr;
    void* modeSubscription_ = nullptr;
    void* editSubscription_ = nullptr;
#else
    DWORD threadMgrCookie_ = TF_INVALID_COOKIE;
    DWORD compartmentCookie_ = TF_INVALID_COOKIE;
    DWORD editSinkCookie_ = TF_INVALID_COOKIE;
#endif
    ITfContext* editSinkContext_ = nullptr;

    ITfComposition* composition_ = nullptr;
    ITfContext* compositionContext_ = nullptr;

    TfGuidAtom atomInput_ = TF_INVALID_GUIDATOM;
    TfGuidAtom atomConverted_ = TF_INVALID_GUIDATOM;
    TfGuidAtom atomTarget_ = TF_INVALID_GUIDATOM;

    PipeClient pipe_;
    std::string sid_;
    bool serverActive_ = false;
    bool directDetection_ = false; // 英数状態でもローマ字検知が有効 (Meltype.exe の応答)
    bool directEnglishWord_ = false;
    bool fullWidthSpace_ = false;  // 設定「スペースを全角で入力」(Meltype.exe の応答の fullWidthSpace。日本語入力のときは Space も受け取る)
    ULONGLONG lastHello_ = 0;
    bool open_ = true;
    bool codeApp_ = false;         // コードエディター・ターミナル (Meltype.exe の応答の code)
    bool codeEnglish_ = false;     // 次に打つキーがコードの行なので英数のまま通る (Meltype.exe の応答の english)
#ifndef MELTYPE_RUST_CORE
    UINT passedVk_ = 0;            // 最後にアプリに通したキー (キャレットが動いたときに Meltype.exe に伝える)
    bool passedControl_ = false;   // そのとき Ctrl を押していたか
    ULONGLONG passedTime_ = 0;
    bool passedPending_ = false;   // そのキーをまだ Meltype.exe に伝えていない
#endif
    bool disabled_ = false;
    bool ownProcess_ = false;      // Meltype.exe 自身の中で動いている (要求にスレッドを付ける)  // Meltype.exe 自身の中では動かない (自分のサーバーを待って固まるため)
#ifdef MELTYPE_RUST_CORE
    void* keyState_ = nullptr;
    unsigned long long focusEpoch_ = 0;
    unsigned long long modeEpoch_ = 0;
    unsigned int shutdownPhase_ = 0;
    HRESULT ServiceFocus(ITfDocumentMgr*,int);
#else
    bool eatenDown_[256] = {};
    struct Reinjected {
        UINT vk = 0;
        ULONGLONG time = 0;
    };
#endif
#ifdef MELTYPE_RUST_CORE
    void* resume_ = nullptr;  // Rust owns the saved range/context/text.
#else
    Microsoft::WRL::ComPtr<ITfRange> resumeRange_;      // アプリが確定した打ちかけの文字 (次のキーで変換中に戻す)
    Microsoft::WRL::ComPtr<ITfContext> resumeContext_;
    std::wstring resumeText_;
#endif
    bool lastConverting_ = false;  // 最後に置いた変換中の文字が、変換した状態か (打っている途中なら false)
#ifndef MELTYPE_RUST_CORE
    int staleCommitTries_ = 0;     // 前の入力欄に残った変換中の文字を確定しようとした回数 (打鍵ごとに 1 回)
    UINT staleVk_ = 0;             // OnTestKeyDown で数えたキーと時刻 (続く OnKeyDown で数え直さない)
    ULONGLONG staleTime_ = 0;
#endif
#ifdef MELTYPE_RUST_CORE
    void* abandoned_ = nullptr;  // Rust owns retained and queued old compositions.
#else
    struct Abandoned {
        Microsoft::WRL::ComPtr<ITfComposition> composition;
        Microsoft::WRL::ComPtr<ITfContext> context;
    };
    std::vector<Abandoned> abandoned_;  // 手放した前の入力欄の変換中の文字 (あとで下線を消して終える)
#endif
    // 手放した変換中の文字の一覧から外す。一覧にあれば true
    bool ForgetAbandoned(ITfComposition* composition);
    // 終わるときに、手放した変換中の文字をその場で終える
    void EndAbandonedNow();
#ifndef MELTYPE_RUST_CORE
    UINT reinjectGuardVk_ = 0;     // 同じキーを続けて送り直した回数 (送り直しが止まらなくなるのを防ぐ)
    ULONGLONG reinjectGuardTime_ = 0;
    int reinjectGuardCount_ = 0;
    bool markerSeen_ = false;      // このアプリには送り直しの印が届く
    Reinjected reinjected_[4];     // 送り直して、まだ届いていないキー (それぞれ 1 回だけ素通しする)
#endif
#ifndef MELTYPE_RUST_CORE
    UINT passVk_ = 0;              // OnTestKeyDown で素通しにしたキー
    ULONGLONG passTime_ = 0;
#endif
    bool caretMoved_ = false;      // 前に確定してから、キャレットが動いたかもしれない (次のキーで Meltype.exe に伝える)
    bool ownEdit_ = false;         // 自分の編集セッションが終わった通知を、キャレットが動いたことにしない
    bool secretChecked_ = false;   // このキーで、入力欄の種類 (パスワード欄でないこと) を確かめた
    bool secretWhileComposing_ = false;  // 変換中にパスワード欄になったが、受け取る前に確定できなかった (キーの処理の中で確定して送り直す)
#ifndef MELTYPE_RUST_CORE
    UINT secretVk_ = 0;            // OnTestKeyDown で入力欄の種類を読んだキー・入力欄・時刻・結果 (続く OnKeyDown で 1 回だけ使う)
    Microsoft::WRL::ComPtr<ITfContext> secretContext_;
    ULONGLONG secretTime_ = 0;
    int secretResult_ = -1;
    bool secretPrivate_ = false;
    std::wstring lastSecretScopes_;  // 調べるとき用: 前にパスワードとみなした入力欄の種類 (同じならログに書かない)
#endif
    bool privateField_ = false;    // 前に入力欄の種類を読んだとき、入力を覚えないように求める入力欄 (IS_PRIVATE) だった
    const wchar_t* scopeSource_ = nullptr;  // 入力欄の種類を前に読んだところ (ログ用)
    bool keepAppCaret_ = false;    // 確定したあと、キャレットを動かさない (クリック・別の入力欄で確定したとき)
    bool deactivated_ = false;     // 終わった後は Meltype.exe とやり取りしない
    const wchar_t* passReason_ = nullptr;  // 最後にキーを通した理由 (ログに同じ理由を続けて書かない)
    bool keyFailed_ = false;
    bool consumed_ = true;
#endif

#ifndef MELTYPE_RUST_CORE
    std::unique_ptr<CandidateWindow> candidates_;
    LangBarButton* langBar_ = nullptr;
#endif
};

#undef MELTYPE_SERVICE_OVERRIDE

// ITfEditSession を関数で作る
HRESULT RunEditSession(ITfContext* context, TfClientId clientId, DWORD flags, std::function<HRESULT(TfEditCookie)> action);

}  // namespace meltype
