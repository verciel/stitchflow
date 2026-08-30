"""
Unit tests for the Stitchflow Embroidery Engine Sidecar (engine.py).
Tests inspect, render_preview, export, and error boundaries.
"""

import json
import os
import subprocess
import sys
import unittest
from pathlib import Path

# Add sidecar directory to sys.path
PROJECT_ROOT = Path(__file__).resolve().parent.parent
SIDECAR_SCRIPT = PROJECT_ROOT / "src-tauri" / "embroidery-engine" / "engine.py"
PYTHON_BIN = sys.executable

try:
    import pyembroidery
except ImportError:
    pyembroidery = None


class TestEmbroideryEngine(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.test_dir = PROJECT_ROOT / "target" / "test_scratch"
        cls.test_dir.mkdir(parents=True, exist_ok=True)

        # Create a sample DST pattern
        cls.dst_sample = cls.test_dir / "sample.dst"
        pattern = pyembroidery.EmbPattern()
        pattern.add_thread({"rgb": 0xFF0000, "description": "Classic Red"})
        pattern.add_thread({"rgb": 0x0000FF, "description": "Royal Blue"})

        # Red section
        pattern.add_stitch_absolute(0, 0, 0)
        pattern.add_stitch_absolute(0, 50, 50)
        pattern.add_stitch_absolute(0, 100, 0)

        # Color change
        pattern.add_command(16)  # COLOR_CHANGE
        pattern.add_stitch_absolute(0, 100, 100)
        pattern.add_stitch_absolute(0, 50, 150)
        pattern.add_stitch_absolute(0, 0, 100)

        pyembroidery.write_dst(pattern, str(cls.dst_sample))

    def run_engine(self, *args):
        cmd = [PYTHON_BIN, str(SIDECAR_SCRIPT)] + list(args)
        proc = subprocess.run(cmd, capture_output=True, text=True)
        return proc.returncode, proc.stdout, proc.stderr

    def test_inspect_sample(self):
        code, stdout, stderr = self.run_engine("inspect", str(self.dst_sample))
        self.assertEqual(code, 0, f"Inspect failed: {stderr}")

        data = json.loads(stdout)
        self.assertEqual(data["format"], "DST")
        self.assertGreater(data["stitches"], 0)
        self.assertIn("width_mm", data)
        self.assertIn("height_mm", data)
        self.assertIn("bounds", data)

    def test_render_preview(self):
        out_png = self.test_dir / "sample_preview.png"
        out_svg = self.test_dir / "sample_preview.svg"
        code, stdout, stderr = self.run_engine("render", str(self.dst_sample), str(out_png), str(out_svg))
        self.assertEqual(code, 0, f"Render failed: {stderr}")

        data = json.loads(stdout)
        self.assertEqual(data["status"], "ok")
        self.assertTrue(out_png.exists())
        self.assertGreater(out_png.stat().st_size, 1000)
        self.assertTrue(out_svg.exists())

    def test_export_formats(self):
        target_pes = self.test_dir / "sample_exported.pes"
        code, stdout, stderr = self.run_engine("export", str(self.dst_sample), str(target_pes), "pes")
        self.assertEqual(code, 0, f"Export to PES failed: {stderr}")

        data = json.loads(stdout)
        self.assertEqual(data["status"], "ok")
        self.assertEqual(data["format"], "PES")
        self.assertTrue(target_pes.exists())
        self.assertGreater(target_pes.stat().st_size, 100)

        # Inspect exported file to ensure valid embroidery header
        code, stdout, _ = self.run_engine("inspect", str(target_pes))
        self.assertEqual(code, 0)
        pes_data = json.loads(stdout)
        self.assertEqual(pes_data["format"], "PES")

    def test_unsupported_format_rejection(self):
        dummy_file = self.test_dir / "unsupported.txt"
        dummy_file.write_text("not an embroidery file")
        code, stdout, stderr = self.run_engine("inspect", str(dummy_file))
        self.assertNotEqual(code, 0)
        self.assertIn("Unsupported embroidery format", stderr)

    def test_missing_file_error(self):
        missing = self.test_dir / "non_existent.pes"
        code, stdout, stderr = self.run_engine("inspect", str(missing))
        self.assertNotEqual(code, 0)
        self.assertIn("not found", stderr)

    def test_safe_writer_fallback(self):
        # HUS is read-only in pyembroidery; export should transparently fall back to PES/DST
        target_hus = self.test_dir / "sample_exported.hus"
        code, stdout, stderr = self.run_engine("export", str(self.dst_sample), str(target_hus), "hus")
        self.assertEqual(code, 0, f"Export fallback failed: {stderr}")
        data = json.loads(stdout)
        self.assertEqual(data["status"], "ok")
        self.assertIn(data["format"], ["PES", "DST"])


if __name__ == "__main__":
    unittest.main()

