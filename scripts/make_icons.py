#!/usr/bin/env python3
"""生成 Gemini Draw 的应用图标（纯标准库，无需 Pillow）。

输出：
  src-tauri/icons/{32x32.png,128x128.png,128x128@2x.png,icon.png,icon.ico,icon.icns}
"""

import math
import os
import struct
import zlib

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src-tauri", "icons")

BG_A = (0x2B, 0x6C, 0xFF)  # 蓝
BG_B = (0xA8, 0x55, 0xF7)  # 紫
FG = (255, 255, 255, 255)


def lerp(a, b, t):
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(3))


def rounded_alpha(x, y, size, radius):
    """返回圆角矩形覆盖比例（0~1），用于抗锯齿边缘。"""
    r = radius
    if x < r and y < r:
        return 1.0 if (x - r) ** 2 + (y - r) ** 2 <= r * r else 0.0
    if x < r and y > size - r:
        return 1.0 if (x - r) ** 2 + (y - (size - r)) ** 2 <= r * r else 0.0
    if x > size - r and y < r:
        return 1.0 if (x - (size - r)) ** 2 + (y - r) ** 2 <= r * r else 0.0
    if x > size - r and y > size - r:
        return 1.0 if (x - (size - r)) ** 2 + (y - (size - r)) ** 2 <= r * r else 0.0
    return 1.0


def star_alpha(nx, ny, radius, power=0.55):
    """四角星：|x|^p + |y|^p <= r^p"""
    d = abs(nx) ** power + abs(ny) ** power
    limit = radius ** power
    return 1.0 if d <= limit else 0.0


def render(size):
    ss = 3  # 超采样
    px = [[0.0, 0.0, 0.0, 0.0] for _ in range(size * size)]
    total = float(ss * ss)
    for y in range(size):
        for x in range(size):
            acc = [0.0, 0.0, 0.0, 0.0]
            for sy in range(ss):
                for sx in range(ss):
                    fx = x + (sx + 0.5) / ss
                    fy = y + (sy + 0.5) / ss
                    cover = rounded_alpha(fx, fy, size, size * 0.22)
                    if cover <= 0:
                        continue
                    t = (fx / size + fy / size) / 2.0
                    r, g, b = lerp(BG_A, BG_B, max(0.0, min(1.0, t)))
                    a = 255.0
                    # 中心大星
                    nx = (fx - size * 0.44) / (size * 0.5)
                    ny = (fy - size * 0.44) / (size * 0.5)
                    s = star_alpha(nx, ny, 0.78)
                    if s > 0:
                        r, g, b = FG[0], FG[1], FG[2]
                    # 右下小星
                    nx2 = (fx - size * 0.76) / (size * 0.5)
                    ny2 = (fy - size * 0.76) / (size * 0.5)
                    s2 = star_alpha(nx2, ny2, 0.30)
                    if s2 > 0:
                        r, g, b = FG[0], FG[1], FG[2]
                    acc[0] += r
                    acc[1] += g
                    acc[2] += b
                    acc[3] += a
            px[y * size + x] = [c / total for c in acc]
    return px


def to_png(px, size):
    raw = bytearray()
    for y in range(size):
        raw.append(0)  # filter: none
        for x in range(size):
            r, g, b, a = px[y * size + x]
            raw += bytes((int(round(r)), int(round(g)), int(round(b)), int(round(a))))
    return bytes(raw)


def write_png(path, px, size):
    def chunk(tag, data):
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr)
    png += chunk(b"IDAT", zlib.compress(to_png(px, size), 9))
    png += chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(png)
    return png


def write_ico(path, pngs):
    """ICO 容器，直接内嵌 PNG（Vista+ 支持）"""
    header = struct.pack("<HHH", 0, 1, len(pngs))
    offset = 6 + 16 * len(pngs)
    entries = b""
    datas = b""
    for size, png in pngs:
        entries += struct.pack(
            "<BBBBHHII",
            0 if size >= 256 else size,
            0 if size >= 256 else size,
            0,
            0,
            1,
            32,
            len(png),
            offset,
        )
        datas += png
        offset += len(png)
    with open(path, "wb") as f:
        f.write(header + entries + datas)


def write_icns(path, items):
    """ICNS 容器，内嵌 PNG 条目（ic07/ic08/ic09）"""
    body = b""
    for tag, png in items:
        body += tag + struct.pack(">I", len(png) + 8) + png
    with open(path, "wb") as f:
        f.write(b"icns" + struct.pack(">I", len(body) + 8) + body)


def main():
    os.makedirs(OUT, exist_ok=True)
    cache = {}
    for size in (32, 128, 256, 512):
        cache[size] = write_png(os.path.join(OUT, f"{size}.png"), render(size), size)

    # Tauri 需要的命名
    write_png(os.path.join(OUT, "32x32.png"), render(32), 32)
    write_png(os.path.join(OUT, "128x128.png"), render(128), 128)
    write_png(os.path.join(OUT, "128x128@2x.png"), render(256), 256)
    icon_png = write_png(os.path.join(OUT, "icon.png"), render(512), 512)
    write_ico(os.path.join(OUT, "icon.ico"), [(256, cache[256]), (128, cache[128]), (32, cache[32])])
    write_icns(
        os.path.join(OUT, "icon.icns"),
        [(b"ic09", icon_png), (b"ic08", cache[256]), (b"ic07", cache[128])],
    )
    print("icons written to", os.path.abspath(OUT))


if __name__ == "__main__":
    main()
