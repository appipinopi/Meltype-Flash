// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
// Actual OnKeyDown -> Rust key edit -> isolated core -> TSF range write.
#include "../../../native/tip/TextService.h"
#include <algorithm>
#include <cassert>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <objbase.h>
using namespace meltype;
#ifdef MELTYPE_TEST_NATIVE_OWNER
// Exercise the actual production DLL and class factory, not compatibility code.
// 互換ヘッダーの本家IDと分け、配布するFlash固有のクラスを要求する。
constexpr CLSID CLSID_FlashTextService = {0xc540baf6,0x88b1,0x5b30,{0xb5,0x24,0x2c,0x87,0x02,0xb0,0x34,0xfc}};
HMODULE nativeShell=nullptr;
using CanUnload=HRESULT (STDAPICALLTYPE *)();
int NativeUnloadState() {
    auto test=reinterpret_cast<CanUnload>(GetProcAddress(nativeShell,"DllCanUnloadNow"));assert(test);
    return test()==S_OK?0:1;
}
unsigned int NativeLiveCount() {
    auto count=reinterpret_cast<unsigned int (__cdecl *)()>(GetProcAddress(nativeShell,"meltype_tsf_service_state_live_count"));assert(count);return count();
}
void WaitForNativeUnload() {
    const auto deadline=GetTickCount64()+5000;
    while (NativeUnloadState()!=0 && GetTickCount64()<deadline) Sleep(1);
    assert(NativeUnloadState()==0 && "worker OS threads must finish before DLL unload");
}
#define g_locks (NativeUnloadState())
class FixtureService {
    IUnknown* root=nullptr;
public:
    FixtureService() {
        nativeShell=LoadLibraryW(L"MeltypeTip.dll");assert(nativeShell);
        using GetClass=HRESULT (STDAPICALLTYPE *)(REFCLSID,REFIID,void**);
        auto get=reinterpret_cast<GetClass>(GetProcAddress(nativeShell,"DllGetClassObject"));assert(get);
        IClassFactory* factory=nullptr;
        assert(get(CLSID_FlashTextService,IID_IClassFactory,reinterpret_cast<void**>(&factory))==S_OK && factory);
        assert(factory->CreateInstance(nullptr,IID_ITfTextInputProcessorEx,reinterpret_cast<void**>(&root))==S_OK && root);
        factory->Release();
        assert(NativeLiveCount()==1);
    }
    template<class T> T* ServiceInterface() const {T* value=nullptr;assert(root->QueryInterface(__uuidof(T),reinterpret_cast<void**>(&value))==S_OK && value);value->Release();return value;}
    void Release() {assert(root->Release()==0);assert(NativeLiveCount()==0);delete this;}
};
#else
using FixtureService=TextService;
#endif
namespace {
struct PrivateProfile {
    std::filesystem::path root,path;
    PrivateProfile() {
        wchar_t temporary[MAX_PATH]={},name[MAX_PATH]={};
        DWORD count=GetTempPathW(ARRAYSIZE(temporary),temporary);
        assert(count>0 && count<ARRAYSIZE(temporary));
        root=std::filesystem::weakly_canonical(temporary);
        assert(GetTempFileNameW(temporary,L"MLK",0,name));
        path=std::filesystem::weakly_canonical(name);
        assert(path.parent_path()==root && path.filename().native().rfind(L"MLK",0)==0);
        assert(DeleteFileW(path.c_str()));
        assert(std::filesystem::create_directory(path));
    }
    ~PrivateProfile() {
        std::error_code error;
        auto resolved=std::filesystem::weakly_canonical(path,error);
        // Verify the final absolute target before recursive cleanup; never
        // follow a replaced directory outside this exact private temp child.
        if (!error && resolved==path && path.parent_path()==root && path.filename().native().rfind(L"MLK",0)==0)
            std::filesystem::remove_all(path,error);
    }
};
void* rangeMethods[25] = {};
void* contextMethods[17] = {};
void* compositionMethods[7]={};
void* managerMethods[7]={};
void* threadMethods[14]={};
struct ThreadManager {
    void** table=threadMethods;ULONG refs=1;int focusQueries=0;
    static HRESULT STDMETHODCALLTYPE Query(ThreadManager* s,REFIID iid,void** out) {
        *out=nullptr;if (iid!=IID_IUnknown) return E_NOINTERFACE;
        ++s->refs;*out=s;return S_OK;
    }
    static ULONG STDMETHODCALLTYPE Add(ThreadManager* s) {return ++s->refs;}
    static ULONG STDMETHODCALLTYPE Release(ThreadManager* s) {return --s->refs;}
    static HRESULT STDMETHODCALLTYPE Focus(ThreadManager* s,ITfDocumentMgr** out) {++s->focusQueries;*out=nullptr;return S_OK;}
};
struct Document { std::wstring text; LONG selectionStart = 0, selectionEnd = 0; int ranges = 0, selections = 0, writes = 0; ITfContext* context=nullptr; };
struct Range {
    void** table = rangeMethods;
    Document* doc;
    LONG start, end, refs = 1;
    bool owned;
    Range(Document* value,LONG first,LONG last,bool heap) : doc(value),start(first),end(last),owned(heap) { ++doc->ranges; }
    ~Range() { --doc->ranges; }
    static ULONG STDMETHODCALLTYPE Add(Range* self) { return ++self->refs; }
    static ULONG STDMETHODCALLTYPE Release(Range* self) { LONG n = --self->refs; if (!n && self->owned) delete self; return n; }
    static HRESULT STDMETHODCALLTYPE Clone(Range* self,ITfRange** out) {
        *out = reinterpret_cast<ITfRange*>(new Range(self->doc,self->start,self->end,true)); return S_OK;
    }
    static HRESULT STDMETHODCALLTYPE Collapse(Range* self,TfEditCookie cookie,TfAnchor anchor) {
        assert(cookie == 7); if (anchor == TF_ANCHOR_START) self->end = self->start; else self->start = self->end; return S_OK;
    }
    static HRESULT STDMETHODCALLTYPE Empty(Range* self,TfEditCookie cookie,BOOL* out) { assert(cookie == 7); *out = self->start == self->end; return S_OK; }
    static HRESULT STDMETHODCALLTYPE Shift(Range* self,TfEditCookie cookie,LONG amount,LONG* moved,const TF_HALTCOND*) {
        assert(cookie == 7); LONG next = std::clamp(self->start + amount,0L,self->end); *moved = next-self->start; self->start = next; return S_OK;
    }
    static HRESULT STDMETHODCALLTYPE ShiftEnd(Range* self,TfEditCookie cookie,LONG amount,LONG* moved,const TF_HALTCOND*) {
        assert(cookie==7);LONG next=std::clamp(self->end+amount,self->start,static_cast<LONG>(self->doc->text.size()));*moved=next-self->end;self->end=next;return S_OK;
    }
    static HRESULT STDMETHODCALLTYPE Read(Range* self,TfEditCookie cookie,DWORD,wchar_t* out,ULONG maximum,ULONG* count) {
        assert(cookie == 7); *count = (std::min)(maximum,static_cast<ULONG>(self->end-self->start));
        std::copy_n(self->doc->text.data()+self->start,*count,out); return S_OK;
    }
    static HRESULT STDMETHODCALLTYPE Write(Range* self,TfEditCookie cookie,DWORD,const wchar_t* value,LONG count) {
        assert(cookie == 7); self->doc->text.replace(self->start,self->end-self->start,value,count); self->end = self->start+count; ++self->doc->writes; return S_OK;
    }
    static HRESULT STDMETHODCALLTYPE CompareStart(Range* s,TfEditCookie ec,ITfRange* other,TfAnchor anchor,LONG* out) {
        assert(ec==7);auto* r=reinterpret_cast<Range*>(other);LONG right=anchor==TF_ANCHOR_START?r->start:r->end;
        *out=(s->start>right)-(s->start<right);return S_OK;
    }
    static HRESULT STDMETHODCALLTYPE CompareEnd(Range* s,TfEditCookie ec,ITfRange* other,TfAnchor anchor,LONG* out) {
        assert(ec==7);auto* r=reinterpret_cast<Range*>(other);LONG right=anchor==TF_ANCHOR_START?r->start:r->end;
        *out=(s->end>right)-(s->end<right);return S_OK;
    }
    static HRESULT STDMETHODCALLTYPE GetContext(Range* s,ITfContext** out) {*out=s->doc->context;(*out)->AddRef();return S_OK;}
};
struct Composition {
    void** table=compositionMethods;ULONG refs=0;Range* range=nullptr;ITfCompositionSink* sink=nullptr;int ended=0;
    static HRESULT STDMETHODCALLTYPE Query(Composition* s,REFIID iid,void** out) {
        *out=nullptr;if (iid!=IID_IUnknown) return E_NOINTERFACE;
        Add(s);*out=s;return S_OK;
    }
    static ULONG STDMETHODCALLTYPE Add(Composition* s) {return ++s->refs;}
    static ULONG STDMETHODCALLTYPE Release(Composition* s) {
        ULONG count=--s->refs;if (!count) {Range::Release(s->range);s->range=nullptr;s->sink->Release();s->sink=nullptr;}return count;
    }
    static HRESULT STDMETHODCALLTYPE GetRange(Composition* s,ITfRange** out) {assert(s->refs);Range::Add(s->range);*out=reinterpret_cast<ITfRange*>(s->range);return S_OK;}
    static HRESULT STDMETHODCALLTYPE End(Composition* s,TfEditCookie ec) {assert(ec==7 && s->refs);++s->ended;return S_OK;}
};
struct Manager {
    void** table=managerMethods;ULONG refs=1;Composition* composition;
    static ULONG STDMETHODCALLTYPE Add(Manager* s) {return ++s->refs;}
    static ULONG STDMETHODCALLTYPE Release(Manager* s) {return --s->refs;}
    static HRESULT STDMETHODCALLTYPE Start(Manager* s,TfEditCookie ec,ITfRange* range,ITfCompositionSink* sink,ITfComposition** out) {
        assert(ec==7 && !s->composition->refs);ITfRange* cloned=nullptr;
        Range::Clone(reinterpret_cast<Range*>(range),&cloned);auto* c=s->composition;
        c->range=reinterpret_cast<Range*>(cloned);c->sink=sink;sink->AddRef();c->refs=1;
        *out=reinterpret_cast<ITfComposition*>(c);return S_OK;
    }
};
struct Context {
    void** table = contextMethods;
    Range* selected;
    LONG refs = 1;
    Manager* manager=nullptr;
    bool rejectEdits=false;
    static ULONG STDMETHODCALLTYPE Add(Context* self) { return ++self->refs; }
    static ULONG STDMETHODCALLTYPE Release(Context* self) { return --self->refs; }
    static HRESULT STDMETHODCALLTYPE Selection(Context* self,TfEditCookie cookie,ULONG,ULONG count,TF_SELECTION* out,ULONG* fetched) {
        assert(cookie == 7 && count == 1); auto* range = self->selected;
        range->start = range->doc->selectionStart; range->end = range->doc->selectionEnd; ++range->doc->selections;
        Range::Add(range); out->range = reinterpret_cast<ITfRange*>(range); *fetched = 1; return S_OK;
    }
    static HRESULT STDMETHODCALLTYPE Query(Context* s,REFIID iid,void** out) {
        *out=nullptr;if (iid!=IID_ITfContextComposition || !s->manager) return E_NOINTERFACE;
        Manager::Add(s->manager);*out=s->manager;return S_OK;
    }
    static HRESULT STDMETHODCALLTYPE Edit(Context* s,TfClientId client,ITfEditSession* session,DWORD flags,HRESULT* result) {
        assert(client==TF_CLIENTID_NULL);
        if (s->rejectEdits) {*result=E_FAIL;return TF_E_SYNCHRONOUS;}
        assert(flags==3 || flags==7);*result=session->DoEditSession(7);return S_OK;
    }
    static HRESULT STDMETHODCALLTYPE SetSelection(Context* self,TfEditCookie cookie,ULONG count,const TF_SELECTION* value) {
        assert(cookie==7 && count==1);auto* range=reinterpret_cast<Range*>(value[0].range);
        self->selected->doc->selectionStart=range->start;self->selected->doc->selectionEnd=range->end;return S_OK;
    }
    static HRESULT STDMETHODCALLTYPE Property(Context*,REFGUID,ITfProperty** out) { *out = nullptr; return E_NOINTERFACE; }
};
}
int wmain() {
    MSG message={};PeekMessageW(&message,nullptr,0,0,PM_NOREMOVE);
    wchar_t executable[32768]={};assert(GetModuleFileNameW(nullptr,executable,ARRAYSIZE(executable)));
    PrivateProfile isolated;
    const auto& profile=isolated.path;
    {std::ofstream config(profile/L"config.json");config<<"{\"DetectionLevel\":\"Manual\",\"ConversionEngine\":\"System\",\"LearningEnabled\":false}";}
    assert(SetEnvironmentVariableW(L"MELTYPE_RUST_DATA_DIR",profile.c_str()));
    wchar_t systemExpected[512]={};
    DWORD systemCount=GetEnvironmentVariableW(L"MELTYPE_TEST_SYSTEM_NIHONGO",systemExpected,ARRAYSIZE(systemExpected));
    assert(systemCount<ARRAYSIZE(systemExpected));
    if (!systemCount) assert(SetEnvironmentVariableW(L"MELTYPE_TEST_TIP_NO_CONVERTERS",L"1"));
    else if (GetEnvironmentVariableW(L"MELTYPE_TEST_TIP_NO_CONVERTERS",nullptr,0))
        assert(SetEnvironmentVariableW(L"MELTYPE_TEST_TIP_NO_CONVERTERS",nullptr));
    BYTE saved[256]={};assert(GetKeyboardState(saved));BYTE shifted[256]={};shifted[VK_SHIFT]=shifted[VK_LSHIFT]=0x80;assert(SetKeyboardState(shifted));
    rangeMethods[1]=reinterpret_cast<void*>(Range::Add);rangeMethods[2]=reinterpret_cast<void*>(Range::Release);
    rangeMethods[3]=reinterpret_cast<void*>(Range::Read);rangeMethods[4]=reinterpret_cast<void*>(Range::Write);
    rangeMethods[8]=reinterpret_cast<void*>(Range::Shift);rangeMethods[9]=reinterpret_cast<void*>(Range::ShiftEnd);
    rangeMethods[14]=reinterpret_cast<void*>(Range::Empty);rangeMethods[15]=reinterpret_cast<void*>(Range::Collapse);rangeMethods[23]=reinterpret_cast<void*>(Range::Clone);
    rangeMethods[18]=reinterpret_cast<void*>(Range::CompareStart);rangeMethods[19]=reinterpret_cast<void*>(Range::CompareEnd);rangeMethods[24]=reinterpret_cast<void*>(Range::GetContext);
    contextMethods[0]=reinterpret_cast<void*>(Context::Query);contextMethods[1]=reinterpret_cast<void*>(Context::Add);contextMethods[2]=reinterpret_cast<void*>(Context::Release);
    contextMethods[3]=reinterpret_cast<void*>(Context::Edit);contextMethods[5]=reinterpret_cast<void*>(Context::Selection);contextMethods[6]=reinterpret_cast<void*>(Context::SetSelection);
    contextMethods[12]=reinterpret_cast<void*>(Context::Property);contextMethods[13]=reinterpret_cast<void*>(Context::Property);
    compositionMethods[1]=reinterpret_cast<void*>(Composition::Add);compositionMethods[2]=reinterpret_cast<void*>(Composition::Release);
    compositionMethods[0]=reinterpret_cast<void*>(Composition::Query);
    compositionMethods[3]=reinterpret_cast<void*>(Composition::GetRange);compositionMethods[6]=reinterpret_cast<void*>(Composition::End);
    managerMethods[1]=reinterpret_cast<void*>(Manager::Add);managerMethods[2]=reinterpret_cast<void*>(Manager::Release);managerMethods[3]=reinterpret_cast<void*>(Manager::Start);
    Document doc={L"前😀",3,3};Range selected(&doc,3,3,false);Context context={contextMethods,&selected};
    Composition composition;Manager manager={managerMethods,1,&composition};context.manager=&manager;
    doc.context=reinterpret_cast<ITfContext*>(&context);
    auto* service=new FixtureService();BOOL eaten=FALSE;
    // A cold worker may miss the bounded initial hello under host load. Passing
    // the key through then is valid; establish readiness before edit assertions.
    // Deadline/backoff behavior is covered separately by transport fixtures.
    const ULONGLONG readyDeadline=GetTickCount64()+5000;
    do {
        assert(service->ServiceInterface<ITfKeyEventSink>()->OnTestKeyDown(reinterpret_cast<ITfContext*>(&context),VK_SPACE,0,&eaten)==S_OK);
        if (eaten) break;
        Sleep(200);
    } while (GetTickCount64()<readyDeadline);
    assert(eaten && "private worker must become ready before editing");
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnKeyDown(reinterpret_cast<ITfContext*>(&context),VK_SPACE,0,&eaten)==S_OK && eaten);
    assert(doc.text==L"前😀　" && doc.writes==1 && doc.selectionStart==4 && doc.selectionEnd==4);
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnTestKeyUp(nullptr,VK_SPACE,0,&eaten)==S_OK && eaten);
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnKeyUp(nullptr,VK_SPACE,0,&eaten)==S_OK && eaten);
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnTestKeyUp(nullptr,VK_SPACE,0,&eaten)==S_OK && !eaten);
    BYTE plain[256]={};assert(SetKeyboardState(plain));
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnTestKeyDown(reinterpret_cast<ITfContext*>(&context),'A',0,&eaten)==S_OK && eaten);
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnKeyDown(reinterpret_cast<ITfContext*>(&context),'A',0,&eaten)==S_OK && eaten);
    assert(doc.text==L"前😀　あ" && composition.refs==1 && context.refs==2);
    const int endedBeforeShortcut=composition.ended;
    BYTE control[256]={};control[VK_CONTROL]=control[VK_LCONTROL]=0x80;assert(SetKeyboardState(control));
    // A non-composition shortcut commits before the original key passes through.
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnTestKeyDown(reinterpret_cast<ITfContext*>(&context),'C',0,&eaten)==S_OK && !eaten);
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnKeyDown(reinterpret_cast<ITfContext*>(&context),'C',0,&eaten)==S_OK && !eaten);
    assert(doc.text==L"前😀　あ" && composition.refs==0 && composition.ended==endedBeforeShortcut+1 && manager.refs==1);
    assert(doc.selectionStart==5 && doc.selectionEnd==5);
    assert(SetKeyboardState(plain));
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnTestKeyDown(reinterpret_cast<ITfContext*>(&context),'A',0,&eaten)==S_OK && eaten);
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnKeyDown(reinterpret_cast<ITfContext*>(&context),'A',0,&eaten)==S_OK && eaten);
    if (doc.text!=L"前😀　ああ" || composition.refs!=1) {
        fprintf(stderr,"Second A mismatch: composition refs=%lu, UTF16:",composition.refs);
        for (wchar_t unit:doc.text) fprintf(stderr," %04X",static_cast<unsigned int>(unit));
        fprintf(stderr,"; private profile=%s\n",profile.u8string().c_str());
    }
    assert(doc.text==L"前😀　ああ" && composition.refs==1);
    doc.selectionStart=doc.selectionEnd=0;
    const int endedBeforePacket=composition.ended;
    // Unicode packets pass through after committing, preserving the moved caret.
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnTestKeyDown(reinterpret_cast<ITfContext*>(&context),VK_PACKET,0,&eaten)==S_OK && !eaten);
    assert(doc.text==L"前😀　ああ" && composition.refs==0 && composition.ended==endedBeforePacket+1);
    assert(doc.selectionStart==0 && doc.selectionEnd==0);
    // Selected-text reconversion must travel through the real key sink and
    // framed Rust worker, and update only the original selected UTF-16 range.
    auto press=[&](UINT key) {
        auto* events=service->ServiceInterface<ITfKeyEventSink>();
        assert(events->OnTestKeyDown(doc.context,key,0,&eaten)==S_OK && eaten);
        assert(events->OnKeyDown(doc.context,key,0,&eaten)==S_OK && eaten);
        assert(events->OnTestKeyUp(doc.context,key,0,&eaten)==S_OK && eaten);
        assert(events->OnKeyUp(doc.context,key,0,&eaten)==S_OK && eaten);
    };
    const std::wstring selectedOriginal=L"前😀nihongo後";
    const std::wstring selectedConverted=L"前😀"+std::wstring(systemCount?systemExpected:L"にほんご")+L"後";
    for (int action=0;action<3;++action) {
        doc.text=selectedOriginal;doc.selectionStart=3;doc.selectionEnd=10;
        press(VK_CONVERT);
        if (doc.text!=selectedConverted || composition.refs!=1) {
            fprintf(stderr,"Reconversion mismatch action=%d refs=%lu, UTF16:",action,composition.refs);
            for (wchar_t unit:doc.text) fprintf(stderr," %04X",static_cast<unsigned int>(unit));
            fprintf(stderr,"\n");
        }
        // The optional expectation comes from the original C# system adapter.
        assert(doc.text==selectedConverted && composition.refs==1);
        if (action==2) press(VK_DOWN);
        press(action==1?VK_ESCAPE:VK_RETURN);
        assert(composition.refs==0);
        assert(doc.text==(action==0?selectedConverted:selectedOriginal));
    }
    const auto beforeTermination=doc.text;
    doc.selectionStart=doc.selectionEnd=static_cast<LONG>(doc.text.size());
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnTestKeyDown(doc.context,'A',0,&eaten)==S_OK && eaten);
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnKeyDown(doc.context,'A',0,&eaten)==S_OK && eaten);
    assert(doc.text==beforeTermination+L"あ" && composition.refs==1);
    // The application's own composition reference survives its notification.
    Composition::Add(&composition);
    assert(service->ServiceInterface<ITfCompositionSink>()->OnCompositionTerminated(7,reinterpret_cast<ITfComposition*>(&composition))==S_OK);
    assert(composition.refs==1 && doc.text==beforeTermination+L"あ");
    Composition::Release(&composition);
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnTestKeyDown(doc.context,'A',0,&eaten)==S_OK && eaten);
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnKeyDown(doc.context,'A',0,&eaten)==S_OK && eaten);
    assert(doc.text==beforeTermination+L"ああ" && composition.refs==1);
    assert(SetKeyboardState(control));
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnTestKeyDown(doc.context,'C',0,&eaten)==S_OK && !eaten);
    assert(doc.ranges==1 && selected.refs==1 && context.refs==1);
    // A closing field refuses both sync commit and async cleanup. The test/down
    // pair counts once; the following key releases old input and starts in the
    // new field. A later old-host termination must preserve the new preedit.
    assert(SetKeyboardState(plain));
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnTestKeyDown(doc.context,'A',0,&eaten)==S_OK && eaten);
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnKeyDown(doc.context,'A',0,&eaten)==S_OK && eaten);
    const auto abandonedText=doc.text;const int abandonedWrites=doc.writes;
    Document nextDoc={L"",0,0};Range nextSelected(&nextDoc,0,0,false);
    Context nextContext={contextMethods,&nextSelected};Composition nextComposition;
    Manager nextManager={managerMethods,1,&nextComposition};nextContext.manager=&nextManager;
    nextDoc.context=reinterpret_cast<ITfContext*>(&nextContext);context.rejectEdits=true;
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnTestKeyDown(nextDoc.context,'A',0,&eaten)==S_OK && !eaten);
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnKeyDown(nextDoc.context,'A',0,&eaten)==S_OK && !eaten);
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnTestKeyDown(nextDoc.context,'B',0,&eaten)==S_OK && eaten);
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnKeyDown(nextDoc.context,'B',0,&eaten)==S_OK && eaten);
    assert(!nextDoc.text.empty() && nextComposition.refs==1 && composition.refs==1);
    assert(doc.text==abandonedText && doc.writes==abandonedWrites);
    const auto nextPreedit=nextDoc.text;
    Composition::Add(&composition);
    assert(service->ServiceInterface<ITfCompositionSink>()->OnCompositionTerminated(7,reinterpret_cast<ITfComposition*>(&composition))==S_OK);
    Composition::Release(&composition);
    assert(composition.refs==0 && context.refs==1 && nextComposition.refs==1 && nextDoc.text==nextPreedit);
    assert(SetKeyboardState(control));
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnTestKeyDown(nextDoc.context,'C',0,&eaten)==S_OK && !eaten);
    assert(nextComposition.refs==0 && nextContext.refs==1 && nextDoc.ranges==1 && nextSelected.refs==1);
    assert(doc.ranges==1 && selected.refs==1 && manager.refs==1 && nextManager.refs==1);
    assert(SetKeyboardState(plain));
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnTestKeyDown(nextDoc.context,'A',0,&eaten)==S_OK && eaten);
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnKeyDown(nextDoc.context,'A',0,&eaten)==S_OK && eaten);
    assert(nextComposition.refs==1);
    const auto beforeFocus=nextDoc.text;const auto focusCaret=nextDoc.selectionStart;
    const auto endedBeforeFocus=nextComposition.ended;
    // Public document-focus notification commits before dropping old field
    // metadata, without reinserting text or moving the application's caret.
    assert(service->ServiceInterface<ITfThreadMgrEventSink>()->OnSetFocus(static_cast<ITfDocumentMgr*>(nullptr),nullptr)==S_OK);
    assert(nextDoc.text==beforeFocus && nextDoc.selectionStart==focusCaret && nextDoc.selectionEnd==focusCaret);
    assert(nextComposition.refs==0 && nextComposition.ended==endedBeforeFocus+1 && nextContext.refs==1);
    assert(nextDoc.ranges==1 && nextSelected.refs==1);
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnTestKeyDown(nextDoc.context,'A',0,&eaten)==S_OK && eaten);
    assert(service->ServiceInterface<ITfKeyEventSink>()->OnKeyDown(nextDoc.context,'A',0,&eaten)==S_OK && eaten);
    assert(nextComposition.refs==1);
    const auto beforeShutdown=nextDoc.text;const auto endedBeforeShutdown=nextComposition.ended;
    // Public Deactivate runs the production Rust shutdown adapter, final commit,
    // close notification and cleanup without registering/activating this IME.
    assert(service->ServiceInterface<ITfTextInputProcessorEx>()->Deactivate()==S_OK);
    assert(nextDoc.text==beforeShutdown && nextComposition.refs==0 && nextComposition.ended==endedBeforeShutdown+1);
    assert(nextContext.refs==1 && nextDoc.ranges==1 && nextSelected.refs==1);
    assert(service->ServiceInterface<ITfTextInputProcessorEx>()->Deactivate()==S_OK);
    assert(service->ServiceInterface<ITfThreadMgrEventSink>()->OnSetFocus(static_cast<ITfDocumentMgr*>(nullptr),nullptr)==S_OK);
    assert(nextComposition.ended==endedBeforeShutdown+1 && nextDoc.text==beforeShutdown);
    // Reactivate the same service against a private manager. No system TSF
    // manager is activated; uninitialized COM also prevents category creation.
    ULONG_PTR comToken=0;assert(CoGetContextToken(&comToken)==CO_E_NOTINITIALIZED);
    threadMethods[0]=reinterpret_cast<void*>(ThreadManager::Query);
    threadMethods[1]=reinterpret_cast<void*>(ThreadManager::Add);threadMethods[2]=reinterpret_cast<void*>(ThreadManager::Release);
    threadMethods[7]=reinterpret_cast<void*>(ThreadManager::Focus);
    ThreadManager thread;auto* threadInterface=reinterpret_cast<ITfThreadMgr*>(&thread);
    assert(service->ServiceInterface<ITfTextInputProcessorEx>()->ActivateEx(threadInterface,17,TF_TMAE_SECUREMODE)==S_OK);
    assert(thread.refs==2);
#ifndef MELTYPE_TEST_NATIVE_OWNER
    assert(!service->IsOpen());
#endif
    assert(service->ServiceInterface<ITfTextInputProcessorEx>()->Deactivate()==S_OK && thread.refs==1 && g_locks==1);
    assert(service->ServiceInterface<ITfTextInputProcessorEx>()->Activate(threadInterface,17)==S_OK);
    assert(thread.refs==2);
    assert(service->ServiceInterface<ITfCompartmentEventSink>()->OnChange(GUID_COMPARTMENT_KEYBOARD_OPENCLOSE)==S_OK && thread.refs==2);
#ifndef MELTYPE_TEST_NATIVE_OWNER
    assert(service->IsOpen());
#endif
    assert(service->ServiceInterface<ITfTextInputProcessorEx>()->Deactivate()==S_OK && thread.refs==1 && g_locks==1);
    assert(thread.focusQueries==2);
    service->Release();
#ifdef MELTYPE_TEST_NATIVE_OWNER
    WaitForNativeUnload();
#endif
    assert(g_locks==0 && RustServiceStateLiveCount()==0);assert(SetKeyboardState(saved));
#ifdef MELTYPE_TEST_NATIVE_OWNER
    assert(FreeLibrary(nativeShell));nativeShell=nullptr;
    // A second load must re-register its own window classes after unload.
    auto* reloaded=new FixtureService();
    assert(reloaded->ServiceInterface<ITfTextInputProcessorEx>()->ActivateEx(threadInterface,17,TF_TMAE_SECUREMODE)==S_OK);
    assert(reloaded->ServiceInterface<ITfTextInputProcessorEx>()->Deactivate()==S_OK && thread.refs==1);
    reloaded->Release();WaitForNativeUnload();assert(g_locks==0);
    assert(FreeLibrary(nativeShell));nativeShell=nullptr;
#endif
    puts("Actual key edit/commit/termination/abandonment/focus/shutdown/startup: public TSF entry, packaged core, UTF-16 caret, preedit, commits, old field, focus, idempotent shutdown and secure/normal private-manager reactivation passed");
}
