"""MDQL error types — compatible with original Python exceptions."""


class MdqlError(Exception):
    """Base error for all MDQL operations."""
    pass


class ValidationError(MdqlError):
    """A file validation failure."""

    def __init__(self, file_path: str, error_type: str, message: str,
                 field: str | None = None, line_number: int | None = None):
        self.file_path = file_path
        self.error_type = error_type
        self.field = field
        self.message = message
        self.line_number = line_number
        super().__init__(message)

    def __str__(self):
        loc = f" (line {self.line_number})" if self.line_number else ""
        return f"{self.file_path}{loc}: {self.message}"


class QueryParseError(MdqlError):
    """SQL parsing failure."""
    pass


class SchemaNotFoundError(MdqlError):
    """No _mdql.md schema file found."""
    pass


class SchemaInvalidError(MdqlError):
    """Schema file is malformed."""
    pass


class JournalRecoveryError(MdqlError):
    """Journal recovery failure."""
    pass
