# SPDX-License-Identifier: GPL-3.0-or-later
"""Windows acceptance test; types only into Edit controls created by this process.

Run on an unlocked interactive desktop. It refuses to send keys if its own
window is not foreground. The application uses an isolated data directory.
"""
import argparse
import ctypes as c
from ctypes import wintypes as w
import json
from pathlib import Path
import subprocess
import tempfile
import threading
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--unit-host", action="store_true", required=True, help="Drive the ignored Rust acceptance host; only tagged keys to this process bypass injection exclusion")
    args = parser.parse_args()
    user = c.WinDLL("user32", use_last_error=True)
    kernel = c.WinDLL("kernel32", use_last_error=True)
    imm = c.WinDLL("imm32", use_last_error=True)
    imm.ImmDisableIME.argtypes = [w.DWORD]
    imm.ImmDisableIME.restype = w.BOOL
    ptr = c.c_void_p
    proc_type = c.WINFUNCTYPE(c.c_ssize_t, ptr, w.UINT, c.c_size_t, c.c_ssize_t)
    signatures = {
        "CreateWindowExW": (ptr, [w.DWORD, w.LPCWSTR, w.LPCWSTR, w.DWORD, c.c_int, c.c_int, c.c_int, c.c_int, ptr, ptr, ptr, ptr]),
        "DefWindowProcW": (c.c_ssize_t, [ptr, w.UINT, c.c_size_t, c.c_ssize_t]),
        "SendMessageW": (c.c_ssize_t, [ptr, w.UINT, c.c_size_t, c.c_ssize_t]),
        "SetFocus": (ptr, [ptr]), "GetFocus": (ptr, []),
        "LoadKeyboardLayoutW": (ptr, [w.LPCWSTR, w.UINT]), "GetForegroundWindow": (ptr, []),
        "SetForegroundWindow": (w.BOOL, [ptr]),
        "GetWindowThreadProcessId": (w.DWORD, [ptr, ptr]),
        "AttachThreadInput": (w.BOOL, [w.DWORD, w.DWORD, w.BOOL]), "ShowWindow": (w.BOOL, [ptr, c.c_int]),
        "GetWindowTextW": (c.c_int, [ptr, w.LPWSTR, c.c_int]),
        "SetWindowTextW": (w.BOOL, [ptr, w.LPCWSTR]),
        "DestroyWindow": (w.BOOL, [ptr]),
        "EnumWindows": (w.BOOL, [ptr, c.c_ssize_t]),
        "EnumChildWindows": (w.BOOL, [ptr, ptr, c.c_ssize_t]),
        "GetWindowRect": (w.BOOL, [ptr, c.POINTER(w.RECT)]),
        "GetCursorPos": (w.BOOL, [c.POINTER(w.POINT)]),
        "SetCursorPos": (w.BOOL, [c.c_int, c.c_int]),
        "GetAsyncKeyState": (c.c_short, [c.c_int]),
        "IsWindowVisible": (w.BOOL, [ptr]),
        "GetClassNameW": (c.c_int, [ptr, w.LPWSTR, c.c_int]),
        "GetMessageW": (w.BOOL, [ptr, ptr, w.UINT, w.UINT]),
        "TranslateMessage": (w.BOOL, [ptr]), "DispatchMessageW": (c.c_ssize_t, [ptr]),
        "PostQuitMessage": (None, [c.c_int]),
    }
    for name, (restype, argtypes) in signatures.items():
        fn = getattr(user, name)
        fn.restype, fn.argtypes = restype, argtypes
    kernel.GetModuleHandleW.restype = ptr
    kernel.GetModuleHandleW.argtypes = [w.LPCWSTR]

    class WindowClass(c.Structure):
        _fields_ = [("style", w.UINT), ("proc", proc_type), ("extra_class", c.c_int),
                    ("extra_window", c.c_int), ("instance", ptr), ("icon", ptr),
                    ("cursor", ptr), ("background", ptr), ("menu", w.LPCWSTR), ("name", w.LPCWSTR)]

    class Keyboard(c.Structure):
        _fields_ = [("vk", w.WORD), ("scan", w.WORD), ("flags", w.DWORD),
                    ("time", w.DWORD), ("extra", c.c_size_t)]

    class Payload(c.Union):
        _fields_ = [("key", Keyboard), ("padding", c.c_byte * (32 if c.sizeof(ptr) == 8 else 24))]

    class Mouse(c.Structure):
        _fields_ = [("dx", w.LONG), ("dy", w.LONG), ("data", w.DWORD),
                    ("flags", w.DWORD), ("time", w.DWORD), ("extra", c.c_size_t)]

    class Input(c.Structure):
        _fields_ = [("type", w.DWORD), ("payload", Payload)]

    user.SendInput.argtypes = [w.UINT, c.POINTER(Input), c.c_int]
    user.SendInput.restype = w.UINT
    user.RegisterClassW.argtypes = [c.POINTER(WindowClass)]
    ready = threading.Event()
    handles = {}

    @proc_type
    def wndproc(hwnd, message, wp, lp):
        if message == 0x8001:
            user.SetFocus(handles["password" if wp == 2 else "second" if wp else "edit"])
            return 0
        if message == 0x8002:
            user.DestroyWindow(hwnd)
            return 0
        if message == 2:
            user.PostQuitMessage(0)
            return 0
        return user.DefWindowProcW(hwnd, message, wp, lp)

    def ui():
        assert imm.ImmDisableIME(kernel.GetCurrentThreadId()), "Could not isolate the owned test UI from the registered IME"
        user.LoadKeyboardLayoutW("00000409", 1)  # Only this test UI thread.
        instance = kernel.GetModuleHandleW(None)
        wc = WindowClass(0, wndproc, 0, 0, instance, None, None, None, None, "MeltypePortableTest")
        assert user.RegisterClassW(c.byref(wc)), c.get_last_error()
        hwnd = user.CreateWindowExW(0, wc.name, "Meltype portable input acceptance (owned test)",
                                    0x10CF0000, 150, 180, 700, 300, None, None, instance, None)
        handles["window"] = hwnd
        for i, name in enumerate(["edit", "second", "password"]):
            handles[name] = user.CreateWindowExW(0, "EDIT", "", 0x50810080 | (0x20 if name == "password" else 0),
                                                20, 30 + i * 60, 600, 40, hwnd, None, instance, None)
        user.ShowWindow(hwnd, 5)
        assert hwnd and all(handles.values()), f"Window creation failed: {c.get_last_error()}"
        foreground_thread = user.GetWindowThreadProcessId(user.GetForegroundWindow(), None)
        current_thread = kernel.GetCurrentThreadId()
        attached = user.AttachThreadInput(current_thread, foreground_thread, True)
        try:
            user.SetForegroundWindow(hwnd)
            user.SetFocus(handles["edit"])
        finally:
            if attached:
                user.AttachThreadInput(current_thread, foreground_thread, False)
        ready.set()
        msg = w.MSG()
        while user.GetMessageW(c.byref(msg), None, 0, 0) > 0:
            user.TranslateMessage(c.byref(msg))
            user.DispatchMessageW(c.byref(msg))

    def key(vk, up=False):
        assert user.GetForegroundWindow() == handles["window"], "Test window lost foreground; no keys sent"
        value = Input(1, Payload(key=Keyboard(vk, 0, 2 if up else 0, 0, 0x4d525054)))
        assert user.SendInput(1, c.byref(value), c.sizeof(value)) == 1, c.get_last_error()
        time.sleep(.025)

    def press(vk):
        key(vk)
        key(vk, True)

    def toggle():
        key(0x11)
        key(0x10)
        press(ord("K"))
        key(0x10, True)
        key(0x11, True)
        time.sleep(.5)

    def type_text(text):
        for char in text:
            press(ord(char.upper()))
        time.sleep(.3)

    def text(name):
        value = c.create_unicode_buffer(1024)
        user.GetWindowTextW(handles[name], value, len(value))
        return value.value

    def expect(name, expected):
        deadline = time.monotonic() + 4
        while time.monotonic() < deadline and text(name) != expected:
            time.sleep(.05)
        assert text(name) == expected, f"{name}: expected {expected!r}, got {text(name)!r}"

    with tempfile.TemporaryDirectory(prefix="meltype-portable-test-") as directory:
        data = Path(directory)
        (data / "config.json").write_text(json.dumps({"Mode": "Keyboard", "WelcomeShown": True, "TsfIntroduced": True, "AutoUpdate": False, "ToggleKeyboardShortcuts": "Ctrl+Shift+K"}), encoding="utf-8")
        ticks = int(time.time() * 10_000_000) + 621355968000000000
        predictions = {"橋の近くで待っています": "はしのちかくでまっています", "箸をお願いします": "はしをおねがいします"}
        (data / "phrases.txt").write_text("".join(f"{reading}\t{word}\t2\t{ticks}\n" for word, reading in sorted(predictions.items())), encoding="utf-8")
        import os
        env = dict(os.environ, MELTYPE_RUST_DATA_DIR=directory, MELTYPE_PORTABLE_TEST_PID=str(os.getpid()))
        process = subprocess.Popen([str(args.binary.resolve()), *( ["--ignored", "--exact", "portable_keyboard::tests::external_input_host", "--nocapture"] if args.unit_host else ["--portable-keyboard"] )], env=env)
        thread = threading.Thread(target=ui, daemon=True)
        try:
            time.sleep(3)
            assert process.poll() is None, "Application did not start"
            thread.start()
            assert ready.wait(5), "Test window unavailable"
            time.sleep(1)
            # Recover only the modifiers used by this fixture, including after
            # an interrupted previous run; keep the destination owned here.
            for vk in [0x10, 0x11, 0x12]:
                key(vk, True)
            type_text("abc")
            expect("edit", "abc")
            user.SetWindowTextW(handles["edit"], "")
            type_text("a" * 140)
            expect("edit", "a" * 140)
            user.SetWindowTextW(handles["edit"], "")
            toggle()
            deadline = time.monotonic() + 10
            while not (data / "ready-host").exists() and time.monotonic() < deadline:
                time.sleep(.05)
            assert (data / "ready-host").exists(), "Input host did not finish UI Automation initialization"
            print("PASS: the configured Ctrl+Shift+K shortcut enables input")
            type_text("nihongo")
            expect("edit", "")
            press(13)
            expect("edit", "にほんご")
            print("PASS: external Edit receives buffered Japanese commit")
            user.SetWindowTextW(handles["edit"], "")
            type_text("a" * 140)
            press(0x79)  # F10: compare the complete raw input, not its preview.
            press(13)
            expect("edit", "a" * 140)
            print("PASS: input beyond the old 128-character limit is retained and committed")
            user.SetWindowTextW(handles["edit"], "")
            type_text("nihongo")
            key(0x11)
            press(0x4F)  # Ctrl+O edits the composition as F10.
            key(0x11, True)
            press(13)
            expect("edit", "nihongo")
            user.SetWindowTextW(handles["edit"], "")
            type_text("nihongo")
            key(0x11)
            press(0x23)  # Ctrl+End passes to the editor after committing.
            key(0x11, True)
            expect("edit", "にほんご")
            # The Unicode commit arrives before the following shortcut's key
            # edges. Assert its completed state, rather than that intermediate
            # point in the asynchronous batch.
            deadline = time.monotonic() + 4
            while user.GetAsyncKeyState(0x11) & 0x8000 and time.monotonic() < deadline:
                time.sleep(.01)
            assert not (user.GetAsyncKeyState(0x11) & 0x8000), "Control remained pressed after the shortcut"
            print("PASS: Control editing and a Control shortcut preserve the buffered input")
            user.SetWindowTextW(handles["edit"], "")
            type_text("nihon")
            user.SendMessageW(handles["window"], 0x8001, 1, 0)
            time.sleep(.5)
            type_text("go")
            press(13)
            expect("edit", "")
            expect("second", "ご")
            print("PASS: same-window focus change cancels old composition")
            user.SendMessageW(handles["window"], 0x8001, 2, 0)
            time.sleep(.5)
            type_text("abc")
            expect("password", "abc")
            print("PASS: password Edit passes raw input without conversion")
            user.SendMessageW(handles["second"], 0xCF, 1, 0)  # EM_SETREADONLY
            user.SendMessageW(handles["window"], 0x8001, 1, 0)
            time.sleep(.5)
            type_text("ghi")
            expect("second", "ご")
            user.SendMessageW(handles["second"], 0xCF, 0, 0)
            user.SendMessageW(handles["window"], 0x8001, 0, 0)
            time.sleep(.5)
            type_text("ka")
            press(13)
            expect("edit", "か")
            print("PASS: read-only input is excluded and editing resumes without stale composition")
            user.SetWindowTextW(handles["edit"], "")
            type_text("hashi")
            popup_text = []
            candidate_windows = {}
            enum_proc = c.WINFUNCTYPE(w.BOOL, ptr, c.c_ssize_t)
            @enum_proc
            def inspect_popup(hwnd, _):
                pid = w.DWORD()
                user.GetWindowThreadProcessId(hwnd, c.byref(pid))
                if pid.value == process.pid and user.IsWindowVisible(hwnd):
                    classname = c.create_unicode_buffer(128)
                    user.GetClassNameW(hwnd, classname, len(classname))
                    if classname.value == "Static":
                        value = c.create_unicode_buffer(4096)
                        user.GetWindowTextW(hwnd, value, len(value))
                        popup_text.append(value.value)
                        if value.value in predictions or any(value.value.endswith(" " + word) for word in predictions):
                            candidate_windows[hwnd] = value.value
                        user.EnumChildWindows(hwnd, inspect_child, 0)
                return True
            @enum_proc
            def inspect_child(hwnd, _):
                if user.IsWindowVisible(hwnd):
                    value = c.create_unicode_buffer(4096)
                    user.GetWindowTextW(hwnd, value, len(value))
                    popup_text.append(value.value)
                    if any(value.value.endswith(" " + word) for word in predictions):
                        candidate_windows[hwnd] = value.value
                return True
            deadline = time.monotonic() + 4
            while time.monotonic() < deadline:
                popup_text.clear()
                candidate_windows.clear()
                user.EnumWindows(inspect_popup, 0)
                if all(any(word in label for label in popup_text) for word in predictions): break
                time.sleep(.1)
            assert all(any(word in label for label in popup_text) for word in predictions), f"Predictive phrases were not rendered in the real popup: {popup_text!r}"
            press(9)
            press(13)
            deadline = time.monotonic() + 4
            while text("edit") not in predictions and time.monotonic() < deadline: time.sleep(.05)
            assert text("edit") in predictions, "Tab/Enter did not commit a displayed predictive phrase"
            print("PASS: real popup shows learned predictive phrases; Tab/Enter commits the selected phrase")
            user.SetWindowTextW(handles["edit"], "")
            type_text("hashi")
            popup_text.clear()
            candidate_windows.clear()
            user.EnumWindows(inspect_popup, 0)
            assert candidate_windows, "No clickable candidate rows were rendered"
            row, label = next(iter(candidate_windows.items()))
            expected = next(word for word in predictions if label.endswith(" " + word))
            rect, previous_cursor = w.RECT(), w.POINT()
            assert user.GetWindowRect(row, c.byref(rect))
            assert user.GetCursorPos(c.byref(previous_cursor))
            assert user.GetForegroundWindow() == handles["window"], "Owned editor lost focus before candidate click"
            try:
                assert user.SetCursorPos((rect.left + rect.right) // 2, (rect.top + rect.bottom) // 2)
                for flag in [2, 4]:  # LEFTDOWN / LEFTUP, only on the owned host's candidate.
                    value = Input()
                    mouse = Mouse(0, 0, 0, flag, 0, 0x4d525054)
                    c.memmove(c.byref(value.payload), c.byref(mouse), c.sizeof(mouse))
                    assert user.SendInput(1, c.byref(value), c.sizeof(value)) == 1
                    time.sleep(.1)
            finally:
                user.SetCursorPos(previous_cursor.x, previous_cursor.y)
            expect("edit", expected)
            assert user.GetForegroundWindow() == handles["window"], "Candidate click stole the editor focus"
            print("PASS: mouse selects and commits the displayed predictive phrase without stealing focus")
            if args.unit_host:
                (data / "stop-host").touch()
            else:
                result = subprocess.run([str(args.binary.resolve()), "--startup-action", "stop-tray"], env=env, timeout=20)
                assert result.returncode == 0
            process.wait(timeout=20)
            assert process.returncode == 0
            print("PASS: cooperative shutdown removes input hooks")
        finally:
            if process.poll() is None:
                # This subprocess belongs exclusively to this isolated test.
                process.terminate()
                process.wait(timeout=10)
            if handles.get("window"):
                if user.GetForegroundWindow() == handles["window"]:
                    for vk in [0x10, 0x11, 0x12]:
                        key(vk, True)
                user.SendMessageW(handles["window"], 0x8002, 0, 0)
                thread.join(timeout=5)


if __name__ == "__main__":
    main()
