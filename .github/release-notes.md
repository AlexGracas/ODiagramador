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

## Novidades da 0.8.0
- **Organização por Projetos**: suporte à sintaxe oficial de frontmatter Mermaid (`project: ...`) em diagramas `.mmd`. A barra lateral agora agrupa diagramas por projeto.
- **Criação com Projeto Obrigatório**: novos diagramas agora exigem atribuir um projeto (no app e via MCP `create_diagram`).
- **Propriedades do Diagrama**: novo diálogo acessível no cabeçalho para visualizar e editar Projeto, Título e Descrição do diagrama ativo.
- **Data da Última Edição**: a data de modificação (`DD/MM`) agora aparece no canto esquerdo de cada linha na barra lateral, junto ao ícone de status.
- **MCP (Model Context Protocol)**:
  - Recursos `projects://list` e `projects://current` para listagem e descoberta do projeto ativo.
  - Template `project://{name}` para diagramas de um projeto específico.
  - Nova ferramenta `set_diagram_properties` para atualizar metadados sem alterar o corpo do diagrama.
  - Nova ferramenta `list_projects` para agentes de IA inspecionarem projetos e diagramas associados.

## Novidades da 0.7.1
- **Diagramas de classe (UML 2.5)**: novos construtores — classe abstrata, interface, enumeração e pacote — e um botão com o **símbolo UML** de cada relação: associação, associação direcionada, generalização, realização, dependência, agregação, composição e interface fornecida (pirulito). Cada formulário diz o papel de cada classe (pai/filha, todo/parte, cliente/fornecedor) e explica a notação.
- **Setas de classe encostam na classe**: triângulos e losangos não ficam mais soltos a ~9px da caixa (defeito do visual "neo" do Mermaid 12).
- **Menu do botão direito numa classe**: "Adicionar atributo…" e "Adicionar operação…" (aceita `total(): Money`, visibilidade + - # ~, `$` estático e `*` abstrato).
- **Lista de diagramas**: mostra o **tipo** de cada diagrama, agrupa por tipo ou pasta e ordena por nome ou data; pode ser **recolhida** numa faixa "DIAGRAMAS".
- **Painéis recolhidos deixam uma faixa com nome** ("› CÓDIGO", "PROBLEMAS · COMENTÁRIOS · ATIVIDADE") — clique para trazê-los de volta.
- **Renderizadores**: botão "?" na barra do diagrama abre uma explicação didática — o que é um renderizador, qual usar em cada situação, como trocar e uma comparação entre eles.
- MCP: `list_diagrams` informa o tipo; `edit_element` ganha `style` (cores) e `member` (atributo/operação de classe).

## Novidades da 0.7.0
- **Edição pelo mouse mais natural**: clique seleciona, arrastar move (em todos os layouts de grafo), **clique duplo renomeia** e o **botão direito abre um menu de contexto** — renomear/editar texto, forma, **cor** (preenchimento e borda), ligar a outro elemento, excluir e propriedades.
- **Excluir apaga na hora** (sem "clique de novo"); use Desfazer se precisar.
- **Modos Editar / Visualizar** com ícones, e **Desfazer / Refazer** (⌘Z / Ctrl+Z, ⇧⌘Z / Ctrl+Y) — tudo na barra superior junto com os botões de criação, separados por contexto.
- Setas mais fáceis de selecionar (área de clique maior, independente do zoom); clicar fora do painel de edição cancela.
- **Painéis colapsáveis**: clique duplo na barra entre código e diagrama, ou na barra do painel inferior, recolhe/restaura.
- Cada aba inferior (Problemas, Comentários para agentes, Atividade) explica para que serve; passar o mouse sobre um marcador numerado mostra o problema.
- **Comentários para agentes com contexto**: o comentário leva o diagrama aberto e o elemento selecionado (linha e ligações) — você vê e pode editar o contexto antes de enviar. Nova ferramenta MCP **`wait_for_feedback`** (e `odg wait-feedback`) para o agente agir assim que você comentar.
- MCP/núcleo: nova edição `style` (cores) para fluxogramas e diagramas de classe.

## Novidades da 0.6.1
- **`select_element`** (nova ferramenta MCP): o agente seleciona um elemento como se você tivesse clicado nele (contorno, inspetor, linha do código); `get_viewer_status` passa a informá-lo como selecionado.
- **Windows: atualização** — a conferência final agora compara os arquivos instalados com o pacote (antes ela lia a versão da cópia já carregada e acusava falha numa atualização que tinha funcionado).
- **Windows: log** registra o que cada pedido do agente fez (diagrama aberto, zoom, elemento, centro) e os diagramas/elementos que você escolhe.
- Windows: o arquivo de presença do visualizador não falha mais quando outro programa (ex.: OneDrive) o mantém aberto.

> Atualizando da 0.6.0 no Windows: a conferência antiga ainda pode mostrar um erro no fim; os arquivos são instalados mesmo assim — feche e abra o app.

## Novidades da 0.6.0
- **Validação de diagramas de arquitetura**: diagramas de blocos (`block`) agora são analisados (blocos e setas), e há novos avisos para todos os diagramas com setas: seta passando por cima de outro elemento, seta curta demais para a ponta (aparece invertida), rótulo por cima de caixas ou de outro rótulo, e **cruzamentos de setas** (com a contagem no relatório, para comparar versões).
- **Ordem das camadas no ELK**: `arrange_diagram` aceita camadas numeradas (`layers: [{id, layer: 0}, …]`, também para subgrafos), mantendo a ordem mesmo com setas que voltam.
- Setas ligadas a um subgrafo contam como conexão dos nós de dentro (sem falso "não conectado").
- `render_diagram` com `focus` (elemento ou subgrafo) ou `region` mostra só aquela parte, legível, em diagramas grandes.
- **Uma janela só**: quando o agente de outro projeto pede para mostrar um diagrama, a janela aberta troca para aquele projeto em vez de abrir outra (Windows e Linux).
- **macOS**: o app e o `odg` voltam a rodar como Apple Silicon quando iniciados por um programa Intel (sem o aviso de incompatibilidade com versões futuras do macOS).
- Windows: "Novo diagrama" usa o exemplo certo de cada tipo.

## Novidades da 0.5.0
- **Todos os tipos de diagrama do Mermaid** (33): o motor passou para o **Mermaid 12.2** com o plugin **ZenUML**. Além dos de antes: casos de uso, requisitos, C4, arquitetura, blocos, raias, ZenUML, Gantt, linha do tempo, jornada, Kanban, Git, mapa mental, árvore, Ishikawa, event modeling, fluxo de agentes, railroad, pacote, pizza, XY, quadrantes, radar, Sankey, treemap, Venn, Wardley e Cynefin.
- **Novo diagrama** lista todos os tipos, cada um começando pelo exemplo oficial.
- **ELK só onde se aplica**: fluxogramas e diagramas de estados, classes, ER e requisitos continuam com ELK, arrastar e posições exatas; os demais tipos usam o layout do próprio Mermaid (o painel de problemas avisa se houver `%% @odg layout` neles).
- Agentes: `get_syntax_help` tem a sintaxe de todos os tipos, offline (resumo da documentação oficial do Mermaid).
- macOS/Linux: diagramas Event Modeling voltaram a renderizar, e diagramas com textos HTML agora geram PNG (os textos aparecem como texto simples).

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
