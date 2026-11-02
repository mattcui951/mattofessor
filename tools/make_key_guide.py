"""Builds the illustrated Riot API key guide images (src/assets/riot-guide/*.webp) from
screenshots of developer.riotgames.com: crops the browser chrome, masks every API key and
account name, and draws numbered highlights on what to click.

    py -I tools/make_key_guide.py <folder with 1.png 2.webp 3.png 4.webp>
"""

import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

OUT = Path(__file__).resolve().parent.parent / "src" / "assets" / "riot-guide"
ACCENT = (0, 168, 214)
FONT = r"C:\Windows\Fonts\segoeuib.ttf"
MONO = r"C:\Windows\Fonts\consola.ttf"


def font(size, path=FONT):
    return ImageFont.truetype(path, size)


def mask(im, box, text="", size=18, fill=(232, 232, 236), color=(120, 120, 130)):
    """Cover private text with a plain field and a placeholder."""
    d = ImageDraw.Draw(im)
    d.rectangle(box, fill=fill)
    if text:
        x0, y0, x1, y1 = box
        f = font(size, MONO)
        d.text((x0 + 6, (y0 + y1) / 2), text, fill=color, font=f, anchor="lm")


def blur(im, box, radius=10):
    region = im.crop(box).filter(ImageFilter.GaussianBlur(radius))
    im.paste(region, box[:2])


def mark(im, box, n=None, pad=6, width=4, label_side="left"):
    """Rounded highlight around `box`, with a numbered badge."""
    d = ImageDraw.Draw(im)
    x0, y0, x1, y1 = box
    d.rounded_rectangle((x0 - pad, y0 - pad, x1 + pad, y1 + pad), radius=10, outline=ACCENT, width=width)
    if n is not None:
        r = 19
        cx = x0 - pad - r - 8 if label_side == "left" else x1 + pad + r + 8
        cy = (y0 + y1) / 2
        d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=ACCENT)
        d.text((cx, cy), str(n), fill="white", font=font(24), anchor="mm")


def save(im, name, width=1400):
    im = im.convert("RGB")
    if im.width > width:
        im = im.resize((width, round(im.height * width / im.width)), Image.LANCZOS)
    OUT.mkdir(parents=True, exist_ok=True)
    im.save(OUT / name, "WEBP", quality=88)
    print(OUT / name, im.size)


def main(src: Path):
    # 1. Dashboard: development key (masked), Copy, Register Product.
    im = Image.open(src / "1.png").convert("RGB")
    mask(im, (583, 464, 1016, 491), "RGAPI-xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx")
    mark(im, (1084, 462, 1142, 493), 1, label_side="right")
    mark(im, (1337, 108, 1587, 150), 2)
    save(im.crop((230, 0, 1809, 980)), "1-dashboard.webp")

    # 2. Product type: Personal API Key. Crop the browser bars, blur the account name.
    im = Image.open(src / "2.webp").convert("RGB")
    blur(im, (1860, 100, 2000, 150), 12)
    mark(im, (1022, 423, 1601, 676), pad=4, width=3)
    mark(im, (1200, 616, 1423, 653), 3)
    save(im.crop((380, 95, 1640, 720)), "2-personal.webp")

    # 3. Application form, numbered fields.
    im = Image.open(src / "3.png").convert("RGB")
    for n, box in enumerate(
        [
            (745, 272, 1349, 306),  # name
            (745, 541, 1349, 761),  # description
            (745, 852, 1349, 885),  # group
            (745, 929, 1349, 963),  # url
            (745, 1064, 1349, 1098),  # game focus
            (745, 1152, 1028, 1223),  # captcha
        ],
        start=1,
    ):
        mark(im, box, n)
    save(im.crop((660, 62, 1420, 1274)), "3-form.webp", width=900)

    # 4. Approved product: API key (masked) to copy.
    im = Image.open(src / "4.webp").convert("RGB")
    blur(im, (1800, 82, 2000, 120), 12)
    mask(im, (290, 948, 520, 968), "", fill=(245, 245, 245))
    mask(im, (1105, 774, 1500, 796), "RGAPI-xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx", size=15, fill=(255, 255, 255))
    mark(im, (1105, 774, 1500, 796), 1, pad=8)
    mark(im, (275, 188, 1940, 228), pad=3, width=3)
    save(im.crop((0, 78, 2000, 900)), "4-approved.webp")


if __name__ == "__main__":
    main(Path(sys.argv[1]))
