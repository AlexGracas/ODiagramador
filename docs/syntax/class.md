# classDiagram (UML classes)

```mermaid
classDiagram
    direction LR
    class Animal {
        <<abstract>>
        +String name
        -int age
        +makeSound()* void
        +getAge() int
    }
    class Duck {
        +swim()
    }
    class Flyer {
        <<interface>>
        +fly()
    }
    Animal <|-- Duck : inheritance
    Flyer <|.. Duck : realization
    Zoo "1" *-- "many" Animal : composition
    Keeper "1" o-- "1..*" Animal : aggregation
    Duck --> Pond : association
    Duck ..> Food : dependency
    note for Duck "Can fly and swim"
```

Visibility: `+` public, `-` private, `#` protected, `~` package. `*` abstract, `$` static
(after the method). Generics: `class List~T~`. Annotations: `<<interface>>`, `<<abstract>>`,
`<<enumeration>>`, `<<service>>`.
Relations: `<|--` inheritance, `*--` composition, `o--` aggregation, `-->` association,
`..>` dependency, `..|>` realization, `--` link. Cardinality in quotes: `"1" --> "*"`.
