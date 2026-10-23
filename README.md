# ODiagramador

**PT-BR** · Visualizador de diagramas **Mermaid** feito para trabalhar junto com agentes de IA — offline, nativo em macOS, Windows e Linux, e preparado para UML.
**EN** · An AI-friendly **Mermaid** diagram viewer — offline, native on macOS, Windows and Linux, ready for UML.

- 🤖 **Servidor MCP** (`odg mcp`): o agente cria/edita diagramas e recebe na mesma resposta o relatório de problemas **e uma imagem** com marcadores numerados nos problemas (ou o código com a linha do erro destacada).
- 👀 **App desktop nativo**: o humano vê as mudanças do agente ao vivo, navega pelos problemas e deixa **comentários que o agente lê** (`get_feedback`).
- 🧪 **CLI** (`odg check --json`, `odg render --annotated`) para CI e agentes sem MCP.
- 🌐 Interface e mensagens em **português e inglês**; códigos de problema estáveis em inglês.
- 📁 Diagramas são arquivos `.mmd` comuns no seu projeto (versionáveis no git).

## Arquitetura / Architecture

```
render-kit/ (HTML+JS, Mermaid 11.17.2 embutido)  ← usado por TODAS as WebViews e pelo renderizador headless
crates/odg-core    Rust: modelo neutro, lint, relatório, workspace, i18n (Fluent), cheat-sheets
crates/odg-render  Rust: WebView oculta (tao + wry) → render headless para CLI/MCP
crates/odg-cli     Rust: binário `odg` (mcp, check, render, watch, mcp-config)
crates/odg-ffi     Rust: C ABI (JSON in/out) consumida pelos apps
apps/macos         SwiftUI + WKWebView           (core via biblioteca estática)
apps/windows       C# WPF + WebView2 + AvalonEdit (core via odg_ffi.dll)
apps/linux         Rust GTK4 + libadwaita + WebKitGTK 6 + GtkSourceView (core direto)
```

Mermaid precisa de um motor de layout de navegador, então cada app usa a **WebView do próprio sistema** (sem Electron, sem Chromium embutido, sem Java). O app macOS completo, com o servidor MCP dentro, tem ~15 MB.

Agentes e humanos se comunicam **pelo workspace**: `.odiagramador/reports`, `previews`, `events.jsonl` e `feedback.json`.

## Começando / Getting started

Requisitos: Rust estável; macOS 13+ (Xcode CLT), Windows 10/11 (.NET 8, WebView2 runtime) ou Linux (GTK4, libadwaita, WebKitGTK 6, GtkSourceView 5; e WebKitGTK 4.1 para o `odg`).

```bash
cargo test --workspace
cargo build --release -p odg-cli -p odg-ffi
```

```bash
./target/release/odg check examples/diagrams --lang pt-BR
```

Apps:

```bash
apps/macos/build-app.sh            # → dist/ODiagramador.app
```

```bash
cargo build --release --manifest-path apps/linux/Cargo.toml
```

```bash
dotnet publish apps/windows/ODiagramador/ODiagramador.csproj -c Release -o dist/windows
```

Configurar no Claude Code (o app tem o botão **Copiar config MCP**):

```bash
claude mcp add odiagramador -- /caminho/para/odg mcp --workspace /caminho/do/projeto
```

Guia completo para agentes: [docs/AGENTS.md](docs/AGENTS.md) · Roteiro UML: [docs/UML.md](docs/UML.md)

## Status

| Parte | Estado |
|---|---|
| Core, lint, i18n, FFI | testado (`cargo test`) |
| Render headless + CLI + MCP | testado no macOS (ponta a ponta via stdio) |
| App macOS | testado manualmente (edição, prévia, problemas, comentários, atualização ao vivo vinda do agente) |
| App Linux | compila (type-check); execução a validar em Linux |
| App Windows | compila; execução a validar em Windows |
| UML | pontos de extensão prontos; engine planejada em [docs/UML.md](docs/UML.md) |

Mermaid © its contributors (MIT) — `render-kit/vendor/mermaid.LICENSE`.
