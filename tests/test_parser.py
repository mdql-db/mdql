"""Tests for mdql.parser."""

from pathlib import Path

import pytest

from mdql.parser import ParsedFile, parse_file, normalize_heading

FIXTURES = Path(__file__).parent / "fixtures"


class TestNormalizeHeading:
    def test_strips_numbered_prefix(self):
        assert normalize_heading("1. Hypothesis") == "Hypothesis"

    def test_strips_multi_digit(self):
        assert normalize_heading("12. Backtest Methodology") == "Backtest Methodology"

    def test_no_prefix(self):
        assert normalize_heading("Hypothesis") == "Hypothesis"

    def test_no_space_after_dot_not_stripped(self):
        assert normalize_heading("1.Hypothesis") == "1.Hypothesis"


class TestFrontmatter:
    def test_basic_frontmatter(self):
        p = parse_file(FIXTURES / "valid_table" / "simple.md", relative_to=FIXTURES / "valid_table")
        assert p.raw_frontmatter["title"] == "Simple note"
        assert p.raw_frontmatter["author"] == "Rasmus"
        assert p.raw_frontmatter["status"] == "draft"
        assert p.path == "simple.md"
        assert not p.parse_errors

    def test_quoted_date_is_string(self):
        p = parse_file(FIXTURES / "valid_table" / "with-tags.md", relative_to=FIXTURES / "valid_table")
        # Quoted date stays as string
        assert isinstance(p.raw_frontmatter["created"], str)
        assert p.raw_frontmatter["created"] == "2026-04-04"

    def test_unquoted_date_is_date(self):
        p = parse_file(FIXTURES / "valid_table" / "simple.md", relative_to=FIXTURES / "valid_table")
        import datetime
        assert isinstance(p.raw_frontmatter["created"], datetime.date)

    def test_tags_list(self):
        p = parse_file(FIXTURES / "valid_table" / "with-tags.md", relative_to=FIXTURES / "valid_table")
        assert p.raw_frontmatter["tags"] == ["python", "testing"]

    def test_malformed_yaml(self):
        p = parse_file(FIXTURES / "invalid_table" / "malformed-yaml.md", relative_to=FIXTURES / "invalid_table")
        assert any("Malformed YAML" in e for e in p.parse_errors)


class TestH1Detection:
    def test_no_h1(self):
        p = parse_file(FIXTURES / "valid_table" / "simple.md", relative_to=FIXTURES / "valid_table")
        assert p.h1 is None

    def test_h1_present(self):
        p = parse_file(FIXTURES / "strict_table" / "valid-with-h1.md", relative_to=FIXTURES / "strict_table")
        assert p.h1 == "Valid document"

    def test_h1_not_detected_in_code_fence(self):
        p = parse_file(FIXTURES / "valid_table" / "with-code-fence.md", relative_to=FIXTURES / "valid_table")
        assert p.h1 is None  # The # inside code fence should be ignored


class TestH2Sections:
    def test_basic_sections(self):
        p = parse_file(FIXTURES / "valid_table" / "simple.md", relative_to=FIXTURES / "valid_table")
        assert len(p.sections) == 2
        assert p.sections[0].normalized_heading == "Summary"
        assert p.sections[1].normalized_heading == "Notes"
        assert "simple note for testing" in p.sections[0].body

    def test_numbered_headings_normalized(self):
        p = parse_file(
            FIXTURES / "valid_table" / "numbered-headings.md",
            relative_to=FIXTURES / "valid_table",
            normalize_numbered=True,
        )
        assert p.sections[0].raw_heading == "1. Summary"
        assert p.sections[0].normalized_heading == "Summary"
        assert p.sections[1].normalized_heading == "Notes"

    def test_numbered_headings_not_normalized(self):
        p = parse_file(
            FIXTURES / "valid_table" / "numbered-headings.md",
            relative_to=FIXTURES / "valid_table",
            normalize_numbered=False,
        )
        assert p.sections[0].normalized_heading == "1. Summary"

    def test_code_fence_content_preserved(self):
        p = parse_file(FIXTURES / "valid_table" / "with-code-fence.md", relative_to=FIXTURES / "valid_table")
        assert len(p.sections) == 2
        summary = p.sections[0]
        assert "```python" in summary.body
        assert "# This is a comment" in summary.body
        assert "Still part of Summary." in summary.body

    def test_tilde_fence(self):
        p = parse_file(FIXTURES / "valid_table" / "with-code-fence.md", relative_to=FIXTURES / "valid_table")
        notes = p.sections[1]
        assert "~~~bash" in notes.body
        assert "# Another comment" in notes.body

    def test_duplicate_sections_detected(self):
        p = parse_file(FIXTURES / "invalid_table" / "duplicate-section.md", relative_to=FIXTURES / "invalid_table")
        headings = [s.normalized_heading for s in p.sections]
        assert headings.count("Body") == 2

    def test_section_body_stripped(self):
        p = parse_file(FIXTURES / "valid_table" / "simple.md", relative_to=FIXTURES / "valid_table")
        # Body should be stripped of leading/trailing whitespace
        assert not p.sections[0].body.startswith("\n")
        assert not p.sections[0].body.endswith("\n")


class TestSchemaFileAsMarkdown:
    def test_schema_file_parses_like_any_md(self):
        p = parse_file(FIXTURES / "valid_table" / "_mdql.md", relative_to=FIXTURES / "valid_table")
        assert p.raw_frontmatter["type"] == "schema"
        assert p.raw_frontmatter["table"] == "notes"
        assert not p.parse_errors
