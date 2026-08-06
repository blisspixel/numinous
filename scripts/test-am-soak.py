#!/usr/bin/env python3
"""Focused regressions for what the soak accepts as output.

These cover the judgement without running the soak, which walks a dozen rooms
through render and sonify and takes minutes. The live walk is `am-soak.py`,
which CI runs against real binaries.

The judgement is the part worth testing on its own, because it is the part that
was wrong. The soak accepted a picture for existing and a room bed for being
over a thousand bytes. A room bed is uncompressed PCM, so its length is set by
how long it plays and not at all by whether it makes a sound: forty seconds of
silence weighs the same two and a half megabytes as forty seconds of music. The
gate would have reported a pass on a catalog that had gone completely quiet.
"""

from __future__ import annotations

import importlib.util
import struct
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location("am_soak", ROOT / "scripts" / "am-soak.py")
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

AUDIBLE = (
    "wrote a.wav (room bed, 40.00s, 79 events, stereo 16000 Hz, variation 0)\n"
    "Signal: peak 0.13463, RMS 0.04209, crest 10.10 dB, balance -0.04 dB"
)
SILENT = (
    "wrote a.wav (room bed, 40.00s, 0 events, stereo 16000 Hz, variation 0)\n"
    "Signal: peak 0.00000, RMS 0.00000, crest 0.00 dB, balance 0.00 dB"
)

# Forty seconds of stereo PCM. The size a silent bed and a playing one share.
FULL_SIZE_BED = 2_560_000


def png_header(width: int, height: int, signature: bytes | None = None) -> bytes:
    return (
        (MODULE.PNG_SIGNATURE if signature is None else signature)
        + struct.pack(">I", 13)
        + b"IHDR"
        + struct.pack(">II", width, height)
    )


class BedJudgementTests(unittest.TestCase):
    def setUp(self) -> None:
        self.work = Path(tempfile.mkdtemp(prefix="am-soak-contract-"))
        self.bed = self.work / "a.wav"
        self.bed.write_bytes(b"\0" * FULL_SIZE_BED)

    def test_a_playing_bed_is_accepted(self) -> None:
        # Without this the rest could be met by a judgement that rejects
        # everything, which would be just as useless in the other direction.
        self.assertIsNone(MODULE.bed_complaint(self.bed, AUDIBLE))

    def test_a_full_size_silent_bed_is_rejected(self) -> None:
        # The defect this exists for. The file on disk is the full two and a
        # half megabytes, so every size-based check passes it.
        self.assertGreater(self.bed.stat().st_size, 1000)
        complaint = MODULE.bed_complaint(self.bed, SILENT)
        self.assertIsNotNone(complaint)
        self.assertIn("silent", complaint)

    def test_a_report_with_no_signal_line_is_not_a_pass(self) -> None:
        # An export that stopped reporting its signal is a change worth
        # noticing, not a reason to wave the file through.
        complaint = MODULE.bed_complaint(self.bed, "wrote a.wav (room bed)")
        self.assertIsNotNone(complaint)
        self.assertIn("no signal line", complaint)

    def test_a_missing_bed_is_rejected(self) -> None:
        self.assertIsNotNone(MODULE.bed_complaint(self.work / "absent.wav", AUDIBLE))

    def test_the_floors_sit_well_below_a_real_bed(self) -> None:
        # The quietest bed in the soak measures peak 0.133 and RMS 0.031. The
        # floors separate sound from silence; they are not a mixing opinion, and
        # a floor near the real value would fail on ordinary variation.
        self.assertLess(MODULE.MIN_PEAK, 0.133 / 10)
        self.assertLess(MODULE.MIN_RMS, 0.031 / 10)
        self.assertGreater(MODULE.MIN_PEAK, 0)
        self.assertGreater(MODULE.MIN_RMS, 0)


class PictureJudgementTests(unittest.TestCase):
    def setUp(self) -> None:
        self.work = Path(tempfile.mkdtemp(prefix="am-soak-png-"))

    def write(self, name: str, payload: bytes) -> Path:
        path = self.work / name
        path.write_bytes(payload)
        return path

    def test_a_png_of_the_requested_size_is_accepted(self) -> None:
        path = self.write("good.png", png_header(120, 80) + b"the rest of the file")
        self.assertIsNone(MODULE.png_complaint(path, 120, 80))

    def test_a_png_of_the_wrong_size_is_rejected(self) -> None:
        # A render that quietly ignored the geometry it was given.
        path = self.write("small.png", png_header(64, 40) + b"the rest of the file")
        complaint = MODULE.png_complaint(path, 120, 80)
        self.assertIsNotNone(complaint)
        self.assertIn("64x40", complaint)

    def test_an_empty_or_truncated_file_is_rejected(self) -> None:
        for name, payload in (
            ("empty.png", b""),
            ("truncated.png", MODULE.PNG_SIGNATURE),
            ("half-header.png", MODULE.PNG_SIGNATURE + b"\0\0\0\r IHD"),
        ):
            with self.subTest(name=name):
                self.assertIsNotNone(MODULE.png_complaint(self.write(name, payload), 120, 80))

    def test_a_file_that_is_not_a_png_is_rejected(self) -> None:
        path = self.write("gif.png", b"GIF89a" + b"\0" * 40)
        complaint = MODULE.png_complaint(path, 120, 80)
        self.assertIsNotNone(complaint)
        self.assertIn("not a PNG", complaint)

    def test_a_png_that_does_not_open_with_ihdr_is_rejected(self) -> None:
        # The standard requires IHDR first. A file that opens otherwise is not
        # something to read dimensions out of and trust.
        payload = MODULE.PNG_SIGNATURE + struct.pack(">I", 13) + b"IDAT" + struct.pack(">II", 120, 80)
        complaint = MODULE.png_complaint(self.write("noihdr.png", payload), 120, 80)
        self.assertIsNotNone(complaint)
        self.assertIn("IHDR", complaint)


if __name__ == "__main__":
    unittest.main()
