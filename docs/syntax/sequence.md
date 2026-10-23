# sequenceDiagram

```mermaid
sequenceDiagram
    autonumber
    actor U as User
    participant W as Web app
    participant A as API
    participant D as Database

    U->>W: Click "Buy"
    W->>+A: POST /orders
    A->>D: INSERT order
    D-->>A: id = 42
    alt payment ok
        A-->>W: 201 Created
    else payment refused
        A-->>W: 402 Payment Required
    end
    A-->>-W: done
    Note over W,A: All calls use HTTPS
    loop every 5s
        W->>A: GET /orders/42
    end
    opt cache enabled
        W->>W: read cache
    end
```

Arrows: `->>` solid with head, `-->>` dashed with head, `->` / `-->` without head, `-x` cross,
`-)` async. `+`/`-` after the arrow activates/deactivates the target.
Blocks: `alt/else/end`, `opt/end`, `loop/end`, `par/and/end`, `critical/option/end`, `break/end`,
`rect rgb(240,240,255) ... end`.

Common errors: a message needs `: text`; every `alt`/`loop`/`opt` needs its `end`;
participant aliases are declared before use (`participant A as API`).
