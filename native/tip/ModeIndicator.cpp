// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//
// 入力モード (あ / A) をカーソルの近くに 1.2 秒だけ出す。
// フォーカスを奪わず、クリックは下のウィンドウへ通す。
#include "Globals.h"
#include "ModeIndicator.h"

#include <windowsx.h>

namespace meltype {
namespace {

constexpr wchar_t kClassName[] = L"MeltypeModeIndicator";
constexpr UINT_PTR kHideTimer = 1;
constexpr int kSize = 34;
constexpr UINT kHideMilliseconds = 1200;

HWND g_window = nullptr;
bool g_japanese = true;

void Paint(HWND window) {
    PAINTSTRUCT paint{};
    HDC dc = BeginPaint(window, &paint);
    RECT client{};
    GetClientRect(window, &client);
    const COLORREF background = g_japanese ? RGB(76, 160, 255) : RGB(110, 110, 120);
    HBRUSH brush = CreateSolidBrush(background);
    FillRect(dc, &client, brush);
    DeleteObject(brush);

    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, RGB(255, 255, 255));
    HFONT font = CreateFontW(-22, 0, 0, 0, FW_BOLD, FALSE, FALSE, FALSE,
                             DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
                             CLEARTYPE_QUALITY, DEFAULT_PITCH, L"Yu Gothic UI");
    HGDIOBJ previous = SelectObject(dc, font);
    const wchar_t* text = g_japanese ? L"あ" : L"A";
    DrawTextW(dc, text, -1, &client, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    SelectObject(dc, previous);
    DeleteObject(font);
    EndPaint(window, &paint);
}

LRESULT CALLBACK WindowProc(HWND window, UINT message, WPARAM wparam, LPARAM lparam) {
    switch (message) {
        case WM_PAINT:
            Paint(window);
            return 0;
        case WM_TIMER:
            if (wparam == kHideTimer) {
                KillTimer(window, kHideTimer);
                ShowWindow(window, SW_HIDE);
            }
            return 0;
        case WM_NCHITTEST:
            return HTTRANSPARENT;
        case WM_ERASEBKGND:
            return 1;
        default:
            return DefWindowProcW(window, message, wparam, lparam);
    }
}

HWND EnsureWindow() {
    if (g_window) return g_window;
    WNDCLASSEXW definition{};
    definition.cbSize = sizeof(definition);
    definition.lpfnWndProc = WindowProc;
    definition.hInstance = g_module;
    definition.hCursor = LoadCursorW(nullptr, IDC_ARROW);
    definition.lpszClassName = kClassName;
    RegisterClassExW(&definition);
    g_window = CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW |
            WS_EX_TOPMOST,
        kClassName, L"", WS_POPUP, 0, 0, kSize, kSize, nullptr, nullptr, g_module, nullptr);
    if (g_window) {
        SetLayeredWindowAttributes(g_window, 0, 235, LWA_ALPHA);
    }
    return g_window;
}

POINT CaretPoint() {
    POINT point{};
    GUITHREADINFO info{};
    info.cbSize = sizeof(info);
    if (GetGUIThreadInfo(GetCurrentThreadId(), &info) && info.hwndCaret) {
        point.x = info.rcCaret.left;
        point.y = info.rcCaret.top;
        ClientToScreen(info.hwndCaret, &point);
        return point;
    }
    GetCursorPos(&point);
    return point;
}

}  // namespace

void ShowModeIndicator(bool japanese) {
    HWND window = EnsureWindow();
    if (!window) return;
    g_japanese = japanese;

    const POINT caret = CaretPoint();
    int x = caret.x + 8;
    int y = caret.y - kSize;
    const HMONITOR monitor = MonitorFromPoint(caret, MONITOR_DEFAULTTONEAREST);
    MONITORINFO info{};
    info.cbSize = sizeof(info);
    if (GetMonitorInfoW(monitor, &info)) {
        const RECT& work = info.rcWork;
        if (x + kSize > work.right) x = work.right - kSize;
        if (x < work.left) x = work.left;
        if (y < work.top) y = caret.y + 24;
        if (y + kSize > work.bottom) y = work.bottom - kSize;
    }
    SetWindowPos(window, HWND_TOPMOST, x, y, kSize, kSize, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    InvalidateRect(window, nullptr, TRUE);
    SetTimer(window, kHideTimer, kHideMilliseconds, nullptr);
}

}  // namespace meltype
