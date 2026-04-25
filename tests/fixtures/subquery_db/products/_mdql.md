---
type: schema
table: products
primary_key: path

frontmatter:
  name:
    type: string
    required: true
  category:
    type: string
    required: true
  price:
    type: int
    required: true
  quantity:
    type: int
    required: true

h1:
  required: false

sections: {}

rules:
  reject_unknown_frontmatter: false
  reject_unknown_sections: false
---
