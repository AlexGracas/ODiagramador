<!-- Digest of Mermaid 12.2.0 docs/syntax/agentflow.md — MIT © Mermaid contributors (render-kit/vendor/mermaid.LICENSE). Regenerate with scripts/sync-mermaid-docs.py -->
# Agentflow (v12.0.0+)

## Introduction
A flowchart tells you what happens next. An agentflow tells you _who_ is doing it, _with which tool_, and _what contract_ holds at each step. It keeps the familiar flowchart feel — nodes, arrows, containers — but adds a small vocabulary aimed at describing systems built out of language-model agents:
- **Containers** (`flow … end`) group work and nest to any depth.
- **Shapes** carry meaning: a `task` is not a `tool` is not a `decision`.
- **Edges** carry meaning: sequence, reference, and failure are three different arrows.
- **Metadata** (`@{ … }`) attaches the non-visual detail — the model, the instruction, the parameter and return types, the connector a tool is bound to.

## Basic example
```mermaid
agentflow-beta TB
  flow reviewer["Review Agent"]
    changes["Gather changes"]@{ shape: input }
    analyse["Analyse diff"]@{ shape: task }
    lint["run_linter"]@{ shape: tool }
    ok["Clean?"]@{ shape: decision }

    changes --> analyse --> lint --> ok
  end
```

## Default theme, look and layout (v12.0.0+)
Agentflow diagrams use the `redux-color` theme and the `neo` look by default, and are laid out by [ELK](https://www.eclipse.org/elk/) rather than Dagre. Not every

### With the defaults
```mermaid
agentflow-beta TB
  brief["Release brief"]@{ shape: input }
  flow writer["Drafting Agent"]
    draft["Draft the notes"]@{ shape: task }
    lookup["changelog_search"]@{ shape: tool }
    guide["Tone of voice"]@{ shape: refdoc }
    draft --> lookup
    draft -.- guide
  end
  flow reviewer["Review Agent"]
    check["Check the claims"]@{ shape: task }
    ok["Accurate?"]@{ shape: decision }
    check --> ok
  end
  publish["Publish"]@{ shape: action }
  brief --> writer
  writer --> reviewer
  ok --> publish
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
agentflow-beta TB
  brief["Release brief"]@{ shape: input }
  flow writer["Drafting Agent"]
    draft["Draft the notes"]@{ shape: task }
    lookup["changelog_search"]@{ shape: tool }
    guide["Tone of voice"]@{ shape: refdoc }
    draft --> lookup
    draft -.- guide
  end
  flow reviewer["Review Agent"]
    check["Check the claims"]@{ shape: task }
    ok["Accurate?"]@{ shape: decision }
    check --> ok
  end
  publish["Publish"]@{ shape: action }
  brief --> writer
  writer --> reviewer
  ok --> publish
```

## Declaring a diagram
Every diagram starts with the `agentflow-beta` keyword, optionally followed by a direction — `TB`, `TD`, `BT`, `LR`, or `RL`.
```txt
agentflow-beta LR
```

## Nodes and shapes
Nodes are declared exactly as in a flowchart: an id, optionally followed by a label in brackets.
```txt
research["Research the topic"]
```
| Alias      | Meaning                                                  | Underlying shape |
| ---------- | -------------------------------------------------------- | ---------------- |
| `task`     | A unit of work an agent performs                         | `roundedRect`    |
| `tool`     | A callable capability — a function, an API, a subroutine | `subroutine`     |
| `input`    | Data entering the workflow                               | `lean-right`     |
| `decision` | A branch point                                           | `diamond`        |
| `refdoc`   | Reference material an agent consults                     | `lin-doc`        |
| `action`   | A side-effecting step — sending, writing, publishing     | `hexagon`        |
```mermaid
agentflow-beta LR
  brief["Brief"]@{ shape: input }
  draft["Draft copy"]@{ shape: task }
  spellcheck["spell_check"]@{ shape: tool }
  guide["Style guide"]@{ shape: refdoc }
  publish["Publish"]@{ shape: action }

  brief --> draft --> spellcheck --> publish
  draft -.- guide
```

## Edges
Agentflow uses three edge operators, each with a distinct meaning:
| Operator | Semantic    | Reads as                                          |
| -------- | ----------- | ------------------------------------------------- |
| `-->`    | `sequence`  | Control or data flows from source to target       |
| `-.-`    | `reference` | The source consults the target; no control passes |
| `--x`    | `failure`   | The failure path out of the source                |
```mermaid
agentflow-beta TB
  check["Tests pass?"]@{ shape: decision }
  ship["Ship it"]@{ shape: action }
  fix["Fix the build"]@{ shape: task }
  logs["Build logs"]@{ shape: refdoc }

  check -- yes --> ship
  check -- no --> fix
  fix --x check
  fix -.- logs
```

## Containers
A `flow … end` block groups nodes into a container. Containers nest, and a node referenced inside a container belongs to it.
```mermaid
agentflow-beta TB
  flow team["Content Team"]
    flow researcher["Researcher"]
      gather["Gather sources"]@{ shape: task }
    end

    flow writer["Writer"]
      compose["Compose draft"]@{ shape: task }
    end

    researcher --> writer
  end
```

### The `global` block
Sometimes a node is shared by several flows and should not be pulled into whichever one happens to mention it first. Declare it inside a `global … end` block and it stays at the top level no matter where it is referenced:
```mermaid
agentflow-beta TB
  global
    corpus["Shared corpus"]@{ shape: refdoc }
  end

  flow summariser["Summariser"]
    summarise["Summarise"]@{ shape: task }
    summarise -.- corpus
  end

  flow indexer["Indexer"]
    index["Build index"]@{ shape: task }
    index -.- corpus
  end
```

### Collapsing a container
`@{ view: collapsed }` folds a container down to a single summary node while keeping its edges. Edges that crossed the boundary terminate at the collapsed node instead of disappearing.
```mermaid
agentflow-beta TB
  flow intake["Intake"]
    receive["Receive request"]@{ shape: input }
  end

  flow processing["Processing"]
    validate["Validate"]@{ shape: task }
    enrich["Enrich"]@{ shape: task }
    validate --> enrich
  end
  processing@{ view: "collapsed" }

  flow output["Output"]
    publish["Publish"]@{ shape: action }
  end

  receive --> validate
  enrich --> publish
```

## Metadata
Any node, edge, or container can carry an `@{ … }` block. The contents are YAML, so both the single-line form and a multi-line block work:
```txt
researcher@{ model: "claude-sonnet-4-20250514", instruction: "Research and cite sources." }
```
```txt
fetch@{
  shape: tool
  params: "city :: String"
  returns: "Report"
  retry: 2
}
```
| Key                             | Applies to | Effect                                                 |
| ------------------------------- | ---------- | ------------------------------------------------------ |
| `shape`                         | nodes      | Selects the node shape (see the table above)           |
| `label`, `labelType`            | nodes      | Overrides the node's label and how it is parsed        |
| `view`                          | containers | `collapsed` folds the container down to a summary node |
| `algorithm`                     | containers | Per-container ELK algorithm                            |
| `curve`, `animate`, `animation` | edges      | Edge interpolation and animation                       |
| Key            | Applies to        | Purpose                                        |
| -------------- | ----------------- | ---------------------------------------------- |
| `description`  | anything          | Free-text description                          |
| `instruction`  | agents, flows     | The system prompt or standing instruction      |
| `model`        | agents, flows     | The model the agent runs on                    |
| `params`       | tools, containers | Input contract, e.g. `"city :: String"`        |
| `returns`      | tools, containers | Output contract                                |
| `value`        | input nodes       | A concrete value                               |
| `example`      | input nodes       | An example value                               |
| `connectorRef` | tools, actions    | The connector capability this node is bound to |

## Connectors
A connector describes an external system a tool talks to. Declare it with the `connector` keyword and configure it with metadata; tools then point at one of its capabilities with `connectorRef`.
```mermaid
agentflow-beta LR
  connector github["GitHub API"]
  github@{ protocol: "http", endpoint: "https://api.github.com" }

  title["Issue title"]@{ shape: input }
  create["create_issue"]@{ shape: tool, connectorRef: "github.create_issue" }

  title --> create
```

## A worked example
```mermaid
agentflow-beta TB
  connector llm["LLM API"]
  llm@{ protocol: "http", endpoint: "https://api.example.com/chat" }

  flow coffee_team["Coffee Team"]
    city["city"]@{ shape: input, value: "Stockholm" }

    flow researcher["Researcher"]
      research["research_location"]@{ shape: tool, params: "city :: String", returns: "Report" }
      write["write_copy"]@{ shape: tool, connectorRef: "llm.chat", returns: "CoffeeCopy" }
      city --> research --> write
    end
    researcher@{ instruction: "Research the city and draft English coffee copy citing sources." }

    flow designer["Designer"]
      render["generate_html"]@{ shape: tool, connectorRef: "llm.chat", returns: "String" }
      brand["Nordic Brand Guide"]@{ shape: refdoc }
      render -.- brand
    end

    researcher --> designer
  end
```

## Configuration
Agentflow has its own config namespace, so it can be tuned without moving flowcharts on the same page.
| Option           | Description                                      | Default |
| ---------------- | ------------------------------------------------ | ------- |
| `titleTopMargin` | Margin above the diagram title                   | `25`    |
| `diagramPadding` | Padding around the diagram as a whole, in pixels | `8`     |
| `nodeSpacing`    | Spacing between nodes on the same level          | `50`    |
| `rankSpacing`    | Spacing between nodes on different levels        | `50`    |
| `useMaxWidth`    | Scale the diagram to the available width         | `true`  |
```mermaid
---
config:
  agentflow:
    nodeSpacing: 70
    rankSpacing: 70
---
agentflow-beta TB
  a["A"] --> b["B"]
```

### Theme variables
Containers are themed with `flowContainerStroke`, defined by every theme mermaid

### Layout
Agentflow renders through the unified renderer, so it works with any registered layout engine. ELK is the default, and an individual container can also select its own ELK algorithm:
```mermaid
---
config:
  layout: elk
---
agentflow-beta TB
  flow tools["Tools"]
    a["a"]@{ shape: tool }
    b["b"]@{ shape: tool }
  end
  tools@{ algorithm: "elk.rectpacking" }
```

## For tool builders
Two accessors on the diagram DB are meant for programs rather than people:
- **`getSemanticModel()`** returns the diagram with presentation-only controls (`view`, `class`, `style`, `icon`, `img`, `w`, `h`) stripped, so downstream consumers get a stable semantic view that does not shift when someone restyles the diagram.
- **`getDiagnostics()`** returns structured warnings — unresolved references, misapplied metadata keys, containment violations, unsatisfied capability requirements — each with a source position. Diagnostics are warnings; they never block a render.
