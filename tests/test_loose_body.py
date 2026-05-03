"""Tests for loose body warning when files have content but no H2 sections."""

from mdql.api import Table


SCHEMA = """\
---
type: schema
table: notes
primary_key: path
frontmatter:
  title:
    type: string
h1:
  required: false
sections: {}
rules:
  reject_unknown_frontmatter: false
  reject_unknown_sections: false
  reject_duplicate_sections: true
---
"""


def _make_table(tmp_path):
    (tmp_path / "_mdql.md").write_text(SCHEMA)
    return Table(str(tmp_path))


class TestLooseBodyWarning:
    def test_no_warning_with_h2_sections(self, tmp_path):
        t = _make_table(tmp_path)
        (tmp_path / "note.md").write_text(
            "---\ntitle: Note\n---\n# Note\n\n## Details\n\nSome details.\n"
        )
        rows, errors = t.load()
        assert len(rows) == 1
        assert len(errors) == 0

    def test_warning_with_body_no_h2(self, tmp_path):
        t = _make_table(tmp_path)
        (tmp_path / "note.md").write_text(
            "---\ntitle: Note\n---\n# Note\n\nThis is loose body content.\n"
        )
        rows, errors = t.load()
        assert len(rows) == 1
        assert len(errors) == 1
        assert "not queryable" in errors[0]

    def test_no_warning_empty_body(self, tmp_path):
        t = _make_table(tmp_path)
        (tmp_path / "note.md").write_text(
            "---\ntitle: Note\n---\n# Note\n"
        )
        rows, errors = t.load()
        assert len(rows) == 1
        assert len(errors) == 0

    def test_row_still_loaded_with_warning(self, tmp_path):
        t = _make_table(tmp_path)
        (tmp_path / "a.md").write_text(
            "---\ntitle: A\n---\n\nLoose content here.\n"
        )
        (tmp_path / "b.md").write_text(
            "---\ntitle: B\n---\n\n## Section\n\nStructured.\n"
        )
        rows, errors = t.load()
        assert len(rows) == 2
        assert len(errors) == 1
        titles = {r["title"] for r in rows}
        assert titles == {"A", "B"}

    def test_no_warning_when_h2_present_with_loose_content(self, tmp_path):
        t = _make_table(tmp_path)
        (tmp_path / "note.md").write_text(
            "---\ntitle: Note\n---\n\nSome preamble.\n\n## Section\n\nBody.\n"
        )
        rows, errors = t.load()
        assert len(rows) == 1
        loose = [e for e in errors if "not queryable" in e]
        assert len(loose) == 0

    def test_validate_reports_loose_body(self, tmp_path):
        t = _make_table(tmp_path)
        (tmp_path / "note.md").write_text(
            "---\ntitle: Note\n---\n\nLoose content.\n"
        )
        errors = t.validate()
        assert any("not queryable" in e for e in errors)

    def test_no_warning_whitespace_only_body(self, tmp_path):
        t = _make_table(tmp_path)
        (tmp_path / "note.md").write_text(
            "---\ntitle: Note\n---\n\n\n\n"
        )
        rows, errors = t.load()
        assert len(rows) == 1
        assert len(errors) == 0
