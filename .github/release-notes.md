Visualizador de diagramas **Mermaid** feito para trabalhar com agentes de IA — offline, nativo, com servidor MCP embutido.
AI-friendly **Mermaid** diagram viewer — offline, native, with a built-in MCP server.

## Downloads

| Plataforma | Arquivo |
|---|---|
| macOS 13+ (Apple Silicon e Intel) | `ODiagramador-…-macos-universal.zip` |
| Windows 10/11 (x64) | `ODiagramador-…-windows-x64.zip` |

### macOS
1. Descompacte e arraste **ODiagramador.app** para *Aplicativos*.
2. O app ainda não é assinado com Developer ID: na primeira vez, clique com o botão direito → **Abrir** → **Abrir**
   (ou, no Terminal: `xattr -dr com.apple.quarantine /Applications/ODiagramador.app`).

### Windows
1. Descompacte a pasta `ODiagramador` onde preferir e execute **ODiagramador.exe** (não precisa instalar o .NET).
2. O executável ainda não é assinado: se o SmartScreen avisar, clique em **Mais informações → Executar assim mesmo**.
3. Requer o runtime WebView2 (já presente no Windows 10/11 atualizado).

## Usar com agentes de IA (MCP)

O servidor MCP vem dentro do app (`odg`). No app, use **Copiar config MCP** — ou, no Claude Code:

```bash
claude mcp add odiagramador -- /Applications/ODiagramador.app/Contents/MacOS/odg mcp --workspace /caminho/do/projeto
```

No Windows o caminho é `...\ODiagramador\odg.exe`. Guia completo: [docs/AGENTS.md](https://github.com/AlexGracas/ODiagramador/blob/main/docs/AGENTS.md).

## Nesta versão
- Prévia ao vivo, painel de problemas com linha/coluna e dicas, comentários do humano para o agente, PT/EN.
- Construtores por tipo de diagrama (fluxograma, sequência, classes, estados, ER) e seletor de direção.
- Inspetor: renomear, trocar forma e excluir elementos e ligações.
- Layout: ELK, ELK interativo (arrastar) e posições exatas, guardadas em comentários `%% @odg`.
- MCP: create/update/patch/validate/render/inspect, `add_element`, `edit_element`, `arrange_diagram`, feedback e atividade.

## Situação conhecida
- **macOS**: testado.
- **Windows**: compilado e testado na inicialização em CI; ainda não validado manualmente em uma máquina Windows. Relatos de problemas são bem-vindos.
- Linux: compila em CI; sem pacote nesta versão.
