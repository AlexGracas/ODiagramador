# erDiagram

```mermaid
erDiagram
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ ORDER_ITEM : contains
    PRODUCT ||--o{ ORDER_ITEM : "is in"
    CUSTOMER {
        int id PK
        string name
        string email UK
    }
    ORDER {
        int id PK
        int customer_id FK
        date created_at
    }
```

Cardinality (left/right): `|o` zero or one, `||` exactly one, `}o` zero or more, `}|` one or more.
`--` identifying (solid), `..` non-identifying (dashed). The label after `:` is required;
quote it when it has spaces.
