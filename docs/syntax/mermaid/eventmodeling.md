<!-- Digest of Mermaid 12.2.0 docs/syntax/eventmodeling.md — MIT © Mermaid contributors (render-kit/vendor/mermaid.LICENSE). Regenerate with scripts/sync-mermaid-docs.py -->
# Event Modeling Diagram (v11.15.0+)

## Introduction
You can read more at [Event Modeling web page](https://eventmodeling.org/).
- Trigger - UI and Processor
- Command
- View / Read Model
- Event
- State View
- State Change
- Translation
- Automation

## Basic Syntax
The DSL supports two types of syntaxes - compact and relaxed one. Both will be mentioned in the examples and both can be used interchangeably.

### Timeline
The timeline is the key part of the diagram and it is composed of **Time Frame** definitions. Speedy Time Frame typing is the key of the compact notation. For compact notation you type the `tf` token. For the relaxed notation you type `timeframe` token.
```md
eventmodeling

tf 01 ui CartUI
tf 02 cmd AddItem
tf 03 evt ItemAdded
```
```mermaid
eventmodeling

tf 01 ui CartUI
tf 02 cmd AddItem
tf 03 evt ItemAdded
```
```md
eventmodeling

timeframe 01 ui CartUI
timeframe 02 command AddItem
timeframe 03 event ItemAdded
```

### Inline data
It is possible to provide data examples for individual Time Frames. An Inline Data is wrapped in curly brackets on the same line as the Time Frame.
```mermaid
eventmodeling

tf 01 ui CartUI
tf 02 cmd AddItem { description: string }
tf 03 evt ItemAdded { description: string }
```
```mermaid
eventmodeling

timeframe 01 ui CartUI
timeframe 02 command AddItem { description: string }
timeframe 03 event ItemAdded { description: string }
```

### Data block
If you need to provide more complex data description, you can define the **Data Block** separately. Data Block is referenced from a Time Frame by providing the identifier inside of double-square brackets `[[identifier]]`, similar to wiki links convention in Markdown.
```mermaid
eventmodeling

tf 01 ui CartUI
tf 02 cmd AddItem [[AddItem01]]
tf 03 evt ItemAdded [[ItemAdded]]
tf 04 cmd AddItem [[AddItem02]]
tf 05 evt ItemAdded [[ItemAdded]]

data AddItem01 {
  description: 'john'
  image: 'avatar_john'
  price: 20.4
}

data AddItem02 {
  description: 'jack'
  image: 'avatar_jack'
  price: 12.5
}

data ItemAdded {
  description: string
  image: string
  price: number
}
```
```mermaid
eventmodeling

timeframe 01 ui CartUI
timeframe 02 command AddItem [[AddItem01]]
timeframe 03 event ItemAdded [[ItemAdded]]
timeframe 04 command AddItem [[AddItem02]]
timeframe 05 event ItemAdded [[ItemAdded]]

data AddItem01 {
  description: 'john'
  image: 'avatar_john'
  price: 20.4
}

data AddItem02 {
  description: 'jack'
  image: 'avatar_jack'
  price: 12.5
}

data ItemAdded {
  description: string
  image: string
  price: number
}
```

### Resetting the flow
By default the diagram builds the relations based on the inference. But modeling a more complex business flow requires to break such inference from time to time. For that you can use a different type of Time Frame called **Reset Frame**. It is represented by a `rf` / `resetframe` token.
```mermaid
eventmodeling

tf 01 ui CartUI
tf 02 cmd AddItem
tf 03 evt ItemAdded

rf 04 evt External.InventoryChanged
tf 05 pcr InventoryProcessor
tf 06 cmd ChangeInventory
tf 07 evt Cart.InventoryChanged
```
```mermaid
eventmodeling

timeframe 01 ui CartUI
timeframe 02 command AddItem
timeframe 03 event ItemAdded

resetframe 04 event External.InventoryChanged
timeframe 05 processor InventoryProcessor
timeframe 06 command ChangeInventory
timeframe 07 event Cart.InventoryChanged
```
```mermaid
eventmodeling

tf 01 evt ItemAdded
tf 02 ui OtherUI
rf 03 rmo CartItems ->> 01
```

### Multiple relations
There are situations where you need to specify multiple relations for an entity. For example when a **Read Model** is built of multiple **Events**. You can use `->>` token for that.
```mermaid
eventmodeling

rf 02 evt CartCreated
rf 03 evt ItemAdded
rf 04 evt ItemRemoved
rf 05 evt CartCleared
tf 01 rmo CartUI ->> 02 ->> 03 ->> 04 ->> 05
```

## Event Modeling patterns
This chapter briefly summarizes each Event Modeling pattern.

### State Change
```mermaid
eventmodeling

tf 01 ui CartUI
tf 02 cmd AddItem
tf 03 evt ItemAdded
```

### State View
```mermaid
eventmodeling

tf 03 evt ItemAdded
tf 02 rmo CartItems
tf 04 ui CartUI
```

### Translation
```mermaid
eventmodeling

tf 03 evt External.InventoryChanged
tf 02 pcr InventoryProcessor
tf 04 cmd ChangeInventory
tf 05 evt Cart.InventoryChanged
```

## Details of the Syntax

### Entity types
Entity type is determined by the third column in the Time Frame grammar.
```md
tf 01 pcr InventoryProcessor
```
```md
timeframe 01 processor InventoryProcessor
```
- `ui`: UI - belongs to UI/Automation swimlane
- `pcr` / `processor`: Processor - belongs to UI/Automation swimlane
- `cmd` / `command`: Command - belongs to Command/Read Model swimlane
- `rmo` / `readmodel`: Read Model - belongs to Command/Read Model swimlane
- `evt` / `event`: Event - belongs to Events swimlane

### Swimlanes and Namespaces
There are three basic **Swimlanes** inferred from the Entity Type.
- UI/Automation
- Command/Read Model
- Events
```md
Inventory.InventoryChanged
```
```mermaid
eventmodeling

rf 01 evt Inventory.InventoryChanged
rf 02 evt External.InventoryChanged
```

### Data block data types
A **Data** is always wrapped in curly brackets. It can be prepended by a data type in backticks. The pattern applies for **Inline Data** as well as for **Data Block**.
```md
tf 01 rmo UserAdded `json`{ "name": "foo" }
```
```md
data UserAdded01 `json`{
"name": "foo"
}
```
- `json`
- `jsobj`
- `figma`
- `salt`
- `uri`
- `md`
- `html`
- `text`
