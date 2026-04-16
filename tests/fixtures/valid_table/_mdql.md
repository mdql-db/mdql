---
type: schema
table: notes
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
  tags:
    type: string[]
    required: false
  status:
    type: string
    required: false
    enum: [draft, approved, archived]

h1:
  required: false

sections:
  Summary:
    type: markdown
    required: false
  Notes:
    type: markdown
    required: false

rules:
  reject_unknown_frontmatter: true
  reject_unknown_sections: false
  reject_duplicate_sections: true
  normalize_numbered_headings: true
---

# notes

Test fixture table for valid markdown rows.
