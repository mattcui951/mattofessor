"""Prints Mattofessor's windows (position, size, topmost, no-activate, visible), the mock game
window, and which window has focus. Used to check overlay placement without screenshots.

    py -I tools/inspect_windows.py
"""

import ctypes
import ctypes.wintypes as wt

user32 = ctypes.windll.user32
kernel32 = ctypes.windll.kernel32
GWL_EXSTYLE = -20
WS_EX_TOPMOST = 0x8
WS_EX_NOACTIVATE = 0x08000000


def exe_of(hwnd):
    pid = wt.DWORD()
    user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
    h = kernel32.OpenProcess(0x1000, False, pid.value)
    if not h:
        return ""
    buf = ctypes.create_unicode_buffer(1024)
    n = wt.DWORD(1024)
    kernel32.QueryFullProcessImageNameW(h, 0, buf, ctypes.byref(n))
    kernel32.CloseHandle(h)
    return buf.value.rsplit("\\", 1)[-1]


def title_of(hwnd):
    buf = ctypes.create_unicode_buffer(256)
    user32.GetWindowTextW(hwnd, buf, 256)
    return buf.value


def main():
    fg = user32.GetForegroundWindow()
    rows = []

    @ctypes.WINFUNCTYPE(ctypes.c_bool, wt.HWND, wt.LPARAM)
    def cb(hwnd, _):
        exe = exe_of(hwnd)
        if exe.lower() in ("mattofessor.exe", "mattofessor mock game.exe"):
            r = wt.RECT()
            user32.GetWindowRect(hwnd, ctypes.byref(r))
            ex = user32.GetWindowLongPtrW(hwnd, GWL_EXSTYLE)
            rows.append(
                f"{exe:28} {title_of(hwnd)[:24]:24} visible={bool(user32.IsWindowVisible(hwnd))!s:5} "
                f"rect=({r.left},{r.top}) {r.right - r.left}x{r.bottom - r.top} topmost={bool(ex & WS_EX_TOPMOST)!s:5} "
                f"noactivate={bool(ex & WS_EX_NOACTIVATE)!s:5} focused={hwnd == fg}"
            )
        return True

    user32.EnumWindows(cb, 0)
    print("\n".join(rows) or "no windows")
    print("foreground:", exe_of(fg), "|", title_of(fg))


if __name__ == "__main__":
    main()
