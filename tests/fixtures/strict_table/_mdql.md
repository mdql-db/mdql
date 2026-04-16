---
type: schema
table: docs
primary_key: path

frontmatter:
  title:
    type: string
    required: true
  author:
    type: string
    required: true
  created:
    type: datetime
    required: true

h1:
  required: true
  must_equal_frontmatter: title

sections:
  Summary:
    type: markdown
    required: true
  Details:
    type: markdown
    required: false

rules:
  reject_unknown_frontmatter: true
  reject_unknown_sections: true
  reject_duplicate_sections: true
  normalize_numbered_headings: false
---

# docs

Strict table: H1 required and must match title, unknown sections rejected.
