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
from typing import Any, Dict, List, Optional


SUPPORTED_FORMATS = {"dst", "pes", "jef", "vp3", "exp", "hus", "xxx", "sew", "pcs", "pec"}
WRITABLE_FORMATS = {"pes", "dst", "jef", "exp", "vp3", "xxx", "pec", "u01", "tbf"}


def safe_write_pattern(pattern, dst_path: Path) -> Path:
    import pyembroidery
    dst_path = Path(dst_path).resolve()
    dst_path.parent.mkdir(parents=True, exist_ok=True)
    ext = dst_path.suffix.lower().lstrip(".")

    if ext not in WRITABLE_FORMATS:
        dst_path = dst_path.with_suffix(".pes")

    try:
        pyembroidery.write(pattern, str(dst_path))
        return dst_path
    except Exception:
        fallback_path = dst_path.with_suffix(".dst")
        pyembroidery.write(pattern, str(fallback_path))
        return fallback_path


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
    pattern, _ = load_pattern(src_path)
    actual_dst = safe_write_pattern(pattern, dst_path)

    return {
        "status": "ok",
        "src": str(src_path),
        "dst": str(actual_dst),
        "format": actual_dst.suffix.lstrip(".").upper(),
        "size_bytes": actual_dst.stat().st_size if actual_dst.exists() else 0,
    }



def digitize_image(
    image_path: Path,
    dst_path: Path,
    target_fmt: str = "pes",
    width_mm: float = 50.0,
    height_mm: float = 50.0,
    out_preview_png: Path | None = None,
) -> dict:
    target_ext = target_fmt.lower().lstrip(".")
    if target_ext not in SUPPORTED_FORMATS:
        raise ValueError(f"Target format .{target_ext} is not supported")

    from PIL import Image, ImageOps
    import pyembroidery

    img = Image.open(image_path).convert("RGBA")

    # 1. Resize to target dimension grid (10 px per mm resolution)
    w_px = max(60, min(1200, int(width_mm * 10)))
    h_px = max(60, min(1200, int(height_mm * 10)))
    img = img.resize((w_px, h_px), Image.Resampling.LANCZOS)

    pixels = img.load()
    mask = [[False for _ in range(w_px)] for _ in range(h_px)]
    colors_detected = {}

    for y in range(h_px):
        for x in range(w_px):
            r, g, b, a = pixels[x, y]
            # Detect foreground shapes (ignore white / transparent backgrounds)
            if a > 80 and not (r > 238 and g > 238 and b > 238):
                mask[y][x] = True
                qr = (r // 32) * 32
                qg = (g // 32) * 32
                qb = (b // 32) * 32
                key = f"#{qr:02x}{qg:02x}{qb:02x}"
                colors_detected[key] = colors_detected.get(key, 0) + 1

    pattern = pyembroidery.EmbPattern()

    # Determine dominant thread color
    hex_color = list(colors_detected.keys())[0] if colors_detected else "#1e293b"
    r_val = int(hex_color[1:3], 16)
    g_val = int(hex_color[3:5], 16)
    b_val = int(hex_color[5:7], 16)
    pattern.add_thread({"hex": hex_color, "description": "AI Digitized Thread", "brand": "Standard"})

    cx = w_px / 2.0
    cy = h_px / 2.0
    scale_x = (width_mm * 10.0) / w_px
    scale_y = (height_mm * 10.0) / h_px

    # Tatami fill parameters (0.4mm line spacing, 2.5mm stitch length)
    row_step = 4
    stitch_pitch = 25

    last_x, last_y = None, None

    for row_idx, y in enumerate(range(0, h_px, row_step)):
        spans = []
        in_span = False
        start_x = 0
        for x in range(w_px):
            if mask[y][x]:
                if not in_span:
                    in_span = True
                    start_x = x
            else:
                if in_span:
                    in_span = False
                    spans.append((start_x, x - 1))
        if in_span:
            spans.append((start_x, w_px - 1))

        if not spans:
            continue

        if row_idx % 2 == 1:
            spans.reverse()

        for span_start, span_end in spans:
            if row_idx % 2 == 0:
                cur_x = span_start
                ex = (cur_x - cx) * scale_x
                ey = (y - cy) * scale_y
                if last_x is None or math.hypot(ex - last_x, ey - last_y) > 40:
                    pattern.add_stitch_relative(pyembroidery.TRIM, 0, 0)
                    pattern.add_stitch_absolute(pyembroidery.JUMP, ex, ey)
                pattern.add_stitch_absolute(pyembroidery.STITCH, ex, ey)

                cur_x += int(stitch_pitch / scale_x)
                while cur_x < span_end:
                    ex = (cur_x - cx) * scale_x
                    pattern.add_stitch_absolute(pyembroidery.STITCH, ex, ey)
                    cur_x += int(stitch_pitch / scale_x)

                ex = (span_end - cx) * scale_x
                pattern.add_stitch_absolute(pyembroidery.STITCH, ex, ey)
                last_x, last_y = ex, ey
            else:
                cur_x = span_end
                ex = (cur_x - cx) * scale_x
                ey = (y - cy) * scale_y
                if last_x is None or math.hypot(ex - last_x, ey - last_y) > 40:
                    pattern.add_stitch_relative(pyembroidery.TRIM, 0, 0)
                    pattern.add_stitch_absolute(pyembroidery.JUMP, ex, ey)
                pattern.add_stitch_absolute(pyembroidery.STITCH, ex, ey)

                cur_x -= int(stitch_pitch / scale_x)
                while cur_x > span_start:
                    ex = (cur_x - cx) * scale_x
                    pattern.add_stitch_absolute(pyembroidery.STITCH, ex, ey)
                    cur_x -= int(stitch_pitch / scale_x)

                ex = (span_start - cx) * scale_x
                pattern.add_stitch_absolute(pyembroidery.STITCH, ex, ey)
                last_x, last_y = ex, ey

    pattern.add_stitch_relative(pyembroidery.END, 0, 0)
    actual_dst = safe_write_pattern(pattern, dst_path)

    if out_preview_png:
        out_preview_png.parent.mkdir(parents=True, exist_ok=True)
        render_preview(actual_dst, out_preview_png)

    return inspect_pattern(actual_dst)


def edit_pattern(src_path: Path, dst_path: Path, operations_json_str: str, out_preview_png: Optional[Path] = None) -> Dict[str, Any]:
    import pyembroidery
    src_path = Path(src_path).resolve()
    dst_path = Path(dst_path).resolve()
    if out_preview_png:
        out_preview_png = Path(out_preview_png).resolve()

    if not src_path.exists():
        raise FileNotFoundError(f"Source embroidery file not found: {src_path}")

    pattern = pyembroidery.read(str(src_path))
    if not pattern:
        raise ValueError("Failed to parse source embroidery file")


    if operations_json_str.endswith(".json") and Path(operations_json_str).exists():
        with open(operations_json_str, "r", encoding="utf-8") as f:
            ops = json.load(f)
    else:
        try:
            ops = json.loads(operations_json_str)
        except Exception:
            try:
                import ast
                ops = ast.literal_eval(operations_json_str)
            except Exception as e:
                raise ValueError(f"Invalid operations JSON: {operations_json_str} ({e})")

    if not isinstance(ops, list):
        ops = [ops]

    COLOR_HEX_MAP = {
        "red": (220, 38, 38),
        "blue": (37, 99, 235),
        "royal blue": (29, 78, 216),
        "green": (22, 163, 74),
        "gold": (234, 179, 8),
        "yellow": (250, 204, 21),
        "purple": (147, 51, 234),
        "black": (0, 0, 0),
        "white": (255, 255, 255),
        "pink": (236, 72, 153),
        "orange": (234, 88, 12),
        "teal": (13, 148, 136),
    }

    def hex_to_rgb(hex_str):
        h = str(hex_str).lstrip("#")
        if len(h) == 6:
            try:
                return tuple(int(h[i : i + 2], 16) for i in (0, 2, 4))
            except Exception:
                return (128, 128, 128)
        return (128, 128, 128)

    for op in ops:
        op_type = str(op.get("op", "")).lower()

        # Calculate bounding box center (cx, cy) before each geometric operation
        bounds = pattern.bounds()
        cx = (bounds[0] + bounds[2]) / 2.0
        cy = (bounds[1] + bounds[3]) / 2.0

        if op_type in ("recolor_stop", "change_color", "recolor"):
            idx = op.get("stop_index", op.get("color_index"))
            target_source_color = op.get("from_color") or op.get("from_hex")

            # If target source color specified (e.g. "change red to blue"), find closest thread
            if target_source_color and pattern.threadlist:
                src_rgb = COLOR_HEX_MAP.get(str(target_source_color).lower()) or hex_to_rgb(target_source_color)
                best_idx = 0
                best_dist = float("inf")
                for i, th in enumerate(pattern.threadlist):
                    th_rgb = hex_to_rgb(th.hex_color())
                    d = (th_rgb[0] - src_rgb[0]) ** 2 + (th_rgb[1] - src_rgb[1]) ** 2 + (th_rgb[2] - src_rgb[2]) ** 2
                    if d < best_dist:
                        best_dist = d
                        best_idx = i
                idx = best_idx
            elif idx is None:
                idx = 0
            else:
                idx = int(idx)

            to_hex = str(op.get("to_hex", op.get("to", "#000000")))
            desc = op.get("description", op.get("name", ""))
            if 0 <= idx < len(pattern.threadlist):
                pattern.threadlist[idx].set_hex_color(to_hex)
                if desc:
                    pattern.threadlist[idx].description = str(desc)

        elif op_type == "scale":
            factor = float(op.get("factor", 1.0))
            if 0.05 < factor < 10.0:
                # Exact algebraic center-preserving scale:
                tx = cx * (1.0 - factor)
                ty = cy * (1.0 - factor)
                m = pyembroidery.EmbMatrix([factor, 0.0, 0.0, 0.0, factor, 0.0, tx, ty, 1.0])
                pattern.transform(m)

        elif op_type == "rotate":
            angle_deg = float(op.get("angle_deg", op.get("angle", 0.0)))
            theta_rad = math.radians(angle_deg)
            ct = math.cos(theta_rad)
            st = math.sin(theta_rad)
            tx = cx * (1.0 - ct) + cy * st
            ty = cy * (1.0 - ct) - cx * st
            m = pyembroidery.EmbMatrix([ct, st, 0.0, -st, ct, 0.0, tx, ty, 1.0])
            pattern.transform(m)

        elif op_type == "flip":
            axis = str(op.get("axis", "horizontal")).lower()
            if axis in ("horizontal", "x"):
                m = pyembroidery.EmbMatrix([-1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 2.0 * cx, 0.0, 1.0])
            else:
                m = pyembroidery.EmbMatrix([1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 2.0 * cy, 1.0])
            pattern.transform(m)

        elif op_type == "fit_hoop":
            hoop_size_mm = float(op.get("max_dimension_mm", op.get("hoop_size_mm", 60.0)))
            usable_margin_factor = float(op.get("margin_factor", 0.90))
            usable_field_mm = hoop_size_mm * usable_margin_factor

            w_mm = (bounds[2] - bounds[0]) / 10.0
            h_mm = (bounds[3] - bounds[1]) / 10.0
            current_max = max(w_mm, h_mm)

            if current_max > 0 and current_max > usable_field_mm:
                factor = usable_field_mm / current_max
                tx = cx * (1.0 - factor)
                ty = cy * (1.0 - factor)
                m = pyembroidery.EmbMatrix([factor, 0.0, 0.0, 0.0, factor, 0.0, tx, ty, 1.0])
                pattern.transform(m)

    actual_dst = safe_write_pattern(pattern, dst_path)

    if out_preview_png:
        out_preview_png.parent.mkdir(parents=True, exist_ok=True)
        render_preview(actual_dst, out_preview_png)

    return inspect_pattern(actual_dst)




def main():
    if len(sys.argv) < 2:
        print(json.dumps({"error": "Usage: engine.py <inspect|render|export|digitize|edit> [args...]"}), file=sys.stderr)
        sys.exit(1)

    cmd = sys.argv[1]

    try:
        if cmd == "inspect" or (cmd not in ("render", "export", "digitize", "edit") and len(sys.argv) == 2):
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
        elif cmd == "digitize":
            if len(sys.argv) < 6:
                raise ValueError("Usage: engine.py digitize <src_image> <dst_emb_file> <format> <width_mm> <height_mm> [out_preview_png]")
            src_img = Path(sys.argv[2])
            dst_emb = Path(sys.argv[3])
            fmt = sys.argv[4]
            w_mm = float(sys.argv[5])
            h_mm = float(sys.argv[6]) if len(sys.argv) > 6 else w_mm
            prev_p = Path(sys.argv[7]) if len(sys.argv) > 7 else None
            res = digitize_image(src_img, dst_emb, fmt, w_mm, h_mm, prev_p)
            print(json.dumps(res))
        elif cmd == "edit":
            if len(sys.argv) < 5:
                raise ValueError("Usage: engine.py edit <src_emb_file> <dst_emb_file> <operations_json> [out_preview_png]")
            src_emb = Path(sys.argv[2])
            dst_emb = Path(sys.argv[3])
            ops_json = sys.argv[4]
            prev_p = Path(sys.argv[5]) if len(sys.argv) > 5 else None
            res = edit_pattern(src_emb, dst_emb, ops_json, prev_p)
            print(json.dumps(res))
        else:
            raise ValueError(f"Unknown command: {cmd}")
    except Exception as exc:
        print(json.dumps({"error": str(exc)}), file=sys.stderr)
        sys.exit(1)



if __name__ == "__main__":
    main()


