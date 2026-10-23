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

O servidor MCP vem dentro do app (`odg`). No app, o botão **MCP** copia a configuração para Claude Code, Codex, Cursor ou Claude Desktop — ou, no Claude Code:

```bash
claude mcp add odiagramador -- /Applications/ODiagramador.app/Contents/MacOS/odg mcp --workspace /caminho/do/projeto
```

No Windows o caminho é `...\ODiagramador\odg.exe`. Guia completo: [docs/AGENTS.md](https://github.com/AlexGracas/ODiagramador/blob/main/docs/AGENTS.md).

## Novidades da 0.4.0
- **O agente controla a sua visualização** (novas ferramentas MCP):
  - `open_diagram` troca o diagrama aberto no app;
  - `set_zoom` muda o zoom (`1.5`, `"150%"`, `fit`, `in`, `out`);
  - `center_on_element` coloca um elemento no meio da tela (ele pisca rapidamente para você achar).
- `get_viewer_status` agora informa também o zoom, o elemento no centro da tela e os elementos visíveis.
- Para mandar mensagens: `notify_user` (aviso) e `ask_user` (pergunta com botões de resposta).

> Se o seu agente não mostra essas ferramentas, atualize a configuração do MCP (botão **MCP** do app) para usar o `odg` desta versão e reinicie o agente.

## Novidades da 0.3.2
- **Windows: atualização automática corrigida.** Antes, clicar em *Atualizar agora* reabria a mesma versão (o script auxiliar falhava sem avisar). Agora o próprio app troca os arquivos, confere a versão instalada, desfaz tudo se algo der errado e registra cada passo em `%LocalAppData%\ODiagramador\odiagramador.log`.

> Quem está na 0.2.x ou 0.3.0/0.3.1 no Windows: baixe esta versão manualmente uma vez (descompacte por cima da pasta atual). A partir dela, as próximas atualizações são instaladas pelo app.

## Novidades da 0.3.1
- **Tela de atualização** mostra as novidades formatadas (títulos, listas, **negrito**, `código`, links) em vez do texto Markdown cru, e só o que mudou desde a sua versão.
- Observação: a tela de atualização é desenhada pela versão instalada; a formatação aparece a partir da próxima atualização depois desta.

## Novidades da 0.3.0
- **O agente conversa com você pelo app** (novas ferramentas MCP):
  - `show_diagram` abre um diagrama no app (e abre o app, se estiver fechado);
  - `highlight_elements` coloca elementos em evidência: eles brilham, o resto esmaece e a vista centraliza neles, com uma nota do agente;
  - `notify_user` mostra avisos; `ask_user` faz perguntas com botões de resposta ou texto, e a resposta volta para o agente;
  - `get_user_replies` e `get_viewer_status` (o app está aberto? qual diagrama você está vendo?).
  - As ferramentas que alteram diagramas aceitam `show: true` para já mostrar o resultado.
- **Botões com efeito ao passar o mouse** (barra do Windows e botões da prévia em todas as plataformas).
- Windows: o ícone do MCP acompanha o tema escuro; abrir o app pelo agente não perde mais o diagrama (o `&` do endereço era cortado).

## Novidades da 0.2.2
- **Ícone novo** do aplicativo (fluxograma sobre um fundo azul e roxo) no macOS, no Windows e no Linux.

## Novidades da 0.2.1
- **Copiar configuração do MCP**: agora sempre aparece uma janela com o resultado, o texto copiado e onde colar em cada programa (Claude Code, Codex, Cursor, Claude Desktop). Antes a confirmação ficava escondida atrás da janela de Configurações.
- **Windows: log de diagnóstico** em `%LocalAppData%\ODiagramador\odiagramador.log` (Configurações → Geral → *Abrir pasta de logs*), registrando cliques nos menus, cópia para a área de transferência, MCP remoto, atualizações e erros.

## Novidades da 0.2.0
- **Barra de ferramentas com ícones**: abrir pasta, novo diagrama, **Exportar** (PNG ou SVG) e **MCP** com o ícone de cada programa (Claude Code, Codex, Cursor, Claude Desktop).
- **Configurações** (⌘, no macOS; engrenagem no Windows/Linux): idioma, aparência clara/escura/do sistema, fonte do editor, fonte do diagrama (vale também para os agentes).
- **MCP remoto**: exponha o servidor MCP numa porta (só este computador ou a rede local, sempre com token) e copie a configuração remota para cada cliente. Na linha de comando: `odg mcp --http 0.0.0.0:7410 --token …`.
- **Atualização automática**: o app verifica os releases do GitHub (uma vez por dia, ou em *Verificar atualizações…*) e pode baixar, substituir e reiniciar sozinho (macOS e Windows; no Linux abre a página do release).
- **Windows**: corrigido o *Copiar config MCP* (a área de transferência agora é tentada de novo quando outro app a bloqueia, com aviso de confirmação e, em último caso, uma janela com o texto para copiar). Visual Fluent com modo escuro (.NET 10).

> Atualizando da 0.1.x: baixe esta versão manualmente uma vez; a partir dela, as próximas chegam pelo próprio app.

## Funcionalidades
- Prévia ao vivo, painel de problemas com linha/coluna e dicas, comentários do humano para o agente, PT/EN.
- Construtores por tipo de diagrama (fluxograma, sequência, classes, estados, ER) e seletor de direção.
- Inspetor: renomear, trocar forma e excluir elementos e ligações.
- Layout: ELK, ELK interativo (arrastar) e posições exatas, guardadas em comentários `%% @odg`.
- MCP: create/update/patch/validate/render/inspect, `add_element`, `edit_element`, `arrange_diagram`, feedback e atividade.

## Situação conhecida
- **macOS**: testado.
- **Windows**: compilado e testado na inicialização em CI; ainda não validado manualmente em uma máquina Windows. Relatos de problemas são bem-vindos.
- Linux: compila em CI; sem pacote nesta versão.
