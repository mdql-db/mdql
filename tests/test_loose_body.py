"""Tests for loose body rejection — content not under H2 is a hard error."""

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


class TestLooseBodyRejection:
    def test_no_error_with_h2_sections(self, tmp_path):
        t = _make_table(tmp_path)
        (tmp_path / "note.md").write_text(
            "---\ntitle: Note\n---\n# Note\n\n## Details\n\nSome details.\n"
        )
        rows, errors = t.load()
        assert len(rows) == 1
        assert len(errors) == 0

    def test_error_with_body_no_h2(self, tmp_path):
        t = _make_table(tmp_path)
        (tmp_path / "note.md").write_text(
            "---\ntitle: Note\n---\n# Note\n\nThis is loose body content.\n"
        )
        rows, errors = t.load()
        assert len(rows) == 0
        assert len(errors) == 1
        assert "not allowed" in errors[0]

    def test_no_error_empty_body(self, tmp_path):
        t = _make_table(tmp_path)
        (tmp_path / "note.md").write_text(
            "---\ntitle: Note\n---\n# Note\n"
        )
        rows, errors = t.load()
        assert len(rows) == 1
        assert len(errors) == 0

    def test_error_with_loose_content_before_h2(self, tmp_path):
        t = _make_table(tmp_path)
        (tmp_path / "note.md").write_text(
            "---\ntitle: Note\n---\n\nSome preamble.\n\n## Section\n\nBody.\n"
        )
        rows, errors = t.load()
        assert len(rows) == 0
        assert any("not allowed" in e for e in errors)

    def test_validate_reports_loose_body(self, tmp_path):
        t = _make_table(tmp_path)
        (tmp_path / "note.md").write_text(
            "---\ntitle: Note\n---\n\nLoose content.\n"
        )
        errors = t.validate()
        assert any("not allowed" in e for e in errors)

    def test_no_error_whitespace_only_body(self, tmp_path):
        t = _make_table(tmp_path)
        (tmp_path / "note.md").write_text(
            "---\ntitle: Note\n---\n\n\n\n"
        )
        rows, errors = t.load()
        assert len(rows) == 1
        assert len(errors) == 0

    def test_h1_alone_is_not_loose_body(self, tmp_path):
        t = _make_table(tmp_path)
        (tmp_path / "note.md").write_text(
            "---\ntitle: Note\n---\n\n# Note\n"
        )
        rows, errors = t.load()
        assert len(rows) == 1
        assert len(errors) == 0
