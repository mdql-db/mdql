"""Unquoted date literals in SQL (issue #66).

`WHERE date >= 2026-01-01` lexed as the arithmetic expression 2026 - 1 - 1 and
compared 2024 against the column, so the query returned a plausible wrong row
set with no error. These lock the fix end-to-end through the Python binding.
"""

import pytest
from mdql import Database
from mdql.errors import MdqlError


DATES = [
    "2025-06-30",
    "2025-12-31",
    "2026-01-01",
    "2026-01-02",
    "2026-07-15",
]


def make_db(tmp_path):
    db_dir = tmp_path / "datedb"
    db_dir.mkdir()
    (db_dir / "_mdql.md").write_text("---\ntype: database\nname: datedb\n---\n")

    events = db_dir / "events"
    events.mkdir()
    (events / "_mdql.md").write_text(
        "---\ntype: schema\ntable: events\nprimary_key: path\n"
        "frontmatter:\n  title:\n    type: string\n  event_type:\n    type: string\n"
        "  date:\n    type: string\n  ts:\n    type: string\n---\n"
    )
    for i, d in enumerate(DATES):
        (events / f"e{i}.md").write_text(
            f"---\ntitle: E{i}\nevent_type: UNLOCK\ndate: {d}\n"
            f"ts: {d}T12:30:00\n---\n# E{i}\n"
        )
    return db_dir


def count(db, pred):
    rows, _ = db.query(
        f"SELECT date FROM events WHERE event_type = 'UNLOCK' AND {pred}"
    )
    return len(rows)


class TestUnquotedDateWhere:
    def test_unquoted_matches_quoted(self, tmp_path):
        # Exact #66 repro shape: the unquoted form used to collapse to 2024
        # and over-count.
        db = Database(make_db(tmp_path))
        assert count(db, "date >= '2026-01-01'") == 3
        assert count(db, "date >= 2026-01-01") == 3

    def test_unquoted_is_not_arithmetic(self, tmp_path):
        # `>= 2024` is what 2026-01-01 used to become; it must no longer agree.
        db = Database(make_db(tmp_path))
        assert count(db, "date >= 2024") != count(db, "date >= 2026-01-01")

    def test_split_is_exhaustive(self, tmp_path):
        db = Database(make_db(tmp_path))
        total = count(db, "date != ''")
        assert count(db, "date >= 2026-01-01") + count(db, "date < 2026-01-01") == total

    @pytest.mark.parametrize(
        "pred,expected",
        [
            ("date = 2026-01-01", 1),
            ("date > 2026-01-01", 2),
            ("date <= 2025-12-31", 2),
            ("date != 2026-01-01", 4),
            ("date IN (2025-06-30, 2026-07-15)", 2),
        ],
    )
    def test_operators(self, tmp_path, pred, expected):
        db = Database(make_db(tmp_path))
        assert count(db, pred) == expected

    def test_unquoted_timestamp(self, tmp_path):
        db = Database(make_db(tmp_path))
        assert count(db, "ts >= 2026-01-01T12:30:00") == 3
        assert count(db, "ts = 2026-01-01T12:30:00") == 1


class TestNonIsoDateIsLoud:
    @pytest.mark.parametrize("literal", ["2026-1-1", "2026-01-1", "2026-1-01"])
    def test_non_padded_raises(self, tmp_path, literal):
        # Non-padded dates sort wrong against ISO strings, so they error
        # rather than answer a different question.
        db = Database(make_db(tmp_path))
        with pytest.raises(MdqlError) as exc:
            db.query(f"SELECT date FROM events WHERE date >= {literal}")
        assert "2026-01-01" in str(exc.value)


class TestArithmeticStillWorks:
    def test_spaced_subtraction_unaffected(self, tmp_path):
        db = Database(make_db(tmp_path))
        rows, _ = db.query("SELECT 2026 - 1 - 1 AS n FROM events LIMIT 1")
        assert rows[0]["n"] == 2024
