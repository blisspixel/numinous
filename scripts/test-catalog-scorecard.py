#!/usr/bin/env python3
"""The catalog scorecard reads `numinous rooms --json`, not the human listing."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "catalog_scorecard", ROOT / "scripts" / "catalog-scorecard.py"
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class RoomsJsonTests(unittest.TestCase):
    def test_rows_come_from_id_and_wing(self) -> None:
        rows = MODULE.rows_from_rooms_json(
            '[{"id": "times-tables", "wing": "Number & Pattern", "title": "Times Tables"},'
            '{"id": "lissajous", "wing": "Waves & Sound", "title": "Lissajous"}]'
        )
        self.assertEqual(
            rows,
            [
                ("times-tables", "Number & Pattern"),
                ("lissajous", "Waves & Sound"),
            ],
        )

    def test_the_human_listing_is_not_a_catalog(self) -> None:
        listing = (
            "356 rooms in 13 wings\n\n"
            "times-tables  Number & Pattern  Times Tables\n\n"
            "numinous describe <id> for the story, "
            "numinous watch <id> to step inside.\n"
        )
        with self.assertRaises(RuntimeError):
            MODULE.rows_from_rooms_json(listing)

    def test_a_row_without_a_wing_is_rejected(self) -> None:
        with self.assertRaises(RuntimeError):
            MODULE.rows_from_rooms_json('[{"id": "times-tables"}]')


if __name__ == "__main__":
    unittest.main()
