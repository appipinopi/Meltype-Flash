// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
// Production TSF key entry points and callback ABI, no real input injection.
#include "../../../native/tip/TextService.h"
#include <cassert>
#include <cstdio>
using namespace meltype;
namespace {
struct RustKeySnapshot {
    void* composition; void* context;
    int disabled,open,detection,english;
    unsigned int staleTries,staleVk;
    unsigned long long staleTime;
    int fullWidthSpace;
};
struct RustKeyCallbacks {
    void* owner; IUnknown* lifetime;
    void (__cdecl *read)(void*,RustKeySnapshot*);
    void (__cdecl *update)(void*,unsigned int,unsigned long long,unsigned long long);
    void (__cdecl *resume)(void*,void*);
    int (__cdecl *commitNow)(void*,int);
    void (__cdecl *commitAsync)(void*,int);
    void (__cdecl *abandon)(void*);
    int (__cdecl *ready)(void*);
    int (__cdecl *secret)(void*,unsigned int,void*,int);
    void (__cdecl *pass)(void*,unsigned int);
};
bool RustWouldEat(ITfContext*,UINT,LPARAM,bool,const RustKeyCallbacks&,wchar_t&);
bool RustWouldEat(ITfContext* context,UINT key,LPARAM message,bool test,const RustKeyCallbacks& callbacks,wchar_t& ch) {
    using F=int (__cdecl *)(void*,unsigned int,unsigned long long,int,const RustKeyCallbacks*,wchar_t*);
    auto fn=reinterpret_cast<F>(GetProcAddress(GetModuleHandleW(L"meltype_native.dll"),"meltype_tsf_would_eat"));assert(fn);
    return fn(context,key,static_cast<ULONG_PTR>(message),test?1:0,&callbacks,&ch)!=0;
}
int RustKeySecret(void* token,ITfContext* context,UINT key,bool test,void* owner,IUnknown* lifetime,int (__cdecl *reader)(void*,void*,int*),int& field) {
    using F=int (__cdecl *)(void*,void*,unsigned int,int,void*,void*,int (__cdecl *)(void*,void*,int*),int*);
    auto fn=reinterpret_cast<F>(GetProcAddress(GetModuleHandleW(L"meltype_native.dll"),"meltype_tsf_key_secret"));assert(fn);
    return fn(token,context,key,test?1:0,owner,lifetime,reader,&field);
}
void* methods[17] = {};
struct Context {
    void** table=methods;
    ULONG refs=1;
    static HRESULT STDMETHODCALLTYPE Query(Context*,REFIID,void** out) {*out=nullptr;return E_NOINTERFACE;}
    static ULONG STDMETHODCALLTYPE Add(Context* self) {return ++self->refs;}
    static ULONG STDMETHODCALLTYPE Release(Context* self) {return --self->refs;}
};
struct Keyboard {
    BYTE saved[256] = {};
    Keyboard() {assert(GetKeyboardState(saved));BYTE clear[256]={};assert(SetKeyboardState(clear));}
    ~Keyboard() {SetKeyboardState(saved);}
};
struct State {TextService* service;RustKeySnapshot snapshot={};bool checked=false;int queries=0,passes=0;};
}
int wmain() {
    MSG message={};PeekMessageW(&message,nullptr,0,0,PM_NOREMOVE);
    Keyboard keyboard;
    methods[0]=reinterpret_cast<void*>(Context::Query);
    methods[1]=reinterpret_cast<void*>(Context::Add);
    methods[2]=reinterpret_cast<void*>(Context::Release);
    Context document;
    auto* context=reinterpret_cast<ITfContext*>(&document);
    auto* service=new TextService();
    BOOL eaten=TRUE;
    assert(service->OnTestKeyDown(context,VK_SHIFT,0,&eaten)==S_OK && !eaten);
    assert(service->OnTestKeyDown(context,VK_KANJI,0,&eaten)==S_OK && eaten);
    assert(service->OnKeyDown(context,VK_KANJI,0,&eaten)==S_OK && eaten);
    assert(service->OnTestKeyUp(context,VK_KANJI,0,&eaten)==S_OK && eaten);
    assert(service->OnKeyUp(context,VK_KANJI,0,&eaten)==S_OK && eaten);
    assert(service->OnTestKeyUp(context,VK_KANJI,0,&eaten)==S_OK && !eaten);
    assert(service->OnKeyDown(context,VK_KANJI,0,&eaten)==S_OK && eaten);
    assert(service->OnSetFocus(FALSE)==S_OK);
    assert(service->OnKeyUp(context,VK_KANJI,0,&eaten)==S_OK && !eaten);
    assert(service->OnTestKeyUp(context,static_cast<WPARAM>(-1),0,&eaten)==S_OK && !eaten);
    assert(RustKeyFlags(VK_SHIFT,0,false)&1);
    assert(RustKeyFlags(VK_KANJI,0,false)&2);
    BYTE shifted[256]={};shifted[VK_SHIFT]=shifted[VK_LSHIFT]=0x80;
    assert(SetKeyboardState(shifted));
    assert(RustKeyDown(VK_SHIFT) && RustKeyChar(VK_SPACE,0)==L' ');
    void* keyState=RustKeyStateCreate();
    assert(keyState);
    RustKeyStale stale={};
    RustKeyStaleWrite(keyState,3,65,0x123456789abcdef0ULL);
    RustKeyStaleRead(keyState,stale);
    assert(stale.tries==3 && stale.key==65 && stale.time==0x123456789abcdef0ULL);
    RustKeyStaleClear(keyState);
    RustKeyStaleRead(keyState,stale);
    assert(stale.tries==0 && stale.key==0 && stale.time==0x123456789abcdef0ULL);
    int reads=0,privateField=0;
    auto reader=[](void* p,void*,int* field)->int {++*static_cast<int*>(p);*field=1;return -1;};
    assert(RustKeySecret(keyState,context,65,true,&reads,service->ServiceInterface<ITfTextInputProcessorEx>(),reader,privateField)==-1);
    assert(privateField==1 && reads==1 && document.refs==2);
    assert(RustKeySecret(keyState,context,65,false,&reads,service->ServiceInterface<ITfTextInputProcessorEx>(),reader,privateField)==-1);
    assert(privateField==1 && reads==1 && document.refs==1);
    DWORD error=0;
    auto sender=[](void*,const INPUT* inputs,unsigned int count)->unsigned int {
        assert(count==2);
        assert(inputs[0].type==INPUT_KEYBOARD && inputs[0].ki.wVk==65 && inputs[0].ki.dwFlags==0);
        assert(inputs[1].ki.dwFlags==KEYEVENTF_KEYUP && inputs[0].ki.dwExtraInfo==0x4d4c5450);
        return 2;  // Controlled transport: no key is injected.
    };
    assert(RustReinject(keyState,65,service->ServiceInterface<ITfTextInputProcessorEx>(),error,nullptr,sender)==1 && error==0);
    LPARAM extra=SetMessageExtraInfo(0x4d4c5450);
    assert(RustIsReinjected(keyState,65) && !RustIsReinjected(keyState,65));
    SetMessageExtraInfo(extra);
    auto failure=[](void*,const INPUT*,unsigned int)->unsigned int {SetLastError(ERROR_ACCESS_DENIED);return 0;};
    assert(RustReinject(keyState,66,service->ServiceInterface<ITfTextInputProcessorEx>(),error,nullptr,failure)==0 && error==ERROR_ACCESS_DENIED);
    RustKeyStateReset(keyState);RustKeyStateDestroy(keyState);
    State state{service};state.snapshot.open=1;
    RustKeyCallbacks callbacks={
        &state,service->ServiceInterface<ITfTextInputProcessorEx>(),
        [](void* p,RustKeySnapshot* out) {*out=static_cast<State*>(p)->snapshot;},
        [](void* p,unsigned int op,unsigned long long a,unsigned long long) {
            auto* s=static_cast<State*>(p);
            if (op==0) s->checked=false;
            else if (op==4) s->checked=a!=0;
        },
        [](void*,void*) {},
        [](void*,int)->int {assert(false);return 0;},
        [](void*,int) {assert(false);},
        [](void*) {assert(false);},
        [](void* p)->int {
            auto* s=static_cast<State*>(p);++s->queries;
            s->service->Release();s->service=nullptr;
            assert(g_locks==1);return 1;
        },
        [](void* p,unsigned int key,void*,int test)->int {assert(!static_cast<State*>(p)->service && key==VK_SPACE && test && g_locks==1);return 0;},
        [](void* p,unsigned int reason) {assert(reason==0 && g_locks==1);++static_cast<State*>(p)->passes;},
    };
    wchar_t ch=0;
    assert(RustWouldEat(context,VK_SPACE,0,true,callbacks,ch));
    assert(ch==L' ' && state.checked && state.queries==1 && state.passes==1);
    assert(document.refs==1 && g_locks==0);
    puts("Actual TSF key adapters: production test/down/up, keyboard/callback ABI, cached private metadata, controlled replay/marker, Shift+Space and owner release passed");
}
