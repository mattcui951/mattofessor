"""Saves a PNG of a screen region (for checking overlay windows over the mock game).

    py -I tools/grab.py x y w h out.png
"""

import ctypes
import ctypes.wintypes as wt
import struct
import subprocess
import sys
import zlib

x, y, w, h = map(int, sys.argv[1:5])
out = sys.argv[5]
user32, gdi32 = ctypes.windll.user32, ctypes.windll.gdi32
user32.SetProcessDPIAware()
src = user32.GetDC(None)
mem = gdi32.CreateCompatibleDC(src)
bmp = gdi32.CreateCompatibleBitmap(src, w, h)
gdi32.SelectObject(mem, bmp)
gdi32.BitBlt(mem, 0, 0, w, h, src, x, y, 0x00CC0020 | 0x40000000)


class BMI(ctypes.Structure):
    _fields_ = [("biSize", wt.DWORD), ("biWidth", wt.LONG), ("biHeight", wt.LONG), ("biPlanes", wt.WORD), ("biBitCount", wt.WORD),
                ("biCompression", wt.DWORD), ("biSizeImage", wt.DWORD), ("biXPelsPerMeter", wt.LONG), ("biYPelsPerMeter", wt.LONG),
                ("biClrUsed", wt.DWORD), ("biClrImportant", wt.DWORD)]


bi = BMI(ctypes.sizeof(BMI), w, -h, 1, 32, 0, 0, 0, 0, 0, 0)
buf = ctypes.create_string_buffer(w * h * 4)
gdi32.GetDIBits(mem, bmp, 0, h, buf, ctypes.byref(bi), 0)
raw = b"".join(b"\x00" + bytes(buf.raw[r * w * 4:(r + 1) * w * 4][i:i + 3][::-1] for i in range(0, w * 4, 4)) for r in range(h)) if False else None
rows = []
data = buf.raw
for r in range(h):
    line = bytearray(b"\x00")
    row = data[r * w * 4:(r + 1) * w * 4]
    line += bytes(v for i in range(0, len(row), 4) for v in (row[i + 2], row[i + 1], row[i]))
    rows.append(bytes(line))


def chunk(t, d):
    return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)


png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(b"".join(rows), 6)) + chunk(b"IEND", b"")
open(out, "wb").write(png)
print(out)
