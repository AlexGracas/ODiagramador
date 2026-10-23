# stateDiagram-v2 (UML state machines)

```mermaid
stateDiagram-v2
    direction LR
    [*] --> Idle
    Idle --> Running : start
    Running --> Paused : pause
    Paused --> Running : resume
    Running --> [*] : stop

    state Running {
        [*] --> Loading
        Loading --> Ready : loaded
    }

    state check <<choice>>
    Ready --> check
    check --> Done : ok
    check --> Failed : error

    state fork_state <<fork>>
    note right of Failed : retries 3 times
```

`[*]` is start/end. Composite states use `state Name { ... }`. Pseudo-states: `<<choice>>`,
`<<fork>>`, `<<join>>`. Concurrency inside a composite state: separate regions with `--`.
Labels with spaces: `state "Waiting for payment" as Waiting`.
