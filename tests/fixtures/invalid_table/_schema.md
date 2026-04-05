---
type: schema
table: broken
primary_key: path

frontmatter:
  title:
    type: string
    required: true
  count:
    type: int
    required: true
  created:
    type: date
    required: true

h1:
  required: false

sections:
  Body:
    type: markdown
    required: true

rules:
  reject_unknown_frontmatter: true
  reject_unknown_sections: true
  reject_duplicate_sections: true
---

# broken

Test fixtures for invalid markdown rows.
