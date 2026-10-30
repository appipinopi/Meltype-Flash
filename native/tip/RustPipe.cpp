// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
// Replaces Pipe.cpp for Rust preview builds. A private Rust worker owns core
// sessions; the Rust DLL enforces deadlines and kills timed-out worker trees.
#include "Pipe.h"
#include "DisplayAttributes.h"
#include "LangBar.h"
#include "TextService.h"
#include "CandidateWindow.h"
#include <new>
#include <cstring>

namespace meltype {
namespace {
struct RustApi {
    HMODULE module = nullptr;
    std::string worker;
    void* (__cdecl *create)(const char*) = nullptr;
    void (__cdecl *destroy)(void*) = nullptr;
    char* (__cdecl *request)(void*, const char*,unsigned int,int) = nullptr;
    void (__cdecl *freeText)(void*) = nullptr;
    void* (__cdecl *displayAttribute)(const GUID*) = nullptr;
    void* (__cdecl *displayAttributeEnum)() = nullptr;
    HRESULT (__cdecl *classObject)(const GUID*,const GUID*,void**,HRESULT (__cdecl *)(const GUID*,void**),void (__cdecl *)(int)) = nullptr;
    HRESULT (__cdecl *registerServer)(const wchar_t*,unsigned int) = nullptr;
    HRESULT (__cdecl *unregisterServer)() = nullptr;
    HRESULT (__cdecl *editSession)(void*,unsigned int,unsigned int,void*,HRESULT (__cdecl *)(void*,unsigned int),void (__cdecl *)(void*)) = nullptr;
    int (__cdecl *surrounding)(void*,unsigned int,void*,void (__cdecl *)(void*,const unsigned short*,unsigned int,const unsigned short*,unsigned int)) = nullptr;
    int (__cdecl *writeRange)(void*,void*,unsigned int,const unsigned short*,unsigned int,int) = nullptr;
    void* (__cdecl *startComposition)(void*,void*,void*,unsigned int) = nullptr;
    int (__cdecl *updateComposition)(void*,void*,unsigned int,const char*,unsigned int,unsigned int,unsigned int) = nullptr;
    int (__cdecl *endComposition)(void*,void*,unsigned int) = nullptr;
    void (__cdecl *setAttribute)(void*,void*,unsigned int,unsigned int) = nullptr;
    int (__cdecl *applyCommits)(void*,unsigned int,const char*,void*,void* (__cdecl *)(void*),int (__cdecl *)(void*,void*,const unsigned short*,unsigned int)) = nullptr;
    void* (__cdecl *captureResume)(void*,void*,unsigned int) = nullptr;
    int (__cdecl *resumeMatches)(void*,void*) = nullptr;
    void* (__cdecl *tryResume)(void*,void*,void*,unsigned int) = nullptr;
    void (__cdecl *resumeAttribute)(void*,void*,unsigned int,unsigned int) = nullptr;
    void (__cdecl *destroyResume)(void*) = nullptr;
    void* (__cdecl *abandonedCreate)() = nullptr;
    void (__cdecl *abandonedReset)(void*) = nullptr;
    void (__cdecl *abandonedDestroy)(void*) = nullptr;
    HRESULT (__cdecl *abandonedAdd)(void*,void*,void*,void*,unsigned int) = nullptr;
    int (__cdecl *abandonedForget)(void*,void*) = nullptr;
    HRESULT (__cdecl *abandonedEnd)(void*,void*,unsigned int) = nullptr;
    void (__cdecl *clearCompositionAttribute)(void*,unsigned int) = nullptr;
    unsigned int (__cdecl *inputScope)(void*,unsigned int) = nullptr;
    int (__cdecl *readOpen)(void*) = nullptr;
    int (__cdecl *writeOpen)(void*,unsigned int,int) = nullptr;
    int (__cdecl *contextDisabled)(void*) = nullptr;
    void* (__cdecl *subscribe)(unsigned int,void*,unsigned int,void*,void**) = nullptr;
    void (__cdecl *unsubscribe)(void*) = nullptr;
    unsigned int (__cdecl *keyFlags)(unsigned int,unsigned long long,unsigned int) = nullptr;
    unsigned int (__cdecl *keyModifiers)() = nullptr;
    int (__cdecl *keyDown)(int) = nullptr;
    unsigned int (__cdecl *keyChar)(unsigned int,unsigned long long) = nullptr;
    void* (__cdecl *keyStateCreate)() = nullptr;
    void (__cdecl *keyStateReset)(void*) = nullptr;
    void (__cdecl *keyStateDestroy)(void*) = nullptr;
    void (__cdecl *keyCacheClear)(void*) = nullptr;
    int (__cdecl *isReinjected)(void*,unsigned int) = nullptr;
    int (__cdecl *reinject)(void*,unsigned int,void*,void*,unsigned int (__cdecl *)(void*,const INPUT*,unsigned int),unsigned int*) = nullptr;
    void (__cdecl *keyForeground)(void*,int) = nullptr;
    void (__cdecl *keyStaleRead)(void*,RustKeyStale*) = nullptr;
    void (__cdecl *keyStaleWrite)(void*,unsigned int,unsigned int,unsigned long long) = nullptr;
    void (__cdecl *keyStaleClear)(void*) = nullptr;
    int (__cdecl *keyPassed)(void*,int,int,RustKeyPassed*) = nullptr;
    int (__cdecl *jsonVisit)(const unsigned char*,unsigned int,void*,RustJsonEmit) = nullptr;
    char* (__cdecl *jsonQuote)(const unsigned short*,unsigned int) = nullptr;
    int (__cdecl *coreReady)(const RustCoreStateCallbacks*) = nullptr;
    int (__cdecl *coreHello)(const unsigned short*,unsigned int,const RustCoreStateCallbacks*) = nullptr;
    RustServiceState* (__cdecl *serviceCreate)() = nullptr;
    void (__cdecl *serviceDestroy)(RustServiceState*) = nullptr;
    unsigned int (__cdecl *serviceLayout)(unsigned int*,unsigned int) = nullptr;
    unsigned int (__cdecl *serviceLive)() = nullptr;
    HRESULT (__cdecl *serviceObjectCreate)(const char*,const GUID*,void**,void*,void (__cdecl *)(void*,unsigned int,int),void (__cdecl *)(void*,unsigned int,int),void (__cdecl *)(void*,unsigned int,int),void (__cdecl *)(void*,unsigned int,int),void (__cdecl *)(int)) = nullptr;
    HRESULT (__cdecl *serviceRoute)(RustServiceState*,void*,const char*,const RustServiceCall*,void*,void (__cdecl *)(void*,unsigned int,int),void (__cdecl *)(void*,unsigned int,int),void (__cdecl *)(void*,unsigned int,int),void (__cdecl *)(void*,unsigned int,int)) = nullptr;
    void* (__cdecl *serviceComCreate)(void*,HRESULT (__cdecl *)(void*,const RustServiceCall*),void (__cdecl *)(void*)) = nullptr;
    void* (__cdecl *serviceComInterface)(void*,const GUID*) = nullptr;
    int (__cdecl *serviceInitialize)(RustServiceState*,unsigned int,int,int,const char*) = nullptr;
    void (__cdecl *serviceClearResume)(RustServiceState*) = nullptr;
    int (__cdecl *serviceExchange)(RustServiceState*,const char*,void*,const unsigned char*,unsigned int,unsigned int,int,void*,int (__cdecl *)(void*,const unsigned char*,unsigned int),void (__cdecl *)(void*)) = nullptr;
    HRESULT (__cdecl *serviceFocusNative)(RustServiceState*,void*,void*,void*,const char*,void*,int,void*,void (__cdecl *)(void*,unsigned int,int)) = nullptr;
    int (__cdecl *serviceEditedNative)(RustServiceState*,void*,void*,const char*,void*,void*,unsigned int,void*,void (__cdecl *)(void*,unsigned int,int)) = nullptr;
    HRESULT (__cdecl *serviceOpenedNative)(RustServiceState*,void*,void*,const char*,const GUID*,void*,void (__cdecl *)(void*,unsigned int,int)) = nullptr;
    HRESULT (__cdecl *serviceShutdown)(RustServiceState*,void*,void*,const char*,void*,void (__cdecl *)(void*,unsigned int,int)) = nullptr;
    HRESULT (__cdecl *serviceActivateNative)(RustServiceState*,void*,const char*,void*,unsigned int,unsigned int,void* const*,void*,void (__cdecl *)(void*,unsigned int,int),void (__cdecl *)(void*,unsigned int,int),void* (__cdecl *)(void*)) = nullptr;
    int (__cdecl *serviceApplyComposition)(RustServiceState*,void*,void*,void*,unsigned int,const char*) = nullptr;
    int (__cdecl *serviceWriteCommit)(RustServiceState*,void*,void*,void*,void*,unsigned int,const unsigned short*,unsigned int) = nullptr;
    int (__cdecl *serviceEndComposition)(RustServiceState*,void*,unsigned int) = nullptr;
    HRESULT (__cdecl *serviceTerminated)(RustServiceState*,void*,const char*,unsigned int,void*,void*,void (__cdecl *)(void*)) = nullptr;
    int (__cdecl *serviceResume)(RustServiceState*,void*,const char*,void*,void*,void*,void (__cdecl *)(void*)) = nullptr;
    int (__cdecl *serviceAbandon)(RustServiceState*,void*,const char*,void*,void (__cdecl *)(void*)) = nullptr;
    int (__cdecl *serviceInputScope)(RustServiceState*,void*,void*,unsigned int,void*,void (__cdecl *)(void*,unsigned int,int)) = nullptr;
    int (__cdecl *serviceModeAction)(RustServiceState*,void*,const char*,void*,unsigned int,unsigned int,int,void*,void (__cdecl *)(void*,unsigned int,int)) = nullptr;
    int (__cdecl *serviceApplyReply)(RustServiceState*,void*,void*,const char*,void*,unsigned int,const unsigned char*,unsigned int,void*,void (__cdecl *)(void*)) = nullptr;
    int (__cdecl *serviceDispatchKey)(RustServiceState*,void*,void*,const char*,unsigned int,void*,unsigned int,unsigned long long,void*,void (__cdecl *)(void*,unsigned int,int),void (__cdecl *)(void*,unsigned int,int),unsigned int (__cdecl *)(void*,const INPUT*,unsigned int)) = nullptr;
    HRESULT (__cdecl *serviceCommit)(RustServiceState*,void*,const char*,unsigned int,void*,unsigned int,int,void*,RustServiceApply,void (__cdecl *)(void*)) = nullptr;
    HRESULT (__cdecl *serviceSelectCandidate)(RustServiceState*,void*,const char*,int,int,void*,RustServiceApply,void (__cdecl *)(void*)) = nullptr;
    int (__cdecl *serviceCore)(RustServiceState*,const char*,void*,void*,void (__cdecl *)(void*,unsigned int,int),unsigned long long (__cdecl *)(void*),int) = nullptr;
    unsigned int (__cdecl *serviceMode)(const RustServiceState*) = nullptr;
    int (__cdecl *serviceWriteOpen)(RustServiceState*,int) = nullptr;
    int (__cdecl *serviceApplyMode)(RustServiceState*,unsigned int,void*,void*,void (__cdecl *)(void*,int)) = nullptr;
    HRESULT (__cdecl *serviceEditSink)(RustServiceState*,void*,void*,void*) = nullptr;
    int (__cdecl *serviceSetSid)(RustServiceState*,const char*) = nullptr;
    const char* (__cdecl *serviceSid)(const RustServiceState*) = nullptr;
    void (__cdecl *serviceClose)(RustServiceState*) = nullptr;
    void* (__cdecl *serviceCreateBar)(RustServiceState*,void*,int (__cdecl *)(void*),void (__cdecl *)(void*)) = nullptr;
    void (__cdecl *serviceUpdateBar)(RustServiceState*) = nullptr;
    void (__cdecl *serviceRemoveBar)(RustServiceState*,void*) = nullptr;
    void* (__cdecl *serviceCreateCandidates)(RustServiceState*,void*,void (__cdecl *)(void*,int,int)) = nullptr;
    int (__cdecl *serviceShowCandidates)(RustServiceState*,void*,void*,unsigned int,const char*) = nullptr;
    void (__cdecl *serviceHideCandidates)(RustServiceState*) = nullptr;
    void (__cdecl *serviceDropCandidates)(RustServiceState*) = nullptr;
    unsigned int (__cdecl *processName)(unsigned short*,unsigned int) = nullptr;
    RustApi() {
        wchar_t path[32768];
        const DWORD size = GetModuleFileNameW(g_module, path, ARRAYSIZE(path));
        if (size == 0 || size >= ARRAYSIZE(path)) return;
        std::wstring directory(path, size);
        directory.resize(directory.find_last_of(L"\\/") + 1);
        worker=ToUtf8(directory+L"meltype-tip-worker.exe");
        // Absolute path and restricted dependency search: never load from CWD.
        module = LoadLibraryExW((directory + L"meltype_native.dll").c_str(), nullptr,
                               LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32);
        if (!module) return;
        create = reinterpret_cast<decltype(create)>(GetProcAddress(module, "meltype_tip_transport_create"));
        destroy = reinterpret_cast<decltype(destroy)>(GetProcAddress(module, "meltype_tip_transport_destroy"));
        request = reinterpret_cast<decltype(request)>(GetProcAddress(module, "meltype_tip_transport_request"));
        freeText = reinterpret_cast<decltype(freeText)>(GetProcAddress(module, "meltype_free"));
        displayAttribute = reinterpret_cast<decltype(displayAttribute)>(GetProcAddress(module, "meltype_tsf_create_display_attribute"));
        displayAttributeEnum = reinterpret_cast<decltype(displayAttributeEnum)>(GetProcAddress(module, "meltype_tsf_create_display_attribute_enum"));
        classObject = reinterpret_cast<decltype(classObject)>(GetProcAddress(module, "meltype_tsf_get_class_object"));
        registerServer = reinterpret_cast<decltype(registerServer)>(GetProcAddress(module, "meltype_tsf_register_server"));
        unregisterServer = reinterpret_cast<decltype(unregisterServer)>(GetProcAddress(module, "meltype_tsf_unregister_server"));
        editSession = reinterpret_cast<decltype(editSession)>(GetProcAddress(module, "meltype_tsf_request_edit_session"));
        surrounding = reinterpret_cast<decltype(surrounding)>(GetProcAddress(module, "meltype_tsf_read_surrounding"));
        writeRange = reinterpret_cast<decltype(writeRange)>(GetProcAddress(module, "meltype_tsf_write_range"));
        startComposition = reinterpret_cast<decltype(startComposition)>(GetProcAddress(module, "meltype_tsf_start_composition"));
        updateComposition = reinterpret_cast<decltype(updateComposition)>(GetProcAddress(module, "meltype_tsf_update_composition"));
        endComposition = reinterpret_cast<decltype(endComposition)>(GetProcAddress(module, "meltype_tsf_end_composition"));
        setAttribute = reinterpret_cast<decltype(setAttribute)>(GetProcAddress(module, "meltype_tsf_set_attribute"));
        applyCommits = reinterpret_cast<decltype(applyCommits)>(GetProcAddress(module, "meltype_tsf_apply_commits"));
        captureResume = reinterpret_cast<decltype(captureResume)>(GetProcAddress(module, "meltype_tsf_capture_resume"));
        resumeMatches = reinterpret_cast<decltype(resumeMatches)>(GetProcAddress(module, "meltype_tsf_resume_matches"));
        tryResume = reinterpret_cast<decltype(tryResume)>(GetProcAddress(module, "meltype_tsf_try_resume"));
        resumeAttribute = reinterpret_cast<decltype(resumeAttribute)>(GetProcAddress(module, "meltype_tsf_resume_attribute"));
        destroyResume = reinterpret_cast<decltype(destroyResume)>(GetProcAddress(module, "meltype_tsf_destroy_resume"));
        abandonedCreate = reinterpret_cast<decltype(abandonedCreate)>(GetProcAddress(module, "meltype_tsf_abandoned_create"));
        abandonedReset = reinterpret_cast<decltype(abandonedReset)>(GetProcAddress(module, "meltype_tsf_abandoned_reset"));
        abandonedDestroy = reinterpret_cast<decltype(abandonedDestroy)>(GetProcAddress(module, "meltype_tsf_abandoned_destroy"));
        abandonedAdd = reinterpret_cast<decltype(abandonedAdd)>(GetProcAddress(module, "meltype_tsf_abandoned_add"));
        abandonedForget = reinterpret_cast<decltype(abandonedForget)>(GetProcAddress(module, "meltype_tsf_abandoned_forget"));
        abandonedEnd = reinterpret_cast<decltype(abandonedEnd)>(GetProcAddress(module, "meltype_tsf_abandoned_end"));
        clearCompositionAttribute = reinterpret_cast<decltype(clearCompositionAttribute)>(GetProcAddress(module, "meltype_tsf_clear_composition_attribute"));
        inputScope = reinterpret_cast<decltype(inputScope)>(GetProcAddress(module, "meltype_tsf_input_scope"));
        readOpen = reinterpret_cast<decltype(readOpen)>(GetProcAddress(module, "meltype_tsf_read_open"));
        writeOpen = reinterpret_cast<decltype(writeOpen)>(GetProcAddress(module, "meltype_tsf_write_open"));
        contextDisabled = reinterpret_cast<decltype(contextDisabled)>(GetProcAddress(module, "meltype_tsf_context_disabled"));
        subscribe = reinterpret_cast<decltype(subscribe)>(GetProcAddress(module, "meltype_tsf_subscribe"));
        unsubscribe = reinterpret_cast<decltype(unsubscribe)>(GetProcAddress(module, "meltype_tsf_unsubscribe"));
        keyFlags = reinterpret_cast<decltype(keyFlags)>(GetProcAddress(module, "meltype_tsf_key_flags"));
        keyModifiers = reinterpret_cast<decltype(keyModifiers)>(GetProcAddress(module, "meltype_tsf_key_modifiers"));
        keyDown = reinterpret_cast<decltype(keyDown)>(GetProcAddress(module, "meltype_tsf_key_down"));
        keyChar = reinterpret_cast<decltype(keyChar)>(GetProcAddress(module, "meltype_tsf_key_char"));
        keyStateCreate = reinterpret_cast<decltype(keyStateCreate)>(GetProcAddress(module, "meltype_tsf_key_state_create"));
        keyStateReset = reinterpret_cast<decltype(keyStateReset)>(GetProcAddress(module, "meltype_tsf_key_state_reset"));
        keyStateDestroy = reinterpret_cast<decltype(keyStateDestroy)>(GetProcAddress(module, "meltype_tsf_key_state_destroy"));
        keyCacheClear = reinterpret_cast<decltype(keyCacheClear)>(GetProcAddress(module, "meltype_tsf_key_cache_clear"));
        isReinjected = reinterpret_cast<decltype(isReinjected)>(GetProcAddress(module, "meltype_tsf_is_reinjected"));
        reinject = reinterpret_cast<decltype(reinject)>(GetProcAddress(module, "meltype_tsf_reinject"));
        keyForeground = reinterpret_cast<decltype(keyForeground)>(GetProcAddress(module, "meltype_tsf_key_foreground"));
        keyStaleRead = reinterpret_cast<decltype(keyStaleRead)>(GetProcAddress(module, "meltype_tsf_key_stale_read"));
        keyStaleWrite = reinterpret_cast<decltype(keyStaleWrite)>(GetProcAddress(module, "meltype_tsf_key_stale_write"));
        keyStaleClear = reinterpret_cast<decltype(keyStaleClear)>(GetProcAddress(module, "meltype_tsf_key_stale_clear"));
        keyPassed = reinterpret_cast<decltype(keyPassed)>(GetProcAddress(module, "meltype_tsf_key_passed"));
        jsonVisit = reinterpret_cast<decltype(jsonVisit)>(GetProcAddress(module, "meltype_json_visit"));
        jsonQuote = reinterpret_cast<decltype(jsonQuote)>(GetProcAddress(module, "meltype_json_quote"));
        coreReady = reinterpret_cast<decltype(coreReady)>(GetProcAddress(module, "meltype_tsf_core_ready"));
        coreHello = reinterpret_cast<decltype(coreHello)>(GetProcAddress(module, "meltype_tsf_core_hello"));
        serviceCreate = reinterpret_cast<decltype(serviceCreate)>(GetProcAddress(module, "meltype_tsf_service_state_create"));
        serviceDestroy = reinterpret_cast<decltype(serviceDestroy)>(GetProcAddress(module, "meltype_tsf_service_state_destroy"));
        serviceLayout = reinterpret_cast<decltype(serviceLayout)>(GetProcAddress(module, "meltype_tsf_service_state_layout"));
        processName = reinterpret_cast<decltype(processName)>(GetProcAddress(module, "meltype_tsf_process_name"));
        serviceInitialize = reinterpret_cast<decltype(serviceInitialize)>(GetProcAddress(module, "meltype_tsf_service_initialize"));
        serviceExchange = reinterpret_cast<decltype(serviceExchange)>(GetProcAddress(module, "meltype_tsf_service_exchange"));
        serviceFocusNative = reinterpret_cast<decltype(serviceFocusNative)>(GetProcAddress(module,"meltype_tsf_service_focus_native"));
        serviceEditedNative = reinterpret_cast<decltype(serviceEditedNative)>(GetProcAddress(module,"meltype_tsf_service_edited_native"));
        serviceOpenedNative = reinterpret_cast<decltype(serviceOpenedNative)>(GetProcAddress(module,"meltype_tsf_service_opened_native"));
        serviceShutdown = reinterpret_cast<decltype(serviceShutdown)>(GetProcAddress(module, "meltype_tsf_service_shutdown"));
        serviceActivateNative = reinterpret_cast<decltype(serviceActivateNative)>(GetProcAddress(module,"meltype_tsf_service_activate_native"));
        serviceApplyComposition = reinterpret_cast<decltype(serviceApplyComposition)>(GetProcAddress(module,"meltype_tsf_service_apply_composition"));
        serviceWriteCommit = reinterpret_cast<decltype(serviceWriteCommit)>(GetProcAddress(module,"meltype_tsf_service_write_commit"));
        serviceEndComposition = reinterpret_cast<decltype(serviceEndComposition)>(GetProcAddress(module,"meltype_tsf_service_end_composition"));
        serviceTerminated = reinterpret_cast<decltype(serviceTerminated)>(GetProcAddress(module,"meltype_tsf_service_terminated"));
        serviceResume = reinterpret_cast<decltype(serviceResume)>(GetProcAddress(module,"meltype_tsf_service_resume"));
        serviceAbandon = reinterpret_cast<decltype(serviceAbandon)>(GetProcAddress(module,"meltype_tsf_service_abandon"));
        serviceInputScope = reinterpret_cast<decltype(serviceInputScope)>(GetProcAddress(module,"meltype_tsf_service_input_scope"));
        serviceModeAction = reinterpret_cast<decltype(serviceModeAction)>(GetProcAddress(module,"meltype_tsf_service_mode_action"));
        serviceApplyReply = reinterpret_cast<decltype(serviceApplyReply)>(GetProcAddress(module,"meltype_tsf_service_apply_reply"));
        serviceDispatchKey = reinterpret_cast<decltype(serviceDispatchKey)>(GetProcAddress(module,"meltype_tsf_service_dispatch_key"));
        serviceCommit = reinterpret_cast<decltype(serviceCommit)>(GetProcAddress(module,"meltype_tsf_service_commit"));
        serviceSelectCandidate = reinterpret_cast<decltype(serviceSelectCandidate)>(GetProcAddress(module,"meltype_tsf_service_select_candidate"));
        serviceCore = reinterpret_cast<decltype(serviceCore)>(GetProcAddress(module, "meltype_tsf_service_core"));
        serviceMode = reinterpret_cast<decltype(serviceMode)>(GetProcAddress(module, "meltype_tsf_service_mode"));
        serviceWriteOpen = reinterpret_cast<decltype(serviceWriteOpen)>(GetProcAddress(module, "meltype_tsf_service_write_open"));
        serviceApplyMode = reinterpret_cast<decltype(serviceApplyMode)>(GetProcAddress(module, "meltype_tsf_service_apply_mode"));
        serviceEditSink = reinterpret_cast<decltype(serviceEditSink)>(GetProcAddress(module, "meltype_tsf_service_edit_sink"));
        serviceClearResume = reinterpret_cast<decltype(serviceClearResume)>(GetProcAddress(module, "meltype_tsf_service_clear_resume"));
        serviceSetSid = reinterpret_cast<decltype(serviceSetSid)>(GetProcAddress(module, "meltype_tsf_service_set_sid"));
        serviceSid = reinterpret_cast<decltype(serviceSid)>(GetProcAddress(module, "meltype_tsf_service_sid"));
        serviceClose = reinterpret_cast<decltype(serviceClose)>(GetProcAddress(module, "meltype_tsf_service_close"));
        serviceCreateBar = reinterpret_cast<decltype(serviceCreateBar)>(GetProcAddress(module, "meltype_tsf_service_create_langbar"));
        serviceUpdateBar = reinterpret_cast<decltype(serviceUpdateBar)>(GetProcAddress(module, "meltype_tsf_service_update_langbar"));
        serviceRemoveBar = reinterpret_cast<decltype(serviceRemoveBar)>(GetProcAddress(module, "meltype_tsf_service_remove_langbar"));
        serviceCreateCandidates = reinterpret_cast<decltype(serviceCreateCandidates)>(GetProcAddress(module, "meltype_tsf_service_create_candidates"));
        serviceShowCandidates = reinterpret_cast<decltype(serviceShowCandidates)>(GetProcAddress(module, "meltype_tsf_service_show_candidates"));
        serviceHideCandidates = reinterpret_cast<decltype(serviceHideCandidates)>(GetProcAddress(module, "meltype_tsf_service_hide_candidates"));
        serviceDropCandidates = reinterpret_cast<decltype(serviceDropCandidates)>(GetProcAddress(module, "meltype_tsf_service_drop_candidates"));
        serviceObjectCreate = reinterpret_cast<decltype(serviceObjectCreate)>(GetProcAddress(module,"meltype_tsf_service_object_create"));
        serviceRoute = reinterpret_cast<decltype(serviceRoute)>(GetProcAddress(module,"meltype_tsf_service_route"));
        serviceComCreate = reinterpret_cast<decltype(serviceComCreate)>(GetProcAddress(module, "meltype_tsf_service_com_create"));
        serviceComInterface = reinterpret_cast<decltype(serviceComInterface)>(GetProcAddress(module, "meltype_tsf_service_com_interface"));
        serviceLive = reinterpret_cast<decltype(serviceLive)>(GetProcAddress(module, "meltype_tsf_service_state_live_count"));
    }
    bool ready() const { return create && destroy && request && freeText; }
    // The module stays loaded for the process lifetime. Do not unload Rust
    // thread state under the Windows loader lock during TSF DLL teardown.
};
RustApi& api() { static RustApi value; return value; }
std::string ViewJson(const JsonValue&);
HRESULT __cdecl CreateTextService(const GUID* iid, void** out) {
    if (!iid || !out) return E_INVALIDARG;
    *out=nullptr;
    return RustCreateNativeService(*iid,out);
}
void __cdecl LockShell(int lock) { if (lock) DllAddRef(); else DllRelease(); }
}
HRESULT RustGetClassObject(REFCLSID clsid, REFIID iid, void** out) {
    if (!out) return E_INVALIDARG;
    *out = nullptr;
    if (!api().classObject) return CLASS_E_CLASSNOTAVAILABLE;
    return api().classObject(&clsid,&iid,out,CreateTextService,LockShell);
}
HRESULT RustRegisterServer() {
    if (!api().registerServer) return E_FAIL;
    wchar_t path[32768];
    DWORD length = GetModuleFileNameW(g_module,path,ARRAYSIZE(path));
    if (!length || length >= ARRAYSIZE(path)) return E_FAIL;
    return api().registerServer(path,length);
}
HRESULT RustUnregisterServer() { return api().unregisterServer ? api().unregisterServer() : E_FAIL; }
namespace {
struct EditAction {
    std::function<HRESULT(TfEditCookie)> action;
    explicit EditAction(std::function<HRESULT(TfEditCookie)> value) : action(std::move(value)) { DllAddRef(); }
    ~EditAction() {
        // Captures may own C++ COM objects with destructors in this shell.
        // Destroy those captures before dropping the DLL's final pin.
        action = {};
        DllRelease();
    }
    static HRESULT __cdecl Invoke(void* context, unsigned int cookie) {
        try { return static_cast<EditAction*>(context)->action(cookie); }
        catch (...) { return E_FAIL; }
    }
    static void __cdecl Destroy(void* context) { delete static_cast<EditAction*>(context); }
};
}
HRESULT RunEditSession(ITfContext* context, TfClientId client, DWORD flags, std::function<HRESULT(TfEditCookie)> action) {
    if (!context) return E_INVALIDARG;
    if (!api().editSession) return E_FAIL;
    auto* owned = new (std::nothrow) EditAction(std::move(action));
    if (!owned) return E_OUTOFMEMORY;
    // Rust consumes owned on success, immediate failure and queued execution.
    return api().editSession(context,client,flags,owned,EditAction::Invoke,EditAction::Destroy);
}
namespace {
struct SurroundingReceiver { std::wstring* before; std::wstring* after; };
void __cdecl ReceiveSurrounding(void* value,const unsigned short* before,unsigned int beforeLength,const unsigned short* after,unsigned int afterLength) {
    auto* receiver = static_cast<SurroundingReceiver*>(value);
    receiver->before->assign(reinterpret_cast<const wchar_t*>(before),beforeLength);
    receiver->after->assign(reinterpret_cast<const wchar_t*>(after),afterLength);
}
}
void RustSurroundingText(TfEditCookie cookie,ITfContext* context,std::wstring& before,std::wstring& after) {
    SurroundingReceiver receiver = {&before,&after};
    if (api().surrounding) api().surrounding(context,cookie,&receiver,ReceiveSurrounding);
}
bool RustWriteRange(TfEditCookie cookie,ITfContext* context,ITfRange* range,const std::wstring& text,bool keepCaret) {
    return api().writeRange && api().writeRange(context,range,cookie,reinterpret_cast<const unsigned short*>(text.data()),static_cast<unsigned int>(text.size()),keepCaret ? 1 : 0) != 0;
}
ITfComposition* RustStartComposition(TfEditCookie cookie, ITfContext* context, ITfCompositionSink* sink, ITfRange* range) {
    return api().startComposition ? static_cast<ITfComposition*>(api().startComposition(context,sink,range,cookie)) : nullptr;
}
bool RustUpdateComposition(TfEditCookie cookie,ITfContext* context,ITfComposition* composition,const JsonValue& view,TfGuidAtom input,TfGuidAtom converted,TfGuidAtom target) {
    if (!api().updateComposition) return false;
    std::string clauses = "[";
    for (size_t i = 0; i < view[L"clauses"].array.size(); ++i) {
        if (i) clauses += ',';
        clauses += JsonString(view[L"clauses"].array[i].Str());
    }
    std::string json = "{\"text\":" + JsonString(view[L"text"].Str()) + ",\"converting\":" + (view[L"converting"].Bool() ? "true" : "false") +
        ",\"clauses\":" + clauses + "],\"selectedClause\":" + std::to_string(view[L"selectedClause"].Int()) + "}";
    return api().updateComposition(context,composition,cookie,json.c_str(),input,converted,target) != 0;
}
void RustEndComposition(TfEditCookie cookie,ITfContext* context,ITfComposition* composition) {
    if (api().endComposition) api().endComposition(context,composition,cookie);
}
bool RustJsonVisit(const std::string& text,void* owner,RustJsonEmit emit) {
    return text.size()<=UINT_MAX && api().jsonVisit && api().jsonVisit(reinterpret_cast<const unsigned char*>(text.data()),static_cast<unsigned int>(text.size()),owner,emit)!=0;
}
bool RustCoreReady(const RustCoreStateCallbacks& callbacks) {return api().coreReady && api().coreReady(&callbacks)!=0;}
RustServiceState* RustCreateServiceState() {
    if (!api().serviceRoute || !api().serviceCreate || !api().serviceDestroy || !api().serviceLayout || !api().serviceCore || !api().serviceApplyReply ||  !api().serviceCommit || !api().serviceSelectCandidate || !api().serviceDispatchKey || !api().serviceInputScope || !api().serviceModeAction || !api().serviceAbandon || !api().serviceTerminated || !api().serviceResume || !api().serviceApplyComposition || !api().serviceWriteCommit || !api().serviceEndComposition || !api().serviceActivateNative || !api().serviceShutdown || !api().serviceOpenedNative || !api().serviceEditedNative || !api().serviceFocusNative || !api().serviceExchange || !api().serviceMode || !api().serviceWriteOpen || !api().serviceApplyMode || !api().serviceEditSink || !api().serviceInitialize || !api().serviceClearResume || !api().serviceSetSid || !api().serviceSid || !api().serviceClose || !api().serviceCreateBar || !api().serviceUpdateBar || !api().serviceRemoveBar || !api().serviceCreateCandidates || !api().serviceShowCandidates || !api().serviceHideCandidates || !api().serviceDropCandidates) return nullptr;
    const unsigned int expected[]={6,static_cast<unsigned int>(sizeof(RustServiceState)),static_cast<unsigned int>(alignof(RustServiceState)),
        static_cast<unsigned int>(offsetof(RustServiceState,refs)),
        static_cast<unsigned int>(offsetof(RustServiceState,manager)),
        static_cast<unsigned int>(offsetof(RustServiceState,client)),
        static_cast<unsigned int>(offsetof(RustServiceState,thread_subscription)),
        static_cast<unsigned int>(offsetof(RustServiceState,key_subscription)),
        static_cast<unsigned int>(offsetof(RustServiceState,mode_subscription)),
        static_cast<unsigned int>(offsetof(RustServiceState,edit_subscription)),
        static_cast<unsigned int>(offsetof(RustServiceState,edit_context)),
        static_cast<unsigned int>(offsetof(RustServiceState,composition)),
        static_cast<unsigned int>(offsetof(RustServiceState,composition_context)),
        static_cast<unsigned int>(offsetof(RustServiceState,atom_input)),
        static_cast<unsigned int>(offsetof(RustServiceState,atom_converted)),
        static_cast<unsigned int>(offsetof(RustServiceState,atom_target)),
        static_cast<unsigned int>(offsetof(RustServiceState,keys)),
        static_cast<unsigned int>(offsetof(RustServiceState,resume)),
        static_cast<unsigned int>(offsetof(RustServiceState,abandoned)),
        static_cast<unsigned int>(offsetof(RustServiceState,last_hello)),
        static_cast<unsigned int>(offsetof(RustServiceState,focus_epoch)),
        static_cast<unsigned int>(offsetof(RustServiceState,mode_epoch)),
        static_cast<unsigned int>(offsetof(RustServiceState,shutdown_phase)),
        static_cast<unsigned int>(offsetof(RustServiceState,server_active)),
        static_cast<unsigned int>(offsetof(RustServiceState,direct_detection)),
        static_cast<unsigned int>(offsetof(RustServiceState,direct_word)),
        static_cast<unsigned int>(offsetof(RustServiceState,open)),
        static_cast<unsigned int>(offsetof(RustServiceState,code_app)),
        static_cast<unsigned int>(offsetof(RustServiceState,code_english)),
        static_cast<unsigned int>(offsetof(RustServiceState,disabled)),
        static_cast<unsigned int>(offsetof(RustServiceState,own_process)),
        static_cast<unsigned int>(offsetof(RustServiceState,last_converting)),
        static_cast<unsigned int>(offsetof(RustServiceState,caret_moved)),
        static_cast<unsigned int>(offsetof(RustServiceState,own_edit)),
        static_cast<unsigned int>(offsetof(RustServiceState,secret_checked)),
        static_cast<unsigned int>(offsetof(RustServiceState,secret_composing)),
        static_cast<unsigned int>(offsetof(RustServiceState,private_field)),
        static_cast<unsigned int>(offsetof(RustServiceState,keep_caret)),
        static_cast<unsigned int>(offsetof(RustServiceState,deactivated)),
        static_cast<unsigned int>(offsetof(RustServiceState,key_failed)),
        static_cast<unsigned int>(offsetof(RustServiceState,consumed)),
        static_cast<unsigned int>(offsetof(RustServiceState,scope_source)),
        static_cast<unsigned int>(offsetof(RustServiceState,pass_reason)),
        static_cast<unsigned int>(offsetof(RustServiceState,sid)),
        static_cast<unsigned int>(offsetof(RustServiceState,transport)),
        static_cast<unsigned int>(offsetof(RustServiceState,langbar)),
        static_cast<unsigned int>(offsetof(RustServiceState,candidates)),
        static_cast<unsigned int>(offsetof(RustServiceState,edit_epoch)),
        static_cast<unsigned int>(offsetof(RustServiceState,full_width_space)),
    };
    unsigned int actual[ARRAYSIZE(expected)]={};
    if (api().serviceLayout(actual,ARRAYSIZE(actual))!=ARRAYSIZE(expected) || std::memcmp(actual,expected,sizeof(expected))!=0) return nullptr;
    return api().serviceCreate();
}
void RustDestroyServiceState(RustServiceState* state) {if (api().serviceDestroy) api().serviceDestroy(state);}
unsigned int RustServiceStateLiveCount() {return api().serviceLive?api().serviceLive():0;}
RustServiceStateStorage::RustServiceStateStorage():value(RustCreateServiceState()) {
    if (!value) throw std::bad_alloc();
    DllAddRef();
}
RustServiceStateStorage::~RustServiceStateStorage() {
    RustDestroyServiceState(value);
    DllRelease();
}

void* RustCreateServiceCom(void* owner,HRESULT (__cdecl *dispatch)(void*,const RustServiceCall*),void (__cdecl *destroy)(void*)) {
    return api().serviceComCreate && api().serviceComInterface?api().serviceComCreate(owner,dispatch,destroy):nullptr;
}
void* RustServiceComInterface(void* root,REFIID iid) {return api().serviceComInterface?api().serviceComInterface(root,&iid):nullptr;}
bool RustServiceExchange(RustServiceState* state,IUnknown* lifetime,const std::string& request,DWORD timeout,bool optional,void* context,int (__cdecl *reply)(void*,const unsigned char*,unsigned int),void (__cdecl *failure)(void*)) {
    return request.size()<=UINT_MAX && api().serviceExchange && api().serviceExchange(state,api().worker.c_str(),lifetime,reinterpret_cast<const unsigned char*>(request.data()),static_cast<unsigned int>(request.size()),timeout,optional?1:0,context,reply,failure)!=0;
}
static void __cdecl ServiceCoreNotice(void*,unsigned int op,int active) {
    if (op==0) TipLog(L"Meltype.exe: %s",active?L"つながりました":L"使えません (動いていない・モードが Meltype IME でない)");
    else if (op==1) TipLog(L"応答を読めません");
}
HRESULT RustServiceShutdown(RustServiceState* state,IUnknown* lifetime,ITfCompositionSink* sink) {
    return api().serviceShutdown?api().serviceShutdown(state,lifetime,sink,api().worker.c_str(),nullptr,
        [](void*,unsigned int op,int value) {try {
            if (op==0) TipLog(L"手放した変換中の文字を終えられませんでした: 0x%08X",static_cast<unsigned int>(value));
            else if (op==1) TipLog(L"応答を読めません");
        }catch (...) {}}):E_FAIL;
}

int RustServiceApplyComposition(RustServiceState* state,IUnknown* lifetime,ITfCompositionSink* sink,TfEditCookie cookie,ITfContext* context,const JsonValue& view) {
    const auto json=ViewJson(view);
    return api().serviceApplyComposition?api().serviceApplyComposition(state,lifetime,sink,context,cookie,json.c_str()):-1;
}
int RustServiceWriteCommit(RustServiceState* state,IUnknown* lifetime,ITfCompositionSink* sink,TfEditCookie cookie,ITfContext* context,ITfRange* range,const std::wstring& text) {
    return text.size()<=UINT_MAX && api().serviceWriteCommit?api().serviceWriteCommit(state,lifetime,sink,context,range,cookie,reinterpret_cast<const unsigned short*>(text.data()),static_cast<unsigned int>(text.size())):-1;
}
int RustServiceEndComposition(RustServiceState* state,IUnknown* lifetime,TfEditCookie cookie) {
    return api().serviceEndComposition?api().serviceEndComposition(state,lifetime,cookie):-1;
}
HRESULT RustServiceTerminated(RustServiceState* state,IUnknown* lifetime,TfEditCookie cookie,ITfComposition* composition) {
    return api().serviceTerminated?api().serviceTerminated(state,lifetime,api().worker.c_str(),cookie,composition,nullptr,
        [](void*) {try {TipLog(L"応答を読めません");}catch (...) {}}):E_NOINTERFACE;
}
int RustServiceResume(RustServiceState* state,IUnknown* lifetime,ITfCompositionSink* sink,ITfContext* context) {
    return api().serviceResume?api().serviceResume(state,lifetime,api().worker.c_str(),sink,context,nullptr,
        [](void*) {try {TipLog(L"応答を読めません");}catch (...) {}}):-1;
}
int RustServiceAbandon(RustServiceState* state,IUnknown* lifetime) {
    return api().serviceAbandon?api().serviceAbandon(state,lifetime,api().worker.c_str(),nullptr,
        [](void*) {try {TipLog(L"応答を読めません");}catch (...) {}}):-1;
}
static void __cdecl ServiceModeNotice(void*,unsigned int op,int value) {
    try {switch(op) {
        case 0:case 1:ServiceCoreNotice(nullptr,op,value);break;
        case 2:TipLog(L"入力モードの表示: %s",value?L"英数 (コードの行)":L"日本語");break;
        case 3:TipLog(L"コードの行: %s",value?L"日本語にしました":L"英数に戻しました");break;
        case 4:{const wchar_t* source=value==3?L"Windows とアプリ":value==1?L"Windows":value==2?L"アプリ":L"無い";TipLog(L"入力欄の種類を読むところ: %s",source);break;}
        case 6:{const wchar_t* reasons[]={nullptr,L"英数",L"Ctrl・Alt・Windows キーを押している",L"この入力欄では使わない",L"英語の単語の続き",L"Meltype.exe が使えない",L"パスワードの入力欄"};if (value>0 && value<ARRAYSIZE(reasons)) TipLog(L"キーを通します: %s",reasons[value]);break;}
        case 7:TipLog(L"アプリが確定した打ちかけの文字を、変換中に戻しました");break;
        case 5:{const wchar_t* step=value==1?L"ITfInputScope":value==2?L"GetInputScopes":value==3?L"VARIANT":L"Rust API";TipLog(L"入力欄の種類を読めないので、パスワード欄として扱います (%s)",step);break;}
    }}catch (...) {}
}
bool RustServiceInputScope(RustServiceState* state,IUnknown* lifetime,TfEditCookie cookie,ITfContext* context) {
    return !api().serviceInputScope || api().serviceInputScope(state,lifetime,context,cookie,nullptr,ServiceModeNotice)!=0;
}
bool RustServiceModeAction(RustServiceState* state,IUnknown* lifetime,ITfContext* context,TfEditCookie cookie,unsigned int op,int value) {
    return api().serviceModeAction && api().serviceModeAction(state,lifetime,api().worker.c_str(),context,cookie,op,value,nullptr,ServiceModeNotice)!=0;
}
static void __cdecl ServiceNotificationNotice(void*,unsigned int op,int value) {
    try {
        if (op==10) TipLog(L"キャレットが変換中の文字の外に出たので確定します");
        else if (op==11) TipLog(L"入力モード: %s",value?L"日本語":L"英数");
        else ServiceModeNotice(nullptr,op,value);
    }catch (...) {}
}
HRESULT RustServiceFocusNative(RustServiceState* s,IUnknown* life,ITfCompositionSink* sink,ITfTextEditSink* edit,ITfDocumentMgr* doc,int event) {
    return api().serviceFocusNative?api().serviceFocusNative(s,life,sink,edit,api().worker.c_str(),doc,event,nullptr,ServiceNotificationNotice):E_NOINTERFACE;
}
bool RustServiceEditedNative(RustServiceState* s,IUnknown* life,ITfCompositionSink* sink,ITfContext* context,ITfEditRecord* record,TfEditCookie cookie) {
    return api().serviceEditedNative && api().serviceEditedNative(s,life,sink,api().worker.c_str(),context,record,cookie,nullptr,ServiceNotificationNotice)!=0;
}
HRESULT RustServiceOpenedNative(RustServiceState* s,IUnknown* life,ITfCompositionSink* sink,REFGUID guid) {
    return api().serviceOpenedNative?api().serviceOpenedNative(s,life,sink,api().worker.c_str(),&guid,nullptr,ServiceNotificationNotice):E_NOINTERFACE;
}
HRESULT RustServiceActivateNative(RustServiceState* state,IUnknown* life,ITfThreadMgr* manager,TfClientId client,DWORD flags,void* const* sinks) {
    return api().serviceActivateNative?api().serviceActivateNative(state,life,api().worker.c_str(),manager,client,flags,sinks,nullptr,
        [](void*,unsigned int op,int value){try {
            if (op==0) TipLog(L"Activate (%s)%s",RustProcessName().c_str(),value?L" 無効: セキュリティ保護された画面":L"");
            else ServiceCoreNotice(nullptr,op-1,value);
        }catch (...) {}},ServiceModeNotice,nullptr):E_NOINTERFACE;
}
static void __cdecl ServiceKeyNotice(void*,unsigned int op,int value) {
    try {switch (op) {
        case 0:TipLog(L"パスワードの入力欄なので何もしません");break;
        case 1:TipLog(L"英数文字を直接入力できませんでした。キーをアプリへ送り直します");break;
        case 2:TipLog(L"編集セッションを始められません: 0x%08X",static_cast<unsigned int>(value));break;
        case 3:TipLog(L"Meltype.exe から応答が来ないので、打った文字をそのまま入れました");break;
        case 4:ServiceModeNotice(nullptr,1,0);break;
        case 5:ServiceModeNotice(nullptr,2,value);break;
        case 6:ServiceModeNotice(nullptr,4,value);break;
        case 7:ServiceModeNotice(nullptr,5,value);break;
        case 8:TipLog(L"同じキーの送り直しが続くので、やめます");break;
        case 9:TipLog(L"キーをアプリに送り直せません: %lu",static_cast<DWORD>(value));break;
    }}catch (...) {}
}

HRESULT RustCreateNativeService(REFIID iid,void** out) {
    if (!out) return E_INVALIDARG;
    *out=nullptr;
    return api().serviceObjectCreate?api().serviceObjectCreate(api().worker.c_str(),&iid,out,nullptr,ServiceNotificationNotice,ServiceKeyNotice,
        [](void*,unsigned int op,int value){try {if(op==0) TipLog(L"Activate (%s)%s",RustProcessName().c_str(),value?L" 無効: セキュリティ保護された画面":L"");else ServiceCoreNotice(nullptr,op-1,value);}catch(...){}},
        [](void*,unsigned int op,int value){try {if(op==0) TipLog(L"手放した変換中の文字を終えられませんでした: 0x%08X",static_cast<unsigned int>(value));else if(op==1) TipLog(L"応答を読めません");}catch(...){}},LockShell):E_FAIL;
}
HRESULT RustRouteService(RustServiceState* state,IUnknown* root,const RustServiceCall* call) {
    return api().serviceRoute?api().serviceRoute(state,root,api().worker.c_str(),call,nullptr,ServiceNotificationNotice,ServiceKeyNotice,
        [](void*,unsigned int op,int value){try {if(op==0) TipLog(L"Activate (%s)%s",RustProcessName().c_str(),value?L" 無効: セキュリティ保護された画面":L"");else ServiceCoreNotice(nullptr,op-1,value);}catch(...){}},
        [](void*,unsigned int op,int value){try {if(op==0) TipLog(L"手放した変換中の文字を終えられませんでした: 0x%08X",static_cast<unsigned int>(value));else if(op==1) TipLog(L"応答を読めません");}catch(...){}}):E_FAIL;
}
static void __cdecl ServiceReplyFailure(void*) {try {TipLog(L"応答を読めません");}catch (...) {}}
HRESULT RustServiceCommit(RustServiceState* state,IUnknown* lifetime,unsigned int mode,ITfContext* context,TfEditCookie cookie,bool keep,void* adapter,RustServiceApply apply) {
    return api().serviceCommit?api().serviceCommit(state,lifetime,api().worker.c_str(),mode,context,cookie,keep?1:0,adapter,apply,ServiceReplyFailure):E_FAIL;
}
HRESULT RustServiceSelectCandidate(RustServiceState* state,IUnknown* lifetime,int index,bool prediction,void* adapter,RustServiceApply apply) {
    return api().serviceSelectCandidate?api().serviceSelectCandidate(state,lifetime,api().worker.c_str(),index,prediction?1:0,adapter,apply,ServiceReplyFailure):E_FAIL;
}
bool RustServiceDispatchKey(RustServiceState* state,IUnknown* lifetime,ITfCompositionSink* sink,unsigned int kind,ITfContext* context,UINT key,LPARAM message) {
    return api().serviceDispatchKey && api().serviceDispatchKey(state,lifetime,sink,api().worker.c_str(),kind,context,key,static_cast<ULONG_PTR>(message),nullptr,ServiceModeNotice,ServiceKeyNotice,nullptr)!=0;
}
bool RustServiceCore(RustServiceState* state,IUnknown* lifetime,bool hello) {
    return api().serviceCore && api().serviceCore(state,api().worker.c_str(),lifetime,nullptr,ServiceCoreNotice,nullptr,hello?1:0)!=0;
}
unsigned int RustServiceMode(const RustServiceState* state) {return api().serviceMode?api().serviceMode(state):0;}
void RustServiceWriteOpen(RustServiceState* state,bool open) {if (api().serviceWriteOpen) api().serviceWriteOpen(state,open?1:0);}
void RustServiceApplyMode(RustServiceState* state,unsigned int reply,IUnknown* lifetime,void* context,void (__cdecl *notice)(void*,int)) {
    if (api().serviceApplyMode) api().serviceApplyMode(state,reply,lifetime,context,notice);
}
HRESULT RustServiceEditSink(RustServiceState* state,ITfDocumentMgr* document,IUnknown* lifetime,ITfTextEditSink* sink) {
    return api().serviceEditSink?api().serviceEditSink(state,document,lifetime,sink):E_NOINTERFACE;
}
void RustServiceInitialize(RustServiceState* state,unsigned int client,int disabled,int own,const char* sid) {
    if (api().serviceInitialize) api().serviceInitialize(state,client,disabled,own,sid);
}
void RustServiceClearResume(RustServiceState* state) {if (api().serviceClearResume) api().serviceClearResume(state);}
void RustServiceSetSid(RustServiceState* state,const char* sid) {
    if (api().serviceSetSid) api().serviceSetSid(state,sid);
}
std::string RustServiceSid(const RustServiceState* state) {
    const char* sid=api().serviceSid?api().serviceSid(state):nullptr;
    return sid?std::string(sid):std::string{};
}
void RustServiceClose(RustServiceState* state) {if (api().serviceClose) api().serviceClose(state);}
ITfLangBarItem* RustServiceCreateBar(RustServiceState* state,void* owner,int (__cdecl *read)(void*),void (__cdecl *toggle)(void*)) {
    return api().serviceCreateBar?static_cast<ITfLangBarItem*>(api().serviceCreateBar(state,owner,read,toggle)):nullptr;
}
void RustServiceUpdateBar(RustServiceState* state) {if (api().serviceUpdateBar) api().serviceUpdateBar(state);}
void RustServiceRemoveBar(RustServiceState* state,IUnknown* manager) {if (api().serviceRemoveBar) api().serviceRemoveBar(state,manager);}
std::wstring RustProcessName() {
    if (!api().processName) return {};
    const unsigned int capacity=api().processName(nullptr,0);
    if (!capacity || capacity>32768) return {};
    std::wstring result(capacity,L'\0');
    if (api().processName(reinterpret_cast<unsigned short*>(result.data()),capacity)!=capacity) return {};
    result.resize(capacity-1);return result;
}
void RustCoreHello(const std::wstring& process,const RustCoreStateCallbacks& callbacks) {
    if (api().coreHello && process.size()<=UINT_MAX) api().coreHello(reinterpret_cast<const unsigned short*>(process.data()),static_cast<unsigned int>(process.size()),&callbacks);
}
std::string RustJsonQuote(const std::wstring& text) {
    if (text.size()>UINT_MAX || !api().jsonQuote || !api().freeText) return {};
    char* quoted=api().jsonQuote(reinterpret_cast<const unsigned short*>(text.data()),static_cast<unsigned int>(text.size()));
    if (!quoted) return {};
    std::string result(quoted);api().freeText(quoted);return result;
}
void RustSetAttribute(TfEditCookie cookie,ITfContext* context,ITfRange* range,TfGuidAtom atom) {
    if (api().setAttribute) api().setAttribute(context,range,cookie,atom);
}
namespace {
std::string CommitJson(const JsonValue& reply) {
    std::string json = "[";
    const auto& edits = reply[L"commits"].array;
    for (size_t i = 0; i < edits.size(); ++i) {
        if (i) json += ',';
        const auto& edit = edits[i];
        json += "{\"text\":" + JsonString(edit[L"text"].Str()) + ",\"deleteBefore\":" + std::to_string(edit[L"deleteBefore"].Int()) +
            ",\"caretBack\":" + std::to_string(edit[L"caretBack"].Int()) + ",\"expect\":" + (edit[L"expect"].type == JsonValue::Type::String ? JsonString(edit[L"expect"].Str()) : "null") + "}";
    }
    return json+"]";
}
}
bool RustApplyCommits(TfEditCookie cookie,ITfContext* context,const JsonValue& reply,void* owner,
    void* (__cdecl *current)(void*),int (__cdecl *write)(void*,void*,const unsigned short*,unsigned int)) {
    return api().applyCommits && api().applyCommits(context,cookie,CommitJson(reply).c_str(),owner,current,write)!=0;
}
int RustServiceApplyReplyBytes(RustServiceState* state,IUnknown* lifetime,ITfCompositionSink* sink,TfEditCookie cookie,ITfContext* context,const unsigned char* bytes,unsigned int length) {
    return api().serviceApplyReply?api().serviceApplyReply(state,lifetime,sink,api().worker.c_str(),context,cookie,bytes,length,nullptr,ServiceReplyFailure):-1;
}

void* RustCaptureResume(TfEditCookie cookie,ITfContext* context,ITfComposition* composition) {
    if (!api().captureResume || !api().resumeMatches || !api().tryResume || !api().resumeAttribute || !api().destroyResume) return nullptr;
    return api().captureResume(context,composition,cookie);
}
bool RustResumeMatches(void* state,ITfContext* context) { return api().resumeMatches && api().resumeMatches(state,context) != 0; }
ITfComposition* RustTryResume(void* state,TfEditCookie cookie,ITfContext* context,ITfCompositionSink* sink) {
    return api().tryResume ? static_cast<ITfComposition*>(api().tryResume(state,context,sink,cookie)) : nullptr;
}
void RustResumeAttribute(TfEditCookie cookie,ITfContext* context,ITfComposition* composition,TfGuidAtom atom) {
    if (api().resumeAttribute) api().resumeAttribute(context,composition,cookie,atom);
}
void RustDestroyResume(void* state) { if (api().destroyResume) api().destroyResume(state); }
void* RustAbandonedCreate() {
    return api().abandonedCreate && api().abandonedReset && api().abandonedDestroy && api().abandonedAdd &&
        api().abandonedForget && api().abandonedEnd && api().clearCompositionAttribute ? api().abandonedCreate() : nullptr;
}
void RustAbandonedReset(void* state) { if (api().abandonedReset) api().abandonedReset(state); }
void RustAbandonedDestroy(void* state) { if (api().abandonedDestroy) api().abandonedDestroy(state); }
void RustAbandonedAdd(void* state,ITfContext* context,ITfComposition* composition,IUnknown* owner,TfClientId client) {
    if (api().abandonedAdd) api().abandonedAdd(state,context,composition,owner,client);
}
bool RustAbandonedForget(void* state,ITfComposition* composition) { return api().abandonedForget && api().abandonedForget(state,composition) != 0; }
HRESULT RustAbandonedEnd(void* state,IUnknown* owner,TfClientId client) { return api().abandonedEnd ? api().abandonedEnd(state,owner,client) : E_NOINTERFACE; }
void RustClearCompositionAttribute(TfEditCookie cookie,ITfComposition* composition) {
    if (api().clearCompositionAttribute) api().clearCompositionAttribute(composition,cookie);
}
unsigned int RustInputScope(TfEditCookie cookie,ITfContext* context) { return api().inputScope ? api().inputScope(context,cookie) : 5; }
int RustReadOpen(ITfThreadMgr* manager) { return api().readOpen ? api().readOpen(manager) : -2; }
void RustWriteOpen(ITfThreadMgr* manager,TfClientId client,bool open) { if (api().writeOpen) api().writeOpen(manager,client,open ? 1 : 0); }
bool RustContextDisabled(ITfContext* context) { return !api().contextDisabled || api().contextDisabled(context) != 0; }
void* RustSubscribe(unsigned int kind,IUnknown* owner,TfClientId client,IUnknown* sink,ITfContext** context) {
    if (context) *context = nullptr;
    return api().subscribe && api().unsubscribe ? api().subscribe(kind,owner,client,sink,reinterpret_cast<void**>(context)) : nullptr;
}
void RustUnsubscribe(void* state) { if (state && api().unsubscribe) api().unsubscribe(state); }
unsigned int RustKeyFlags(UINT key,LPARAM message,bool state) {
    return api().keyFlags ? api().keyFlags(key,static_cast<ULONG_PTR>(message),state && api().keyModifiers ? api().keyModifiers() : 0) : 0;
}
bool RustKeyDown(int key) {return api().keyDown && api().keyDown(key)!=0;}
wchar_t RustKeyChar(UINT key,LPARAM message) {return api().keyChar ? static_cast<wchar_t>(api().keyChar(key,static_cast<ULONG_PTR>(message))) : 0;}
void* RustKeyStateCreate() {return api().keyStateCreate && api().keyStateReset && api().keyStateDestroy && api().keyCacheClear && api().isReinjected && api().reinject && api().keyForeground && api().keyStaleRead && api().keyStaleWrite && api().keyStaleClear && api().keyPassed && api().jsonVisit && api().jsonQuote && api().coreReady && api().coreHello && api().processName ? api().keyStateCreate() : nullptr;}
void RustKeyStateReset(void* state) {if (api().keyStateReset) api().keyStateReset(state);}
void RustKeyStateDestroy(void* state) {if (api().keyStateDestroy) api().keyStateDestroy(state);}
void RustKeyCacheClear(void* state) {if (api().keyCacheClear) api().keyCacheClear(state);}
bool RustIsReinjected(void* state,UINT key) {return api().isReinjected && api().isReinjected(state,key)!=0;}
int RustReinject(void* state,UINT key,IUnknown* lifetime,DWORD& error,void* owner,unsigned int (__cdecl *sender)(void*,const INPUT*,unsigned int)) {
    error=0;
    unsigned int code=0;
    int result=api().reinject ? api().reinject(state,key,lifetime,owner,sender,&code) : -1;
    error=code;
    return result;
}
void RustKeyForeground(void* state,bool foreground) {if (api().keyForeground) api().keyForeground(state,foreground?1:0);}
void RustKeyStaleRead(void* state,RustKeyStale& out) {out={};if (api().keyStaleRead) api().keyStaleRead(state,&out);}
void RustKeyStaleWrite(void* state,unsigned int tries,unsigned int key,unsigned long long time) {if (api().keyStaleWrite) api().keyStaleWrite(state,tries,key,time);}
void RustKeyStaleClear(void* state) {if (api().keyStaleClear) api().keyStaleClear(state);}
bool RustKeyPassedRead(void* state,bool mode,bool changed,RustKeyPassed& out) {out={};return api().keyPassed && api().keyPassed(state,mode?1:0,changed?1:0,&out)!=0;}

bool PipeClient::Transact(const std::string& request, std::string& response, DWORD timeout,bool optional) {
    response.clear();
    if (!api().ready()) return false;
    if (!rustConnection_) rustConnection_ = api().create(api().worker.c_str());
    if (!rustConnection_) return false;
    char* text = api().request(rustConnection_, request.c_str(),timeout,optional?1:0);
    if (!text) return false;
    response.assign(text); api().freeText(text); return true;
}
void PipeClient::Close() {
    if (rustConnection_ && api().ready()) api().destroy(rustConnection_);
    rustConnection_ = nullptr;
}
ITfDisplayAttributeInfo* CreateDisplayAttributeInfo(REFGUID guid) {
    return api().displayAttribute ? static_cast<ITfDisplayAttributeInfo*>(api().displayAttribute(&guid)) : nullptr;
}
IEnumTfDisplayAttributeInfo* CreateDisplayAttributeEnum() {
    return api().displayAttributeEnum ? static_cast<IEnumTfDisplayAttributeInfo*>(api().displayAttributeEnum()) : nullptr;
}
namespace {
std::string ViewJson(const JsonValue& view) {
    std::string json = "{";
    for (const wchar_t* name : {L"candidates",L"notes",L"predictions",L"clauses"}) {
        json += JsonString(name) + ":[";
        const auto& values = view[name].array;
        for (size_t i=0;i<values.size();++i) { if (i) json += ','; json += JsonString(values[i].Str()); }
        json += "],";
    }
    for (const wchar_t* name : {L"suggestion",L"meaning",L"typed",L"text"}) json += JsonString(name) + ":" + JsonString(view[name].Str()) + ',';
    for (const wchar_t* name : {L"selectedIndex",L"selectedPrediction",L"selectedClause"}) json += JsonString(name) + ":" + std::to_string(view[name].Int()) + ',';
    json += std::string("\"converting\":") + (view[L"converting"].Bool() ? "true}" : "false}");
    return json;
}
}
void* RustServiceCreateCandidates(RustServiceState* state,void* owner,void (__cdecl *select)(void*,int,int)) {
    return api().serviceCreateCandidates?api().serviceCreateCandidates(state,owner,select):nullptr;
}
void RustServiceShowCandidates(RustServiceState* state,TfEditCookie cookie,ITfContext* context,ITfComposition* composition,const JsonValue& view) {
    if (!api().serviceShowCandidates) return;
    const auto json=ViewJson(view);
    api().serviceShowCandidates(state,context,composition,cookie,json.c_str());
}
void RustServiceHideCandidates(RustServiceState* state) {if (api().serviceHideCandidates) api().serviceHideCandidates(state);}
void RustServiceDropCandidates(RustServiceState* state) {if (api().serviceDropCandidates) api().serviceDropCandidates(state);}
} // namespace meltype
