"""
Stitchflow Embroidery Engine Sidecar
Contract: Inspect, render, and export embroidery files without modifying original source files.
Supported formats: DST, PES, JEF, VP3, EXP, HUS, XXX, SEW, PCS, PEC.
"""

import json
import math
import os
import sys
from pathlib import Path

SUPPORTED_FORMATS = {"dst", "pes", "jef", "vp3", "exp", "hus", "xxx", "sew", "pcs", "pec"}

DEFAULT_PALETTE = [
    "#2563eb",  # Blue
    "#dc2626",  # Red
    "#16a34a",  # Green
    "#d97706",  # Amber
    "#9333ea",  # Purple
    "#0891b2",  # Cyan
    "#db2777",  # Pink
    "#475569",  # Slate
    "#ea580c",  # Orange
    "#65a30d",  # Lime
    "#4f46e5",  # Indigo
    "#0d9488",  # Teal
    "#b45309",  # Brown
    "#1e293b",  # Dark Charcoal
    "#f59e0b",  # Yellow
    "#e11d48",  # Rose
]


def _format_hex(color_val, fallback_idx=0):
    if color_val is None:
        return DEFAULT_PALETTE[fallback_idx % len(DEFAULT_PALETTE)]
    if isinstance(color_val, str):
        c = color_val.strip()
        if c.startswith("#"):
            return c
        return f"#{c}"
    if isinstance(color_val, int):
        r = (color_val >> 16) & 0xFF
        g = (color_val >> 8) & 0xFF
        b = color_val & 0xFF
        return f"#{r:02x}{g:02x}{b:02x}"
    return DEFAULT_PALETTE[fallback_idx % len(DEFAULT_PALETTE)]


def _hex_to_rgb(hex_str):
    hex_str = hex_str.lstrip("#")
    if len(hex_str) == 6:
        return (int(hex_str[0:2], 16), int(hex_str[2:4], 16), int(hex_str[4:6], 16))
    return (50, 50, 50)


def load_pattern(path: Path):
    ext = path.suffix.lower().lstrip(".")
    if ext not in SUPPORTED_FORMATS:
        raise ValueError(f"Unsupported embroidery format: .{ext}")
    if not path.is_file():
        raise FileNotFoundError(f"Embroidery file not found: {path}")

    try:
        import pyembroidery
    except ImportError:
        raise RuntimeError("pyembroidery sidecar dependency is not installed")

    pattern = pyembroidery.read(str(path))
    if pattern is None:
        raise ValueError(f"Unable to parse embroidery pattern from {path.name}")
    return pattern, ext


def inspect_pattern(path: Path) -> dict:
    pattern, ext = load_pattern(path)

    bounds = pattern.bounds()
    # bounds are in 0.1mm units: (min_x, min_y, max_x, max_y)
    min_x, min_y, max_x, max_y = (0, 0, 0, 0)
    if bounds:
        min_x, min_y, max_x, max_y = bounds

    width_mm = max(0.0, round((max_x - min_x) / 10.0, 2))
    height_mm = max(0.0, round((max_y - min_y) / 10.0, 2))

    total_stitches = len(pattern.stitches) if pattern.stitches else 0

    # Stitches flags check for jumps and trims
    jumps = 0
    trims = 0
    if pattern.stitches:
        for st in pattern.stitches:
            flag = st[2] if len(st) > 2 else 0
            if flag & 1:  # JUMP flag
                jumps += 1
            if flag & 2:  # TRIM flag
                trims += 1

    threads = []
    if pattern.threadlist:
        for idx, th in enumerate(pattern.threadlist):
            color_hex = None
            try:
                if hasattr(th, "hex_color") and callable(th.hex_color):
                    color_hex = th.hex_color()
                elif hasattr(th, "color"):
                    color_hex = _format_hex(th.color, idx)
            except Exception:
                color_hex = DEFAULT_PALETTE[idx % len(DEFAULT_PALETTE)]

            if not color_hex:
                color_hex = DEFAULT_PALETTE[idx % len(DEFAULT_PALETTE)]

            brand = getattr(th, "brand", "") or ""
            desc = getattr(th, "description", "") or getattr(th, "catalog_number", "") or f"Color {idx + 1}"
            threads.append({
                "index": idx + 1,
                "hex": color_hex if color_hex.startswith("#") else f"#{color_hex}",
                "brand": str(brand),
                "description": str(desc)
            })

    color_count = len(threads) if threads else 1

    return {
        "format": ext.upper(),
        "stitches": total_stitches,
        "colors": color_count,
        "width_mm": width_mm,
        "height_mm": height_mm,
        "bounds": [round(min_x, 1), round(min_y, 1), round(max_x, 1), round(max_y, 1)],
        "jumps": jumps,
        "trims": trims,
        "threads": threads,
        "filename": path.name,
    }


def render_preview(path: Path, out_png: Path, out_svg: Path = None, width: int = 800, height: int = 800) -> dict:
    pattern, ext = load_pattern(path)
    out_png.parent.mkdir(parents=True, exist_ok=True)

    try:
        from PIL import Image, ImageDraw
    except ImportError:
        raise RuntimeError("Pillow sidecar dependency is not installed")

    bounds = pattern.bounds()
    if not bounds or (bounds[2] - bounds[0] == 0 and bounds[3] - bounds[1] == 0):
        # Empty pattern, render blank canvas
        img = Image.new("RGBA", (width, height), (248, 250, 252, 255))
        img.save(str(out_png), "PNG")
        return {"status": "ok", "png": str(out_png)}

    min_x, min_y, max_x, max_y = bounds
    p_width = max_x - min_x
    p_height = max_y - min_y

    padding = 60
    avail_w = width - (padding * 2)
    avail_h = height - (padding * 2)

    scale = min(avail_w / max(p_width, 1), avail_h / max(p_height, 1))

    # Center pattern in canvas
    offset_x = padding + (avail_w - (p_width * scale)) / 2.0 - (min_x * scale)
    offset_y = padding + (avail_h - (p_height * scale)) / 2.0 - (min_y * scale)

    # 2x supersampling for crisp anti-aliasing
    ss = 2
    ss_w = width * ss
    ss_h = height * ss
    ss_scale = scale * ss
    ss_ox = offset_x * ss
    ss_oy = offset_y * ss

    img = Image.new("RGBA", (ss_w, ss_h), (250, 250, 252, 255))
    draw = ImageDraw.Draw(img)

    # Draw subtle background grid and hoop center crosshair
    center_cx = ss_w / 2.0
    center_cy = ss_h / 2.0
    grid_color = (230, 235, 243, 255)
    cross_color = (205, 215, 230, 255)

    # Grid marks
    step = int(50 * ss)
    for gx in range(0, ss_w, step):
        draw.line([(gx, 0), (gx, ss_h)], fill=grid_color, width=1)
    for gy in range(0, ss_h, step):
        draw.line([(0, gy), (ss_w, gy)], fill=grid_color, width=1)

    # Hoop center crosshair
    draw.line([(center_cx - 40 * ss, center_cy), (center_cx + 40 * ss, center_cy)], fill=cross_color, width=2)
    draw.line([(center_cx, center_cy - 40 * ss), (center_cx, center_cy + 40 * ss)], fill=cross_color, width=2)

    # Build color map for thread sequences
    threads = []
    if pattern.threadlist:
        for idx, th in enumerate(pattern.threadlist):
            try:
                if hasattr(th, "hex_color") and callable(th.hex_color):
                    h = th.hex_color()
                else:
                    h = _format_hex(getattr(th, "color", None), idx)
            except Exception:
                h = DEFAULT_PALETTE[idx % len(DEFAULT_PALETTE)]
            threads.append(_hex_to_rgb(h))

    if not threads:
        threads = [_hex_to_rgb(DEFAULT_PALETTE[0])]

    # Stitches: (x, y, flags)
    # flags: 0 = STITCH, 1 = JUMP, 2 = TRIM, 4 = STOP, 8 = END, 16 = COLOR_CHANGE
    current_color_idx = 0
    current_color = threads[0]
    line_color = (*current_color, 255)

    last_x, last_y = None, None
    stitch_width = max(2, int(2 * ss))

    if pattern.stitches:
        for stitch in pattern.stitches:
            sx, sy = stitch[0], stitch[1]
            flags = stitch[2] if len(stitch) > 2 else 0

            # Transform coords
            curr_px = sx * ss_scale + ss_ox
            curr_py = sy * ss_scale + ss_oy

            is_jump = bool(flags & 1)
            is_trim = bool(flags & 2)
            is_color_change = bool(flags & (16 | 4))

            if is_color_change:
                current_color_idx = (current_color_idx + 1) % len(threads)
                current_color = threads[current_color_idx]
                line_color = (*current_color, 255)
                last_x, last_y = None, None

            if is_jump or is_trim:
                last_x, last_y = None, None
                continue

            if last_x is not None and last_y is not None:
                # Draw stitch line
                draw.line([(last_x, last_y), (curr_px, curr_py)], fill=line_color, width=stitch_width)

            last_x, last_y = curr_px, curr_py

    # Downsample back to target width/height using LANCZOS for high fidelity
    img = img.resize((width, height), Image.Resampling.LANCZOS)
    img.save(str(out_png), "PNG", optimize=True)

    result = {"status": "ok", "png": str(out_png)}

    # Optional SVG export
    if out_svg:
        out_svg.parent.mkdir(parents=True, exist_ok=True)
        try:
            import pyembroidery
            pyembroidery.write_svg(pattern, str(out_svg))
            result["svg"] = str(out_svg)
        except Exception as svg_err:
            result["svg_error"] = str(svg_err)

    return result


def export_pattern(src_path: Path, dst_path: Path, target_format: str) -> dict:
    target_ext = target_format.lower().lstrip(".")
    if target_ext not in SUPPORTED_FORMATS:
        raise ValueError(f"Target format .{target_ext} is not supported")

    pattern, _ = load_pattern(src_path)
    dst_path.parent.mkdir(parents=True, exist_ok=True)

    import pyembroidery
    pyembroidery.write(pattern, str(dst_path))

    return {
        "status": "ok",
        "src": str(src_path),
        "dst": str(dst_path),
        "format": target_ext.upper(),
        "size_bytes": dst_path.stat().st_size if dst_path.exists() else 0,
    }


def main():
    if len(sys.argv) < 2:
        print(json.dumps({"error": "Usage: engine.py <inspect|render|export> [args...]"}), file=sys.stderr)
        sys.exit(1)

    cmd = sys.argv[1]

    try:
        if cmd == "inspect" or (cmd not in ("render", "export") and len(sys.argv) == 2):
            path_arg = sys.argv[2] if cmd == "inspect" else sys.argv[1]
            res = inspect_pattern(Path(path_arg))
            print(json.dumps(res))
        elif cmd == "render":
            if len(sys.argv) < 4:
                raise ValueError("Usage: engine.py render <src_file> <out_png> [out_svg]")
            src_p = Path(sys.argv[2])
            png_p = Path(sys.argv[3])
            svg_p = Path(sys.argv[4]) if len(sys.argv) > 4 else None
            res = render_preview(src_p, png_p, svg_p)
            print(json.dumps(res))
        elif cmd == "export":
            if len(sys.argv) < 5:
                raise ValueError("Usage: engine.py export <src_file> <dst_file> <target_format>")
            src_p = Path(sys.argv[2])
            dst_p = Path(sys.argv[3])
            fmt = sys.argv[4]
            res = export_pattern(src_p, dst_p, fmt)
            print(json.dumps(res))
        else:
            raise ValueError(f"Unknown command: {cmd}")
    except Exception as exc:
        print(json.dumps({"error": str(exc)}), file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()

