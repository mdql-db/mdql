"""Tests for mdql.cli."""

from pathlib import Path

from typer.testing import CliRunner

from mdql.cli import app

FIXTURES = Path(__file__).parent / "fixtures"
runner = CliRunner()


class TestValidateCommand:
    def test_valid_table(self):
        result = runner.invoke(app, ["validate", str(FIXTURES / "valid_table")])
        assert result.exit_code == 0
        assert "valid" in result.stdout.lower()

    def test_invalid_table(self):
        result = runner.invoke(app, ["validate", str(FIXTURES / "invalid_table")])
        assert result.exit_code == 1

    def test_strict_table_valid_file(self):
        result = runner.invoke(app, ["validate", str(FIXTURES / "strict_table")])
        # Has both valid and invalid files
        assert result.exit_code == 1

    def test_missing_schema(self, tmp_path):
        result = runner.invoke(app, ["validate", str(tmp_path)])
        assert result.exit_code == 1
        assert "error" in result.stdout.lower() or "error" in (result.stderr or "").lower()


class TestSchemaCommand:
    def test_show_schema(self):
        result = runner.invoke(app, ["schema", str(FIXTURES / "valid_table")])
        assert result.exit_code == 0
        assert "notes" in result.stdout
        assert "title" in result.stdout
        assert "string" in result.stdout


class TestInspectCommand:
    def test_inspect_all(self):
        result = runner.invoke(app, ["inspect", str(FIXTURES / "valid_table")])
        assert result.exit_code == 0
        assert "Simple note" in result.stdout

    def test_inspect_single_file(self):
        result = runner.invoke(app, ["inspect", str(FIXTURES / "valid_table"), "-f", "simple.md"])
        assert result.exit_code == 0
        assert "Simple note" in result.stdout

    def test_inspect_json(self):
        result = runner.invoke(app, ["inspect", str(FIXTURES / "valid_table"), "--format", "json"])
        assert result.exit_code == 0
        assert '"title"' in result.stdout

    def test_inspect_missing_file(self):
        result = runner.invoke(app, ["inspect", str(FIXTURES / "valid_table"), "-f", "nonexistent.md"])
        assert result.exit_code == 1
