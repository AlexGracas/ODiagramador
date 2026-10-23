<!-- Digest of Mermaid 12.2.0 docs/syntax/treeView.md — MIT © Mermaid contributors (render-kit/vendor/mermaid.LICENSE). Regenerate with scripts/sync-mermaid-docs.py -->
# TreeView Diagram (v11.14.0+)

## Introduction
A TreeView diagram is used to represent hierarchical data in the form of a directory-like structure, with file/folder icons, connector lines, and optional annotations.

## Syntax
The structure of the tree depends only on indentation. Labels can be **bare** (unquoted) or **quoted** (for names containing spaces).
- Directories are indicated by a trailing `/` on the label — they render in bold text.
- Icons are hidden by default — enable the built-in file/folder icons with the `showIcons` config option, or set one per node with `icon()`.
- Quoted labels (`"my file"`) support spaces in names.
```
treeView-beta
    my-project/
        src/
            index.js
        package.json
        README.md
```
```
treeView-beta
    "my project"
        "folder with spaces"
            "file.js"
```

## Box-Drawing Input
As an alternative to indentation, you can use box-drawing characters to define the tree structure. The parser auto-detects the format — no extra keyword or config is needed. This is how most file tree diagrams are drawn already, so you can turn those into Mermaid diagrams with very little effort.
```mermaid
treeView-beta
├── src/
│   ├── index.ts
│   └── utils.ts
├── package.json
└── README.md
```
```mermaid
treeView-beta
├── src/
│   ├── App.tsx :::highlight icon(logos:react) ## main component
│   └── index.ts ## entry point
├── .env ## environment variables
├── Dockerfile
└── package.json
```
```mermaid
treeView-beta
├── packages/
│   ├── mermaid/
│   │   ├── src/
│   │   │   ├── parser.ts
│   │   │   └── renderer.ts
│   │   └── package.json
│   └── parser/
│       └── src/
└── README.md
```

## Annotations

### Highlighting with :::class
Annotate a node with `:::className` to apply a CSS class. A built-in `highlight` class is provided:
```mermaid
treeView-beta
    src/
        App.tsx :::highlight
        index.js
    package.json
```

### Inline descriptions with `##`
Add a visible description after `##` — rendered next to the label in italic:
```mermaid
treeView-beta
    src/
        index.js ## app entry point
        config.ts ## runtime configuration
    package.json ## project manifest
```

### Icons
Icons are hidden by default. Set the `showIcons` config option to `true` to show the built-in icons — `file` for files and `folder` for directories:
```mermaid
---
config:
  treeView:
    showIcons: true
---
treeView-beta
    src/
        index.js
    package.json
```

#### File-type icons via config maps
Mermaid ships no filename/extension mapping — file-type icons are fully user-configured through the `filenameIcons` and `extensionIcons` config options, using icons from a registered [icon pack](../config/icons.md) such as [material-icon-theme](https://icon-sets.iconify.design/material-icon-theme/). Values are resolved like `icon()` references — `pack:name` is used as-is, unprefixed names resolve …
```mermaid
---
config:
  treeView:
    showIcons: true
    defaultIconPack: material-icon-theme
    filenameIcons:
      Dockerfile: docker
    extensionIcons:
      .ts: typescript
      .tsx: react-ts
      .txt: none
---
treeView-beta
    src/
        App.tsx
        utils.ts
    Dockerfile
    notes.txt
    README.md
```

#### Icon overrides with icon()
Set a node's icon explicitly with `icon(name)`, where `name` is any icon from a registered [icon pack](../config/icons.md), referenced as `pack:name`. Explicit icons always render, even when `showIcons` is off:
```mermaid
treeView-beta
    src/
        App.tsx icon(logos:react)
        index.js
    package.json
```

#### Hiding icons
When `showIcons` is enabled, use `icon()` or `icon(none)` to hide the icon of a single node:
```mermaid
---
config:
  treeView:
    showIcons: true
---
treeView-beta
    src/
        index.js icon(none)
    package.json
```

### Combined annotations
Annotations can be combined in any order:
```mermaid
treeView-beta
    my-project/
        src/
            App.tsx :::highlight icon(logos:react) ## main component
            index.js ## entry point
        .env ## environment variables
        Dockerfile
        package.json
```

## Comments
Use `%%` for invisible comments (standard Mermaid convention):
```
treeView-beta
    %% Generated files — do not edit
    src/
        generated/
        index.js
```

## Examples
Basic with quoted labels:
```mermaid
treeView-beta
    "packages"
        "mermaid"
            "src"
        "parser"
```
```mermaid
treeView-beta
    🚀 rocket-app/
        📦 packages/
            🎨 ui/
            🛠️ utils/
        🧪 tests/
        📝 README.md
        ⚙️ config.yaml
```
```mermaid
---
config:
    treeView:
        rowIndent: 80
        lineThickness: 3
    themeVariables:
        treeView:
            labelFontSize: '20px'
            labelColor: '#FF0000'
            lineColor: '#00FF00'
---
treeView-beta
    "packages"
        "mermaid"
            "src"
        "parser"
```

## Config Variables
| Property        | Description                                                                      | Default Value |
| --------------- | -------------------------------------------------------------------------------- | ------------- |
| rowIndent       | Indentation for each row                                                         | 10            |
| paddingX        | Horizontal padding of row                                                        | 5             |
| paddingY        | Vertical padding of row                                                          | 5             |
| lineThickness   | Thickness of the line                                                            | 1             |
| showIcons       | Whether to show the default file/folder icons (explicit `icon()` always renders) | false         |
| defaultIconPack | Registered iconify pack used to resolve unprefixed icon references               | ''            |
| filenameIcons   | Filename → icon map for file-type icons                                          | {}            |
| extensionIcons  | Extension → icon map for file-type icons                                         | {}            |

### Theme Variables
| Property         | Description                                               | Default Value        |
| ---------------- | --------------------------------------------------------- | -------------------- |
| labelFontSize    | Font size of the label                                    | '16px'               |
| labelColor       | Color of the label                                        | 'black'              |
| lineColor        | Color of the line                                         | 'black'              |
| iconColor        | Color of icons (applies to icons that use `currentColor`) | '#546e7a'            |
| descriptionColor | Color of `##` description text                            | '#6a9955'            |
| highlightBg      | Highlight background fill                                 | rgba(255,193,7,0.15) |
| highlightStroke  | Highlight border stroke                                   | #ffc107              |
