"""A stand-in game window that blocks other apps' hotkeys the way League's game window does
(keyboard raw input registered with RIDEV_NOHOTKEYS), then presses a key combo while it has focus.

    py -I tools/nohotkeys_window.py --check-registerhotkey   # shows the problem with RegisterHotKey
    py -I tools/nohotkeys_window.py --hold 3                  # just press the combo (test the app)

The combo is Ctrl+Shift+F13 (no keyboard has F13, so nothing else reacts to it). To test the app,
set its overlay hotkey to Ctrl+Shift+F13 and run a mock game (tools/mock_client.py).
"""

import argparse
import ctypes
import ctypes.wintypes as wt
import threading
import time

user32 = ctypes.windll.user32
kernel32 = ctypes.windll.kernel32

WNDPROC = ctypes.WINFUNCTYPE(ctypes.c_ssize_t, wt.HWND, wt.UINT, wt.WPARAM, wt.LPARAM)
user32.DefWindowProcW.restype = ctypes.c_ssize_t
user32.DefWindowProcW.argtypes = [wt.HWND, wt.UINT, wt.WPARAM, wt.LPARAM]
user32.CreateWindowExW.restype = wt.HWND
user32.CreateWindowExW.argtypes = [wt.DWORD, wt.LPCWSTR, wt.LPCWSTR, wt.DWORD, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int, wt.HWND, wt.HMENU, wt.HINSTANCE, wt.LPVOID]
user32.GetForegroundWindow.restype = wt.HWND

WS_OVERLAPPEDWINDOW, WS_VISIBLE = 0x00CF0000, 0x10000000
WM_HOTKEY, WM_QUIT, PM_REMOVE = 0x0312, 0x0012, 1
RIDEV_NOHOTKEYS = 0x200
MOD_CONTROL, MOD_SHIFT, MOD_NOREPEAT = 0x2, 0x4, 0x4000
VK_CONTROL, VK_SHIFT, VK_MENU, VK_F13 = 0x11, 0x10, 0x12, 0x7C
KEYEVENTF_KEYUP = 0x2


class WNDCLASSW(ctypes.Structure):
    _fields_ = [("style", wt.UINT), ("lpfnWndProc", WNDPROC), ("cbClsExtra", ctypes.c_int), ("cbWndExtra", ctypes.c_int),
                ("hInstance", wt.HINSTANCE), ("hIcon", wt.HICON), ("hCursor", wt.HANDLE), ("hbrBackground", wt.HBRUSH),
                ("lpszMenuName", wt.LPCWSTR), ("lpszClassName", wt.LPCWSTR)]


class RAWINPUTDEVICE(ctypes.Structure):
    _fields_ = [("usUsagePage", wt.USHORT), ("usUsage", wt.USHORT), ("dwFlags", wt.DWORD), ("hwndTarget", wt.HWND)]


class KEYBDINPUT(ctypes.Structure):
    _fields_ = [("wVk", wt.WORD), ("wScan", wt.WORD), ("dwFlags", wt.DWORD), ("time", wt.DWORD), ("dwExtraInfo", ctypes.c_size_t)]


class INPUT(ctypes.Structure):
    class _U(ctypes.Union):
        _fields_ = [("ki", KEYBDINPUT), ("pad", ctypes.c_byte * 32)]

    _anonymous_ = ("u",)
    _fields_ = [("type", wt.DWORD), ("u", _U)]


def key(vk, up=False):
    i = INPUT(type=1)
    i.ki = KEYBDINPUT(vk, 0, KEYEVENTF_KEYUP if up else 0, 0, 0)
    return i


def press_combo():
    seq = [key(VK_CONTROL), key(VK_SHIFT), key(VK_F13), key(VK_F13, True), key(VK_SHIFT, True), key(VK_CONTROL, True)]
    arr = (INPUT * len(seq))(*seq)
    return user32.SendInput(len(seq), arr, ctypes.sizeof(INPUT))


def pump(seconds):
    msg = wt.MSG()
    end = time.time() + seconds
    while time.time() < end:
        while user32.PeekMessageW(ctypes.byref(msg), None, 0, 0, PM_REMOVE):
            user32.TranslateMessage(ctypes.byref(msg))
            user32.DispatchMessageW(ctypes.byref(msg))
        time.sleep(0.01)


def hotkey_listener(count, ready, stop):
    """RegisterHotKey on its own thread, like a standard global-shortcut library."""
    ok = user32.RegisterHotKey(None, 1, MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT, VK_F13)
    ready.set()
    if not ok:
        count.append(-1)
        return
    msg = wt.MSG()
    while not stop.is_set():
        if user32.PeekMessageW(ctypes.byref(msg), None, 0, 0, PM_REMOVE) and msg.message == WM_HOTKEY:
            count.append(1)
        time.sleep(0.005)
    user32.UnregisterHotKey(None, 1)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check-registerhotkey", action="store_true", help="count RegisterHotKey hits with and without RIDEV_NOHOTKEYS")
    ap.add_argument("--hold", type=float, default=1.5, help="seconds to keep the window up after pressing")
    ap.add_argument("--allow-hotkeys", action="store_true", help="do not block hotkeys (control run)")
    args = ap.parse_args()

    proc = WNDPROC(lambda h, m, w, l: user32.DefWindowProcW(h, m, w, l))
    wc = WNDCLASSW(lpfnWndProc=proc, hInstance=kernel32.GetModuleHandleW(None), lpszClassName="MattofessorNoHotkeys")
    user32.RegisterClassW(ctypes.byref(wc))
    hwnd = user32.CreateWindowExW(0, "MattofessorNoHotkeys", "Stand-in game (blocks hotkeys)", WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                                  200, 200, 480, 200, None, None, wc.hInstance, None)
    count, ready, stop = [], threading.Event(), threading.Event()
    if args.check_registerhotkey:
        threading.Thread(target=hotkey_listener, args=(count, ready, stop), daemon=True).start()
        ready.wait()

    # Foreground lock: share the focused window's input state for a moment to take focus.
    focused = False
    me = kernel32.GetCurrentThreadId()
    for _ in range(10):
        fg = user32.GetWindowThreadProcessId(user32.GetForegroundWindow(), None)
        attached = fg and fg != me and user32.AttachThreadInput(me, fg, True)
        user32.BringWindowToTop(hwnd)
        user32.SetForegroundWindow(hwnd)
        if attached:
            user32.AttachThreadInput(me, fg, False)
        pump(0.3)
        focused = user32.GetForegroundWindow() == hwnd
        if focused:
            break
    print(f"stand-in focused: {focused}; hotkeys blocked: {not args.allow_hotkeys}")
    if not focused:
        print("could not take focus; not pressing anything")
        return 2
    # Like a game that has focus and reads the keyboard through raw input.
    if not args.allow_hotkeys:
        dev = RAWINPUTDEVICE(1, 6, RIDEV_NOHOTKEYS, hwnd)
        assert user32.RegisterRawInputDevices(ctypes.byref(dev), 1, ctypes.sizeof(dev)), "RegisterRawInputDevices failed"
    print(f"pressed Ctrl+Shift+F13 ({press_combo()} events)")
    pump(args.hold)
    stop.set()
    if args.check_registerhotkey:
        print("RegisterHotKey: registration failed" if -1 in count else f"RegisterHotKey fired {len(count)} time(s)")
    user32.DestroyWindow(hwnd)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
