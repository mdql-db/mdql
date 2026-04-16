"""Tests for mdql.pandas integration."""

from pathlib import Path

import pandas as pd
import pytest

from mdql.loader import load_table
from mdql.pandas import load_dataframe, to_dataframe

FIXTURES = Path(__file__).parent / "fixtures"
EXAMPLES = Path(__file__).parent.parent / "examples"


class TestToDataframe:
    def test_basic_conversion(self):
        schema, rows, errors = load_table(FIXTURES / "valid_table")
        df = to_dataframe(rows, schema)
        assert isinstance(df, pd.DataFrame)
        assert "path" in df.columns
        assert "title" in df.columns
        assert len(df) == len(rows)

    def test_date_dtype(self):
        schema, rows, errors = load_table(FIXTURES / "valid_table")
        df = to_dataframe(rows, schema)
        assert pd.api.types.is_datetime64_any_dtype(df["created"])

    def test_string_dtype(self):
        schema, rows, errors = load_table(FIXTURES / "valid_table")
        df = to_dataframe(rows, schema)
        assert df["title"].dtype == "string"

    def test_string_array_kept_as_lists(self):
        schema, rows, errors = load_table(FIXTURES / "valid_table")
        df = to_dataframe(rows, schema)
        if "tags" in df.columns:
            tagged = df[df["tags"].notna()]
            if len(tagged) > 0:
                assert isinstance(tagged.iloc[0]["tags"], list)

    def test_empty_rows(self):
        schema, rows, errors = load_table(FIXTURES / "valid_table")
        df = to_dataframe([], schema)
        assert isinstance(df, pd.DataFrame)
        assert len(df) == 0


class TestLoadDataframe:
    def test_load_valid_table(self):
        df = load_dataframe(FIXTURES / "valid_table")
        assert isinstance(df, pd.DataFrame)
        assert len(df) > 0
        assert "path" in df.columns

    def test_load_with_errors_warn(self):
        with pytest.warns(match="failed validation"):
            df = load_dataframe(FIXTURES / "invalid_table", errors="warn")
        # Should still return valid rows
        assert isinstance(df, pd.DataFrame)

    def test_load_with_errors_raise(self):
        from mdql.errors import MdqlError
        with pytest.raises(MdqlError, match="failed validation"):
            load_dataframe(FIXTURES / "invalid_table", errors="raise")

    def test_load_with_errors_ignore(self):
        df = load_dataframe(FIXTURES / "invalid_table", errors="ignore")
        assert isinstance(df, pd.DataFrame)


class TestWithExampleData:
    @pytest.mark.skipif(
        not (EXAMPLES / "strategies" / "_mdql.md").exists(),
        reason="example data not present",
    )
    def test_load_strategies(self):
        df = load_dataframe(EXAMPLES / "strategies")
        assert len(df) >= 100
        assert pd.api.types.is_datetime64_any_dtype(df["created"])
        assert df["title"].dtype == "string"
        assert df["mechanism"].dtype == "Int64"
        # string[] stays as lists
        assert isinstance(df.iloc[0]["categories"], list)
