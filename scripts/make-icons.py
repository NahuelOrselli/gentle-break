#!/usr/bin/env python3
"""Rasterize the widget hourglass strokes into the app icon and the menu-bar glyph."""

import re
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ICONS = ROOT / "src-tauri" / "icons"
ICONSET = ICONS / "icon.iconset"

# Same centerlines as src/hourglass/strokes.ts. Static: no traveling light.
STROKES = [
    "M28 22 H212",
    "M28 334 H212",
    "M48 46 C44 108 84 146 108 168",
    "M192 46 C196 108 156 146 132 168",
    "M108 188 C84 214 44 256 48 310",
    "M132 188 C156 214 196 256 192 310",
    "M44 40 H196",
    "M44 316 H196",
    "M58 308 Q120 268 182 308",
    "M64 58 C66 104 96 140 112 162",
    "M176 58 C174 104 144 140 128 162",
    "M112 194 C96 222 66 258 64 300",
    "M128 194 C144 222 174 258 176 300",
    "M72 292 Q120 258 168 292",
    "M70 90 Q120 118 170 90",
    "M82 112 Q120 134 158 112",
    "M120 168 V256",
    "M114 178 V232",
    "M126 178 V232",
    "M108 166 V190",
    "M132 166 V190",
    "M28 22 V50",
    "M212 22 V50",
    "M28 306 V334",
    "M212 306 V334",
]


def chunk(tag: bytes, data: bytes) -> bytes:
    return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)


def write_png(path: Path, width: int, height: int, pixels: bytes) -> None:
    raw = b"".join(b"\x00" + pixels[y * width * 4 : (y + 1) * width * 4] for y in range(height))
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    data = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)


def flatten(d: str) -> list[tuple[float, float]]:
    tokens = re.findall(r"[A-Za-z]|-?\d+(?:\.\d+)?", d)
    index = 0
    command = ""
    x = y = 0.0
    points: list[tuple[float, float]] = []

    def take() -> float:
        nonlocal index
        value = float(tokens[index])
        index += 1
        return value

    while index < len(tokens):
        if tokens[index].isalpha():
            command = tokens[index]
            index += 1
        if command == "M":
            x, y = take(), take()
            points.append((x, y))
        elif command == "H":
            x = take()
            points.append((x, y))
        elif command == "V":
            y = take()
            points.append((x, y))
        elif command == "C":
            c1 = (take(), take())
            c2 = (take(), take())
            end = (take(), take())
            points.extend(cubic((x, y), c1, c2, end)[1:])
            x, y = end
        elif command == "Q":
            control = (take(), take())
            end = (take(), take())
            points.extend(quadratic((x, y), control, end)[1:])
            x, y = end
        else:
            raise SystemExit(f"unexpected path command {command}")
    return points


def cubic(p0, p1, p2, p3, steps: int = 18) -> list[tuple[float, float]]:
    out = []
    for step in range(steps + 1):
        t = step / steps
        u = 1 - t
        out.append(
            (
                u**3 * p0[0] + 3 * u**2 * t * p1[0] + 3 * u * t**2 * p2[0] + t**3 * p3[0],
                u**3 * p0[1] + 3 * u**2 * t * p1[1] + 3 * u * t**2 * p2[1] + t**3 * p3[1],
            )
        )
    return out


def quadratic(p0, p1, p2, steps: int = 14) -> list[tuple[float, float]]:
    out = []
    for step in range(steps + 1):
        t = step / steps
        u = 1 - t
        out.append(
            (
                u**2 * p0[0] + 2 * u * t * p1[0] + t**2 * p2[0],
                u**2 * p0[1] + 2 * u * t * p1[1] + t**2 * p2[1],
            )
        )
    return out


def blank(size: int) -> bytearray:
    return bytearray(size * size * 4)


def blend(pixels: bytearray, size: int, x: int, y: int, color: tuple[int, int, int, int], cover: float) -> None:
    if x < 0 or y < 0 or x >= size or y >= size or cover <= 0:
        return
    offset = (y * size + x) * 4
    src_a = color[3] / 255 * min(1.0, cover)
    dst_a = pixels[offset + 3] / 255
    out_a = src_a + dst_a * (1 - src_a)
    if out_a <= 0:
        return
    for channel in range(3):
        src = color[channel] / 255
        dst = pixels[offset + channel] / 255
        pixels[offset + channel] = int((src * src_a + dst * dst_a * (1 - src_a)) / out_a * 255)
    pixels[offset + 3] = int(out_a * 255)


def fill_round_rect(pixels: bytearray, size: int, radius: float, color: tuple[int, int, int, int]) -> None:
    inset = size * 0.04
    left, top, right, bottom = inset, inset, size - inset, size - inset
    for y in range(size):
        for x in range(size):
            px, py = x + 0.5, y + 0.5
            if not (left <= px <= right and top <= py <= bottom):
                continue
            cx = min(max(px, left + radius), right - radius)
            cy = min(max(py, top + radius), bottom - radius)
            if (px - cx) ** 2 + (py - cy) ** 2 <= radius**2:
                blend(pixels, size, x, y, color, 1)


def paint_polyline(
    pixels: bytearray,
    size: int,
    points: list[tuple[float, float]],
    radius: float,
    color: tuple[int, int, int, int],
    halo: float = 0,
) -> None:
    for start, end in zip(points, points[1:]):
        x0, y0 = start
        x1, y1 = end
        reach = radius + halo + 1
        min_x = max(0, int(min(x0, x1) - reach))
        max_x = min(size - 1, int(max(x0, x1) + reach))
        min_y = max(0, int(min(y0, y1) - reach))
        max_y = min(size - 1, int(max(y0, y1) + reach))
        dx, dy = x1 - x0, y1 - y0
        length_sq = dx * dx + dy * dy
        for y in range(min_y, max_y + 1):
            for x in range(min_x, max_x + 1):
                px, py = x + 0.5, y + 0.5
                if length_sq == 0:
                    distance = ((px - x0) ** 2 + (py - y0) ** 2) ** 0.5
                else:
                    t = max(0.0, min(1.0, ((px - x0) * dx + (py - y0) * dy) / length_sq))
                    distance = ((px - (x0 + t * dx)) ** 2 + (py - (y0 + t * dy)) ** 2) ** 0.5
                if distance <= radius:
                    blend(pixels, size, x, y, color, 1)
                elif halo > 0 and distance <= radius + halo:
                    blend(pixels, size, x, y, color, (radius + halo - distance) / halo * 0.4)


def map_points(points: list[tuple[float, float]], size: int) -> list[tuple[float, float]]:
    draw_h = size * 0.74
    draw_w = draw_h * (240 / 360)
    origin_x = (size - draw_w) / 2
    origin_y = (size - draw_h) / 2
    return [(origin_x + x / 240 * draw_w, origin_y + y / 360 * draw_h) for x, y in points]


def render_icon(size: int) -> bytes:
    pixels = blank(size)
    fill_round_rect(pixels, size, size * 0.22, (16, 10, 14, 255))
    radius = max(0.7, size * 0.0055)
    core = (255, 45, 122, 255)
    for path in STROKES:
        paint_polyline(pixels, size, map_points(flatten(path), size), radius, core, halo=max(1.0, size * 0.004))
    return bytes(pixels)


def render_tray(size: int) -> bytes:
    pixels = blank(size)
    radius = max(0.85, size * 0.028)
    ink = (0, 0, 0, 255)
    for path in STROKES:
        paint_polyline(pixels, size, map_points(flatten(path), size), radius, ink)
    return bytes(pixels)


def write_ico(path: Path, images: list[tuple[int, bytes]]) -> None:
    count = len(images)
    header = struct.pack("<HHH", 0, 1, count)
    directory = bytearray()
    blobs = bytearray()
    offset = 6 + 16 * count
    for size, data in images:
        dimension = 0 if size >= 256 else size
        directory += struct.pack("<BBBBHHII", dimension, dimension, 0, 0, 1, 32, len(data), offset)
        offset += len(data)
        blobs += data
    path.write_bytes(header + directory + blobs)


def main() -> None:
    ICONS.mkdir(parents=True, exist_ok=True)
    write_png(ICONS / "tray.png", 32, 32, render_tray(32))
    named = {
        "32x32.png": 32,
        "128x128.png": 128,
        "128x128@2x.png": 256,
        "icon.png": 512,
    }
    for name, size in named.items():
        write_png(ICONS / name, size, size, render_icon(size))
    write_ico(
        ICONS / "icon.ico",
        [(size, (ICONS / name).read_bytes()) for name, size in (("32x32.png", 32), ("128x128.png", 128), ("128x128@2x.png", 256))],
    )
    iconset = {
        "icon_16x16.png": 16,
        "icon_16x16@2x.png": 32,
        "icon_32x32.png": 32,
        "icon_32x32@2x.png": 64,
        "icon_128x128.png": 128,
        "icon_128x128@2x.png": 256,
        "icon_256x256.png": 256,
        "icon_256x256@2x.png": 512,
        "icon_512x512.png": 512,
        "icon_512x512@2x.png": 1024,
    }
    ICONSET.mkdir(parents=True, exist_ok=True)
    for name, size in iconset.items():
        write_png(ICONSET / name, size, size, render_icon(size))


if __name__ == "__main__":
    main()
