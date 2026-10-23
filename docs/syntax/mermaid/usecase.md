<!-- Digest of Mermaid 12.2.0 docs/syntax/usecase.md — MIT © Mermaid contributors (render-kit/vendor/mermaid.LICENSE). Regenerate with scripts/sync-mermaid-docs.py -->
# Use case diagrams (12.0.0+)
Use case diagrams show how actors interact with a system and its use cases. Start a diagram with the `usecase-beta` keyword. Put each statement on its own physical line.
```mermaid
usecase-beta
direction LR
actor Customer("Customer")
systemBoundary "Order system"
  Checkout("Place order")
end
Customer --> Checkout
```

## Default theme, look and layout (v12.0.0+)
Use case diagrams use the `redux-color` theme and the `neo` look by default, and are laid out by [ELK](https://www.eclipse.org/elk/) rather than Dagre. Not every diagram type

### With the defaults
```mermaid
usecase-beta
direction LR
actor Customer
actor Support
systemBoundary Storefront
  Browse("Browse catalogue")
  Checkout("Checkout")
end
systemBoundary Fulfilment
  Track("Track delivery")
end
Customer --> Browse
Customer --> Checkout
Customer --> Track
Support --> Track
Checkout ..> : include Browse
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
usecase-beta
direction LR
actor Customer
actor Support
systemBoundary Storefront
  Browse("Browse catalogue")
  Checkout("Checkout")
end
systemBoundary Fulfilment
  Track("Track delivery")
end
Customer --> Browse
Customer --> Checkout
Customer --> Track
Support --> Track
Checkout ..> : include Browse
```

## Actors and use cases
Actor and use case identifiers match `[A-Za-z0-9_]+`, so they may start with a digit — `1`, `1mg`, and `3rd` are all valid. Identifiers are diagram-wide and are shared by actors, use cases, boundaries, JSON nodes, and explicit edges.
```mermaid
usecase-beta
Customer --> Login
actor Customer
actor Admin("Main administrator")
Login("Sign in")
Report[Generate report]
"Reset password"
Admin --> Report
```

### Actor variants
Actors support four variants. The normal stick actor is the default. Set `type` to `hollow` or `awesome` for the other built-in variants. Set `icon` to an icon name for an icon actor.
```mermaid
usecase-beta
actor Normal("Normal actor")
actor Hollow("Hollow actor")@{ type: hollow }
actor Awesome("Awesome actor")@{ type: awesome }
actor Icon("Registered icon")@{ icon: "fa:user" }
actor Fallback("Missing icon fallback")@{ icon: "not-registered:user" }
Normal --> Manage
Hollow --> Manage
Awesome --> Manage
Icon --> Manage
Fallback --> Manage
```

### Business elements and stereotypes
Set `business: true` to add the conventional business slash to a normal or hollow actor or to an ellipse use case. Rectangular use cases, awesome actors, and icon actors cannot be business elements.
```mermaid
usecase-beta
actor SalesAgent("Sales agent")@{ business: true } <<Employee>>
actor Broker@{ type: hollow, business: true }
Quote("Prepare quote")@{ business: true } <<Core>>
Archive[Archive quote] <<Record>>
SalesAgent --> Quote
Broker --> Quote
Quote --> Archive
```

## Labels
Labels can be unquoted text inside parentheses or brackets, a single-line plain string, or a Mermaid Markdown string. The same forms apply to actor labels, use case labels, boundary titles, relationship labels, and notes.
```mermaid
usecase-beta
actor Analyst(*Analyst*)
Literal(**Literal markers**)
Priced[Costs 19.95 EUR — VAT included!]
Analyst -- files R&D report --> Literal
Analyst --> Priced
```
```mermaid
usecase-beta
actor Reviewer("`*Reviewer*`")
Literal("**Literal markers**")
Formatted("`**Formatted label**
with a physical line break`")
Quoted("Show #quot;quoted#quot; text")
Reviewer -- "`opens **form**`" --> Formatted
Reviewer --> Literal
Reviewer --> Quoted
```

## Lines and comments
Each statement occupies one physical line unless it contains a Markdown string, multiline metadata block, JSON object, accessibility description block, or system boundary block. The final statement does not need a trailing newline.
```mermaid
usecase-beta
%% Actors are declared explicitly.
actor User

%% Each relationship is a separate statement.
User --> Login
Login --> Dashboard
```

## System boundaries
A `systemBoundary` block groups actor and use case declarations. Give the boundary a short identifier followed by an optional display title, then optional metadata and classes. The suffixes are ordered `systemBoundary <id><title>@{ … }:::<classes>`, the same order actors and use cases use.
```mermaid
usecase-beta
systemBoundary sb1["Payment service"]@{ type: package }:::system
  actor Clerk("Payment clerk")
  Authorize("Authorize payment")
  Receipt[Create receipt]
end
Clerk --> Authorize
Authorize --> Receipt
classDef system stroke:#4b4b7a
```
```mermaid
usecase-beta
systemBoundary "Payment service"
  actor Clerk("Payment clerk")
  Authorize("Authorize payment")
end
Payment_service@{ type: package }
Clerk --> Authorize
```

## Relationships

### Associations
Seven solid association operators are supported. The marker direction and type are preserved.
```mermaid
usecase-beta
actor User
actor Support
Start
Finish
User --> Start
Start <-- Support
Start -- Finish
User --o Start
Start o-- Support
User --x Finish
Finish x-- Support
```
```mermaid
usecase-beta
actor User
Login
User -- "include account details" --> Login
```

### Include, extend, and generalization
Use explicit operators for UML relationship semantics.
```mermaid
usecase-beta
actor Admin
actor Person
Checkout
Payment
ApplyCoupon
Admin --|> Person
Checkout ..> : include Payment
ApplyCoupon ..> : extend Checkout
ApplyCoupon --|> Checkout
```

### Edge IDs, styles, animation, and length
Place an explicit edge ID followed by `@` immediately before an operator. Explicit edge IDs share the diagram-wide ID namespace and can be targeted by `class`, `style`, and metadata statements.
```mermaid
usecase-beta
actor Customer
Checkout
Payment
Customer opens@-- "starts checkout" ---> Checkout
Checkout payment@..> : include Payment
classDef emphasized stroke:#d33,stroke-width:3px
class opens,payment emphasized
style opens stroke:#06c,stroke-width:4px
opens@{ animation: fast }
payment@{ animate: false }
```

## Notes
A note attaches to one actor or use case. The target can be declared after the note. A note uses the shared note shape, a plain rectangle in the theme note colors, and is connected to its target by a dotted, markerless line.
```mermaid
usecase-beta
note for Login "`Requires an **active session**`"
note for User "Starts the workflow"
actor User
Login("Sign in")
User --> Login
```

## JSON tables
A top-level JSON declaration creates a table whose title is its identifier. Its body must be a strict JSON object.
```mermaid
usecase-beta
Inspect("Inspect payload")
json Payload@{
  "2": "second in source",
  "1": "first after 2",
  "enabled": true,
  "count": 3,
  "missing": null,
  "colors": ["Red", "Green"],
  "address": { "city": "Oslo" },
  "items": [{ "name": "Book" }],
  "emptyObject": {},
  "emptyArray": []
}:::data
Inspect --> Payload
classDef data stroke:#3572a5
style Payload stroke-width:2px
```

## Styling
Actors, use cases, boundaries, JSON nodes, and explicit edges support Mermaid classes and direct styles. Use `classDef` to define one or more classes, `class` to assign them, `style` for direct declarations, or `:::` on an actor, use case, boundary, or JSON declaration.
```mermaid
usecase-beta
actor Customer:::external
Checkout("Checkout"):::critical
systemBoundary Account
  Profile[Edit profile]
end
json Session@{ "active": true }:::data
Customer --> Checkout
Checkout --> Profile
Profile --> Session
classDef default stroke:#7f8ea3
classDef external,critical stroke-width:3px
class Customer,Checkout external,critical
class Account,Session data
style Checkout stroke:#c33,stroke-width:4px
style Account stroke:#536878
```

## Colors
Colour is keyed to the kind of element, not to the order elements are declared. Actors take
| Theme variable          | Applies to                                |
| ----------------------- | ----------------------------------------- |
| `usecaseActorBkg`       | Actor glyph fill                          |
| `usecaseActorBorder`    | Actor glyph stroke                        |
| `usecaseBkg`            | Use case body fill                        |
| `usecaseBorder`         | Use case body stroke                      |
| `usecaseBoundaryBkg`    | System boundary fill, when not numbered   |
| `usecaseBoundaryBorder` | System boundary stroke, when not numbered |
| `usecaseIncludeLine`    | `include` relationship stroke             |
| `usecaseExtendLine`     | `extend` relationship stroke              |
```mermaid
---
config:
  theme: redux-color
---
usecase-beta
direction LR
actor Customer
systemBoundary Catalogue
  Browse("Browse catalogue")
end
systemBoundary Payment
  Checkout("Checkout")
end
Customer --> Browse
Browse --> Checkout
Checkout ..> : include Browse
```

### Per-element colour rotation
Set `colorScheme: rotate` to extend the numbering from the boundaries to the actors and use
```mermaid
---
config:
  theme: redux-color
  usecase:
    colorScheme: rotate
---
usecase-beta
direction LR
actor Customer
actor Auditor
Browse("Browse catalogue")
Checkout("Checkout")
Customer --> Browse
Browse --> Checkout
Auditor --> Checkout
```

## Configuration
Use case diagrams accept these diagram configuration keys:
| Key                 | Default                   | Purpose                                       |
| ------------------- | ------------------------- | --------------------------------------------- |
| `actorFontSize`     | `14`                      | Actor label font size                         |
| `actorFontFamily`   | `"Open Sans", sans-serif` | Actor label font family                       |
| `actorFontWeight`   | `normal`                  | Actor label font weight                       |
| `usecaseFontSize`   | `12`                      | Use case label font size                      |
| `usecaseFontFamily` | `"Open Sans", sans-serif` | Use case label font family                    |
| `usecaseFontWeight` | `normal`                  | Use case label font weight                    |
| `nodeSpacing`       | `50`                      | Spacing between nodes on the same level       |
| `rankSpacing`       | `50`                      | Spacing between layout ranks                  |
| `diagramPadding`    | `20`                      | Padding around the diagram                    |
| `colorScheme`       | `role`                    | How the diagram takes colour from the theme   |
| `useMaxWidth`       | `true`                    | Whether the SVG scales to the available width |
```mermaid
---
config:
  usecase:
    actorFontSize: 16
    actorFontFamily: Arial
    actorFontWeight: bold
    usecaseFontSize: 14
    usecaseFontFamily: Georgia
    usecaseFontWeight: normal
    nodeSpacing: 60
    rankSpacing: 70
    diagramPadding: 24
    colorScheme: role
    useMaxWidth: false
---
usecase-beta
direction LR
actor Customer
Browse("Browse catalog")
Customer --> Browse
```

## Accessibility
Use `accTitle` and `accDescr` to give the diagram an accessible title and description. `accDescr` accepts a single-line value after `:` or a multiline block.
```mermaid
usecase-beta
accTitle: Account access use cases
accDescr {
  A customer signs in and can reset a password.
  The diagram names the actor, use cases, and associations.
}
actor Customer
SignIn("Sign in")
Reset("Reset password")
Customer --> SignIn
Customer --> Reset
```

## Complete example
Features can be combined in one diagram. This example uses accessibility metadata, actor variants, a package boundary, stereotypes, classes, a JSON table, a note, a labelled long association, an include relationship, an animated edge, and a directly styled edge.
```mermaid
usecase-beta
direction LR
accTitle: Online ordering use cases
accDescr {
  A customer places an order through the storefront.
  Staff review the order and inspect its data.
}
actor Customer("Customer")
actor Staff("Order staff")@{ type: hollow, business: true } <<Employee>>
systemBoundary ordering["Ordering System"]@{ type: package }:::system
  Browse("Browse products")
  Checkout("Checkout") <<Core>>:::critical
  Payment("Process payment")
  Review[Review order]
end
json OrderData@{
  "status": "pending",
  "items": [{ "name": "Book", "quantity": 1 }],
  "total": 29.95
}:::data
note for Checkout "`Validates the **cart** before payment`"
Customer starts@-- "places order" ---> Checkout
Customer --> Browse
Checkout pays@..> : include Payment
Staff --> Review
Review --> OrderData
classDef system stroke:#c8a02a,stroke-width:2px
classDef critical stroke:#c33,stroke-width:3px
classDef data stroke:#3572a5
starts@{ animation: fast }
style pays stroke:#6b46c1,stroke-width:2px
```

## Migrating from PlantUML
A PlantUML use case diagram needs these changes before it renders as Mermaid:
- Put one statement on each physical line. A second statement on the same line is a parse error.
- Undeclared relationship endpoints become ellipse use cases. Declare every actor with `actor ID` somewhere in the document because source position does not imply an actor.
- An association label containing `include` or `extend` remains an association. Use `..> : include` or `..> : extend` for UML include and extend semantics.
- Replace actor `fillColor`, `strokeColor`, `strokeWidth`, and other arbitrary metadata with `classDef`, `class`, or `style`.
- Replace these constructs, which are outside the grammar and raise a parse error: separators and titled separators, `as` aliases, colon actors, standalone `package` and `rectangle` blocks, `skinparam`, `<style>` blocks, `allowmixing`, left, right, up, or down link hints, note placement keywords, standalone notes, multi-target notes, and `newpage`. Style the diagram with `classDef`, `class`, `style`, and `:::` instead, as described under [Styling](#styling).
- Rewrite backslash escapes and plain `\n` multiline text. They parse, so no error points at them. See [Constructs that parse but mean something else](#constructs-that-parse-but-mean-something-else).

### Constructs that parse but mean something else
Unsupported syntax raises a parse error, with two exceptions. These parse without an error and render differently from PlantUML:
- `A("Line1\nLine2")` keeps the backslash and the `n` as literal text on a single line. Use a Markdown string containing a physical newline for a real line break.
- `A("Paren \( x")` keeps the backslash as literal text, because a backslash never escapes the character after it. Write the character directly inside a quoted or Markdown string, or use an entity code such as `#40;` and `#41;`.
```mermaid
usecase-beta
actor Customer
Reset("`Reset
password`")
Refund("Refund #40;partial#41;")
Customer --> Reset
Customer --> Refund
```
