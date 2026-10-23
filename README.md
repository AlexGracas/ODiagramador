# ODiagramador

**PT-BR** · Visualizador de diagramas **Mermaid** feito para trabalhar junto com agentes de IA — offline, nativo em macOS, Windows e Linux, e preparado para UML.
**EN** · An AI-friendly **Mermaid** diagram viewer — offline, native on macOS, Windows and Linux, ready for UML.

- 🤖 **Servidor MCP** (`odg mcp`): o agente cria/edita diagramas e recebe na mesma resposta o relatório de problemas **e uma imagem** com marcadores numerados nos problemas (ou o código com a linha do erro destacada).
- 👀 **App desktop nativo**: o humano vê as mudanças do agente ao vivo, navega pelos problemas e deixa **comentários que o agente lê** (`get_feedback`).
- 💬 **Conversa agente ⇄ usuário**: o agente abre um diagrama no app (`show_diagram`), **destaca elementos** com uma nota (`highlight_elements`), manda avisos (`notify_user`) e **faz perguntas com botões de resposta** (`ask_user`) — a resposta volta para o agente.
- 🧪 **CLI** (`odg check --json`, `odg render --annotated`) para CI e agentes sem MCP.
- 🌐 Interface e mensagens em **português e inglês**; códigos de problema estáveis em inglês.
- 📁 Diagramas são arquivos `.mmd` comuns no seu projeto (versionáveis no git).
- ⚙️ **Configurações**: idioma, modo claro/escuro, fontes do editor e do diagrama, **MCP remoto** por uma porta (com token) e **atualização automática** a partir dos releases do GitHub.
- 🧩 **Construtores**: ao criar um diagrama você escolhe o tipo (fluxograma, sequência, classes, estados, ER) e a prévia ganha uma paleta com os elementos daquele tipo (nó, decisão, ligação, grupo, participante, mensagem, bloco alt/loop, classe, relação, estado, transição, entidade…) e a **direção** (TB/BT/LR/RL). O texto Mermaid é gerado pelo núcleo e aparece no editor. **Clicar num elemento** abre o inspetor para renomear, trocar a forma ou excluir (as ligações do elemento saem junto; os vizinhos ficam); clicar numa ligação permite editar o texto ou excluí-la.
- ✋ **Correção de layout** em fluxogramas, estados, classes, ER e requisitos: ELK automático, **arrastar nós** (ELK interativo, o arraste vira dica) ou **posições exatas**. As posições ficam em comentários `%% @odg`, e o arquivo continua Mermaid válido em qualquer outro lugar ([docs/syntax/layout.md](docs/syntax/layout.md)).

## Download

Baixe a versão mais recente em **[Releases](https://github.com/AlexGracas/ODiagramador/releases/latest)**:
- macOS 13+ (Apple Silicon e Intel): `ODiagramador-…-macos-universal.zip`
- Windows 10/11 x64: `ODiagramador-…-windows-x64.zip`

As instruções de instalação estão nas notas do release (os apps ainda não são assinados com certificado de desenvolvedor).

## Arquitetura / Architecture

```
render-kit/ (HTML+JS; bundle offline Mermaid 11.17.2 + ELK + layouts próprios) ← TODAS as WebViews e o headless
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

Requisitos: Rust estável; macOS 13+ (Xcode CLT), Windows 10/11 (.NET 10 SDK, WebView2 runtime) ou Linux (GTK4, libadwaita, WebKitGTK 6, GtkSourceView 5; e WebKitGTK 4.1 para o `odg`).

```bash
cargo test --workspace
cargo build --release -p odg-cli -p odg-ffi
```

```bash
./target/release/odg check examples/diagrams --lang pt-BR
```

O bundle do render-kit (`render-kit/vendor/mermaid-odg.min.js`) já vem no repositório. Para regerá-lo depois de mudar `render-kit/src/` ou a versão do Mermaid/ELK (precisa de npm, só na hora do build):

```bash
scripts/build-render-kit.sh
```

Apps:

```bash
apps/macos/build-app.sh            # → dist/ODiagramador.app (UNIVERSAL=1 ZIP=1 para o pacote de release)
```

```bash
cargo build --release --manifest-path apps/linux/Cargo.toml
```

```bash
dotnet publish apps/windows/ODiagramador/ODiagramador.csproj -c Release -o dist/windows
```

Configurar no Claude Code (no app, o botão **MCP** copia a configuração pronta para Claude Code, Codex, Cursor e Claude Desktop):

```bash
claude mcp add odiagramador -- /caminho/para/odg mcp --workspace /caminho/do/projeto
```

Acesso remoto (HTTP) — ou ative em **Configurações → MCP → Acesso remoto**:

```bash
ODG_MCP_TOKEN=um-token-longo odg mcp --workspace /caminho/do/projeto --http 0.0.0.0:7410
```

Guia completo para agentes: [docs/AGENTS.md](docs/AGENTS.md) · Roteiro UML: [docs/UML.md](docs/UML.md)

## Status

| Parte | Estado |
|---|---|
| Core, lint, i18n, FFI | testado (`cargo test`) |
| Render headless + CLI + MCP | testado no macOS (ponta a ponta via stdio) |
| App macOS | testado manualmente (edição, prévia, problemas, comentários, atualização ao vivo vinda do agente) |
| App Linux | compila (type-check); execução a validar em Linux |
| App Windows | compila; inicialização testada em CI (release); uso real a validar em Windows |
| Layout (ELK, arrastar, posições exatas) | protótipo testado no macOS e via MCP; roteamento das arestas no modo exato/arrastado é simples (não desvia de nós) |
| UML | pontos de extensão prontos; engine planejada em [docs/UML.md](docs/UML.md) |

Licença / License: [MIT](LICENSE).

Componentes de terceiros: Mermaid © its contributors (MIT) — `render-kit/vendor/mermaid.LICENSE` · ELK/elkjs © Kiel University and others (EPL-2.0) — `render-kit/vendor/elkjs.LICENSE`.
