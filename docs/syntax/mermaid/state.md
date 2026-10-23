<!-- Digest of Mermaid 12.2.0 docs/syntax/stateDiagram.md — MIT © Mermaid contributors (render-kit/vendor/mermaid.LICENSE). Regenerate with scripts/sync-mermaid-docs.py -->
# State diagrams
Mermaid can render state diagrams. The syntax tries to be compliant with the syntax used in plantUml as this will make
```mermaid
---
title: Simple sample
---
stateDiagram-v2
    [*] --> Still
    Still --> [*]

    Still --> Moving
    Moving --> Still
    Moving --> Crash
    Crash --> [*]
```
```mermaid
stateDiagram
    [*] --> Still
    Still --> [*]

    Still --> Moving
    Moving --> Still
    Moving --> Crash
    Crash --> [*]
```

## Default theme, look and layout (v12.0.0+)
State diagrams use the `redux-color` theme and the `neo` look by default, and are laid out by [ELK](https://www.eclipse.org/elk/) rather than Dagre. Not every diagram type

### With the defaults
```mermaid
stateDiagram-v2
  [*] --> Draft
  Draft --> Submitted : submit
  state Review {
    [*] --> Screening
    Screening --> Decision
  }
  Submitted --> Review
  Review --> Published : approved
  Review --> Draft : rejected
  Published --> [*]
```

### The previous appearance
Both are only defaults, so anything you set yourself wins. Naming the previous theme and look
```mermaid
---
config:
  theme: default
  look: classic
  layout: dagre
---
stateDiagram-v2
  [*] --> Draft
  Draft --> Submitted : submit
  state Review {
    [*] --> Screening
    Screening --> Decision
  }
  Submitted --> Review
  Review --> Published : approved
  Review --> Draft : rejected
  Published --> [*]
```

## States
A state can be declared in multiple ways. The simplest way is to define a state with just an id:
```mermaid
stateDiagram-v2
    stateId
```
```mermaid
stateDiagram-v2
    state "This is a state description" as s2
```
```mermaid
stateDiagram-v2
    s2 : This is a state description
```

## Transitions
Transitions are path/edges when one state passes into another. This is represented using text arrow, "-->".
```mermaid
stateDiagram-v2
    s1 --> s2
```
```mermaid
stateDiagram-v2
    s1 --> s2: A transition
```

## Start and End
There are two special states indicating the start and stop of the diagram. These are written with the \[\*] syntax and
```mermaid
stateDiagram-v2
    [*] --> s1
    s1 --> [*]
```

## Composite states
In a real world use of state diagrams you often end up with diagrams that are multidimensional as one state can
```mermaid
stateDiagram-v2
    [*] --> First
    state First {
        [*] --> second
        second --> [*]
    }

    [*] --> NamedComposite
    NamedComposite: Another Composite
    state NamedComposite {
        [*] --> namedSimple
        namedSimple --> [*]
        namedSimple: Another simple
    }
```
```mermaid
stateDiagram-v2
    [*] --> First

    state First {
        [*] --> Second

        state Second {
            [*] --> second
            second --> Third

            state Third {
                [*] --> third
                third --> [*]
            }
        }
    }
```
```mermaid
stateDiagram-v2
    [*] --> First
    First --> Second
    First --> Third

    state First {
        [*] --> fir
        fir --> [*]
    }
    state Second {
        [*] --> sec
        sec --> [*]
    }
    state Third {
        [*] --> thi
        thi --> [*]
    }
```

## Choice
Sometimes you need to model a choice between two or more paths, you can do so using <\<choice>>.
```mermaid
stateDiagram-v2
    state if_state <<choice>>
    [*] --> IsPositive
    IsPositive --> if_state
    if_state --> False: if n < 0
    if_state --> True : if n >= 0
```

## Forks
It is possible to specify a fork in the diagram using <\<fork>> <\<join>>.
```mermaid
   stateDiagram-v2
    state fork_state <<fork>>
      [*] --> fork_state
      fork_state --> State2
      fork_state --> State3

      state join_state <<join>>
      State2 --> join_state
      State3 --> join_state
      join_state --> State4
      State4 --> [*]
```

## Notes
Sometimes nothing says it better than a Post-it note. That is also the case in state diagrams.
```mermaid
    stateDiagram-v2
        State1: The state with a note
        note right of State1
            Important information! You can write
            notes.
        end note
        State1 --> State2
        note left of State2 : This is the note to the left.
```

## Concurrency
As in plantUml you can specify concurrency using the -- symbol.
```mermaid
stateDiagram-v2
    [*] --> Active

    state Active {
        [*] --> NumLockOff
        NumLockOff --> NumLockOn : EvNumLockPressed
        NumLockOn --> NumLockOff : EvNumLockPressed
        --
        [*] --> CapsLockOff
        CapsLockOff --> CapsLockOn : EvCapsLockPressed
        CapsLockOn --> CapsLockOff : EvCapsLockPressed
        --
        [*] --> ScrollLockOff
        ScrollLockOff --> ScrollLockOn : EvScrollLockPressed
        ScrollLockOn --> ScrollLockOff : EvScrollLockPressed
    }
```

## Setting the direction of the diagram
With state diagrams you can use the direction statement to set the direction which the diagram will render like in this
```mermaid
stateDiagram
    direction LR
    [*] --> A
    A --> B
    B --> C
    state B {
      direction LR
      a --> b
    }
    B --> D
```

## Comments
Comments can be entered within a state diagram chart, which will be ignored by the parser. Comments must be prefaced with `%%` (double percent signs) and any text after their start to the next newline will be treated as a comment, including any diagram syntax. They can be on their own line or at the end of a statement.
```mermaid
stateDiagram-v2
    [*] --> Still
    Still --> [*]
%% this is a comment
    Still --> Moving
    Moving --> Still %% another comment
    Moving --> Crash
    Crash --> [*]
```

## Styling with classDefs
As with other diagrams (like flowcharts), you can define a style in the diagram itself and apply that named style to a
1. Cannot be applied to start or end states
2. Cannot be applied to or within composite states
```txt
classDef movement font-style:italic;
```
- the _name_ of the style is `movement`
- the only _property_ is `font-style` and its _value_ is `italic`
```txt
classDef badBadEvent fill:#f00,color:white,font-weight:bold,stroke-width:2px,stroke:yellow
```
- the _name_ of the style is `badBadEvent`
- the first _property_ is `fill` and its _value_ is `#f00`
- the second _property_ is `color` and its _value_ is `white`
- the third _property_ is `font-weight` and its _value_ is `bold`
- the fourth _property_ is `stroke-width` and its _value_ is `2px`
- the fifth _property_ is `stroke` and its _value_ is `yellow`

### Apply classDef styles to states
There are two ways to apply a `classDef` style to a state:
1. use the `class` keyword to apply a classDef style to one or more states in a single statement, or
2. use the `:::` operator to apply a classDef style to a state as it is being used in a transition statement (e.g. with an arrow

#### 1. `class` statement
A `class` statement tells Mermaid to apply the named classDef to one or more classes. The form is:
```txt
class [one or more state names, separated by commas] [name of a style defined with classDef]
```
```txt
class Crash badBadEvent
```
```txt
class Moving, Crash movement
```
```mermaid
   stateDiagram
   direction TB

   accTitle: This is the accessible title
   accDescr: This is an accessible description

   classDef notMoving fill:white
   classDef movement font-style:italic
   classDef badBadEvent fill:#f00,color:white,font-weight:bold,stroke-width:2px,stroke:yellow

   [*]--> Still
   Still --> [*]
   Still --> Moving
   Moving --> Still
   Moving --> Crash
   Crash --> [*]

   class Still notMoving
   class Moving, Crash movement
   class Crash badBadEvent
   class end badBadEvent
```

#### 2. `:::` operator to apply a style to a state
You can apply a classDef style to a state using the `:::` (three colons) operator. The syntax is
```txt
[state]:::[style name]
```
```mermaid
stateDiagram
   direction TB

   accTitle: This is the accessible title
   accDescr: This is an accessible description

   classDef notMoving fill:white
   classDef movement font-style:italic;
   classDef badBadEvent fill:#f00,color:white,font-weight:bold,stroke-width:2px,stroke:yellow

   [*] --> Still:::notMoving
   Still --> [*]
   Still --> Moving:::movement
   Moving --> Still
   Moving --> Crash:::movement
   Crash:::badBadEvent --> [*]
```

## Spaces in state names
Spaces can be added to a state by first defining the state with an id and then referencing the id later.
```mermaid
stateDiagram
    classDef yourState font-style:italic,font-weight:bold,fill:white

    yswsii: Your state with spaces in it
    [*] --> yswsii:::yourState
    [*] --> SomeOtherState
    SomeOtherState --> YetAnotherState
    yswsii --> YetAnotherState
    YetAnotherState --> [*]
```
