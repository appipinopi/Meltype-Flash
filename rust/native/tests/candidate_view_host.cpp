// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
// Exercise the actual C++ shell's reply serialization and Rust projection.
#include "../../../native/tip/TextService.h"
#include <cassert>
#include <cmath>
#include <cstdio>
using namespace meltype;
namespace {
BOOL CALLBACK Find(HWND window,LPARAM data) {
    wchar_t name[128] = {};
    GetClassNameW(window,name,ARRAYSIZE(name));
    if (wcscmp(name,L"Meltype-flashCandidateWindow") == 0 && IsWindowVisible(window)) {
        *reinterpret_cast<HWND*>(data) = window; return FALSE;
    }
    return TRUE;
}
HWND Window() {
    HWND result = nullptr;
    EnumThreadWindows(GetCurrentThreadId(),Find,reinterpret_cast<LPARAM>(&result));
    return result;
}
void Height(HWND window,int logical) {
    assert(window);
    RECT rect = {};
    assert(GetClientRect(window,&rect));
    double scale = GetDpiForWindow(window) / 96.0;
    assert(rect.bottom == static_cast<int>(std::round(logical * scale)));
}
}
int wmain() {
    assert(SUCCEEDED(CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED)));
    HWND foreground = GetForegroundWindow();
    int selected = -1;
    bool prediction = false;
    {
        RustServiceStateStorage storage;
        struct Selection {int* selected;bool* prediction;} selection{&selected,&prediction};
        assert(RustServiceCreateCandidates(storage.value,&selection,[](void* p,int index,int predicted) {
            auto* event=static_cast<Selection*>(p);*event->selected=index;*event->prediction=predicted!=0;
        }));
        JsonValue view;
        assert(ParseJson("{\"predictions\":[\"日本\",\"😀\"],\"selectedPrediction\":1,\"notes\":[\"unused\"],\"meaning\":\"unused\"}",view));
        RustServiceShowCandidates(storage.value,7,nullptr,nullptr,view);
        assert(Window());
        HWND window = Window();
        Height(window,8+2*32);
        assert(GetForegroundWindow() == foreground);
        int y = static_cast<int>((4+32+16)*GetDpiForWindow(window)/96.0);
        SendMessageW(window,WM_LBUTTONUP,0,MAKELPARAM(10,y));
        assert(selected==1 && prediction);
        assert(ParseJson("{\"converting\":true,\"candidates\":[\"日本\"],\"typed\":\"ni\\\"hon\",\"text\":\"日本\",\"meaning\":\"unused\"}",view));
        RustServiceShowCandidates(storage.value,7,nullptr,nullptr,view);
        assert(Window());
        Height(Window(),8+22);
        assert(ParseJson("{\"converting\":true,\"candidates\":[\"日本\"],\"typed\":\"日本\",\"text\":\"日本\"}",view));
        RustServiceShowCandidates(storage.value,7,nullptr,nullptr,view);
        assert(!Window());
    }
    assert(Window()==nullptr && g_locks==0);
    CoUninitialize();
    puts("Actual TSF candidate adapter: Unicode/escaped JSON, predictions, click routing, typed-only geometry, suppression and focus passed");
}
