# ODiagramador — textos em português (Brasil).
# Compartilhado pelo servidor MCP, pela CLI e pelos três apps desktop (via odg-ffi).
# Os códigos de problema (E_*, W_*, I_*) são estáveis; só o texto é traduzido.

## Problemas

problem-E_EMPTY = O código do diagrama está vazio.
hint-E_EMPTY = Comece com o cabeçalho do tipo de diagrama, por exemplo `flowchart TD`, `sequenceDiagram` ou `classDiagram`.

problem-E_UNKNOWN_TYPE = Tipo de diagrama não reconhecido.
hint-E_UNKNOWN_TYPE = A primeira linha (fora comentários) deve declarar o tipo, ex.: `flowchart LR`, `sequenceDiagram`, `classDiagram`, `stateDiagram-v2`, `erDiagram`, `gantt`, `pie`, `mindmap`. Use get_syntax_help para ver exemplos.

problem-E_PARSE = Erro de sintaxe na linha { $line }: { $detail }
problem-E_PARSE-noline = Erro de sintaxe: { $detail }
hint-E_PARSE = Verifique a linha { $line } e a linha anterior. { $expected }
hint-E_PARSE-noline = Valide o diagrama por partes; use get_syntax_help para ver a sintaxe correta.
detail-unexpected-token = símbolo inesperado '{ $token }'
detail-unexpected-eof = o diagrama termina cedo demais (falta algo no final)
detail-unexpected-eol = a linha termina cedo demais (falta algo no fim da linha)
hint-unclosed = A linha { $line } abre '{ $open }' e não fecha: adicione '{ $close }'.
hint-expected = Esperado um destes: { $tokens }.

problem-E_RENDER = O diagrama foi lido, mas não pôde ser desenhado: { $detail }
hint-E_RENDER = Simplifique a construção citada na mensagem ou divida o diagrama.

problem-E_BACKEND = O renderizador não está disponível: { $detail }
hint-E_BACKEND = Em servidores Linux rode com um display (ex.: `xvfb-run odg mcp`); no Windows instale o runtime WebView2.

problem-W_ORPHAN_NODE = "{ $label }" não está conectado a nada.
hint-W_ORPHAN_NODE = Ligue-o com uma aresta (ex.: `{ $id } --> Outro`) ou remova-o se sobrou.

problem-W_OVERLAP = "{ $a }" está sobreposto a "{ $b }".
hint-W_OVERLAP = Mude a direção (TB/LR), mova um dos nós para um subgraph ou encurte os textos.

problem-W_LABEL_OVERFLOW = O texto de "{ $label }" não cabe dentro da forma.
hint-W_LABEL_OVERFLOW = Encurte o texto ou quebre a linha com `<br/>`.

problem-W_LONG_LABEL = O texto de "{ $id }" é muito longo ({ $length } caracteres).
hint-W_LONG_LABEL = Mantenha textos com menos de { $max } caracteres; quebre linhas com `<br/>` ou mova detalhes para uma nota.

problem-W_DUPLICATE_LABEL = Elementos diferentes têm o mesmo texto "{ $label }": { $ids }.
hint-W_DUPLICATE_LABEL = Se forem o mesmo elemento, reutilize um único id; senão, diferencie os textos.

problem-W_TOO_LARGE = O diagrama é grande ({ $nodes } nós, { $edges } arestas).
hint-W_TOO_LARGE = Considere dividir em diagramas menores ou agrupar com subgraphs.

problem-W_WIDE = O diagrama está muito largo ({ $width }×{ $height } px).
hint-W_WIDE = Use a direção de cima para baixo (`flowchart TB`) ou divida cadeias longas.

problem-W_TALL = O diagrama está muito alto ({ $width }×{ $height } px).
hint-W_TALL = Use a direção da esquerda para a direita (`flowchart LR`) ou divida o diagrama.

problem-W_FOREIGN_OBJECT = Este tipo de diagrama usa textos HTML; na prévia PNG eles aparecem como texto simples.
hint-W_FOREIGN_OBJECT = A prévia SVG e o visualizador do app mostram exatamente.

problem-W_RASTER = Não foi possível gerar a prévia PNG: { $detail }
hint-W_RASTER = Use a prévia SVG.

problem-W_LAYOUT_MODE = Modo de layout desconhecido "{ $mode }".
hint-W_LAYOUT_MODE = Use um destes: { $modes }.

problem-W_LAYOUT_UNKNOWN_ID = O comentário de layout cita "{ $id }", que não existe no diagrama.
hint-W_LAYOUT_UNKNOWN_ID = Corrija o id ou remova a linha `%% @odg` (ids diferenciam maiúsculas e minúsculas).

problem-W_LAYOUT_IGNORED = As posições dos nós são ignoradas no modo de layout "{ $mode }".
hint-W_LAYOUT_IGNORED = Adicione `%% @odg layout elk-interactive` (posições como dica) ou `%% @odg layout exact` (posições exatas).

problem-W_LAYOUT_UNSUPPORTED = Comentários de layout não têm efeito em diagramas { $type }: o ELK só organiza diagramas de grafo, então é usado o layout do próprio Mermaid.
hint-W_LAYOUT_UNSUPPORTED = ELK, arrastar e posições exatas funcionam em fluxogramas e diagramas de estados, classes, ER e requisitos; remova as linhas `%% @odg` aqui.

## Resumos

summary-valid = Diagrama { $type } válido: { $nodes } nós, { $edges } arestas. { $problems }
summary-invalid = Diagrama inválido: { $errors } erro(s). Corrija o primeiro erro e valide novamente.
summary-problems = { $warnings ->
    [0] Nenhum problema visual encontrado.
    [one] 1 aviso.
   *[other] { $warnings } avisos.
}

## MCP

mcp-next-invalid = Próximo passo: corrija o primeiro erro (linha exata acima) e envie o arquivo de novo com update_diagram, ou uma correção pequena com patch_diagram.
mcp-next-warnings = Próximo passo: o diagrama renderiza. Veja a imagem (marcadores numerados como na lista) e corrija os avisos com patch_diagram, a menos que sejam intencionais.
mcp-next-clean = Próximo passo: nada a corrigir. Mostre a imagem ao usuário ou chame open_in_viewer.

## Interface desktop

ui-app-title = ODiagramador
ui-open-folder = Abrir pasta…
ui-new-diagram = Novo diagrama
ui-new-diagram-name = Nome do diagrama
ui-save = Salvar
ui-export-png = Exportar PNG…
ui-export-svg = Exportar SVG…
ui-diagrams = Diagramas
ui-no-workspace = Abra uma pasta para começar. Os diagramas são arquivos .mmd dentro dela.
ui-no-diagrams = Nenhum diagrama ainda. Crie um aqui ou peça ao seu agente de IA.
ui-empty = Nenhum diagrama
ui-fit = Ajustar
ui-problems = Problemas
ui-no-problems = Nenhum problema
ui-source = Código
ui-preview = Prévia
ui-feedback = Comentários para agentes
ui-add-feedback = Adicionar comentário
ui-feedback-placeholder = Diga ao agente o que mudar…
ui-resolved = Resolvido
ui-mark-resolved = Marcar como resolvido
ui-activity = Atividade
ui-changed-by-agent = Alterado por agente
ui-changed-by-human = Alterado por você
ui-language = Idioma
ui-theme = Tema
ui-theme-light = Claro
ui-theme-dark = Escuro
ui-copy-mcp-config = Copiar config MCP
ui-copied = Copiado
ui-line = Linha { $line }
ui-valid = Válido
ui-invalid = Inválido
ui-severity-error = Erro
ui-severity-warning = Aviso
ui-severity-info = Info
ui-layout = Layout
ui-layout-auto = Automático (Mermaid)
ui-layout-elk = ELK
ui-layout-interactive = ELK interativo (arrastar)
ui-layout-exact = Posições exatas (arrastar)
ui-reset-positions = Restaurar posições
ui-cancel = Cancelar
ui-create = Criar

## Palette (constructors)

ui-new-diagram-type = Tipo de diagrama
ui-type-flowchart = Fluxograma
ui-type-sequence = Sequência
ui-type-class = Classes (UML)
ui-type-state = Máquina de estados (UML)
ui-type-er = Entidade-relacionamento (ER)
ui-c-direction = Direção
ui-c-node = Nó
ui-c-decision = Decisão
ui-c-startend = Início/Fim
ui-c-edge = Ligação
ui-c-subgraph = Grupo
ui-c-participant = Participante
ui-c-actor = Ator
ui-c-message = Mensagem
ui-c-block = Bloco
ui-c-note = Nota
ui-c-autonumber = Numeração
ui-c-class = Classe
ui-c-relation = Relação
ui-c-state = Estado
ui-c-transition = Transição
ui-c-start = Início
ui-c-end = Fim
ui-c-choice = Escolha
ui-c-entity = Entidade
ui-f-label = Texto
ui-f-shape = Forma
ui-f-connect-from = Conectar a partir de
ui-f-edge-label = Texto da ligação
ui-f-from = De
ui-f-to = Para
ui-f-style = Linha
ui-f-title = Título
ui-f-members = Membros
ui-f-type = Tipo
ui-f-condition = Condição
ui-f-position = Posição
ui-f-other = Também sobre
ui-f-name = Nome
ui-f-stereotype = Estereótipo
ui-f-attributes = Atributos (um por linha)
ui-f-methods = Métodos (um por linha)
ui-f-from-card = Cardinalidade (origem)
ui-f-to-card = Cardinalidade (destino)
ui-f-for = Ligado a
ui-f-add = Adicionar
ui-f-cancel = Cancelar
ui-f-none = (nenhum)
ui-o-rect = Retângulo
ui-o-round = Arredondado
ui-o-stadium = Cápsula
ui-o-decision = Decisão
ui-o-database = Banco de dados
ui-o-circle = Círculo
ui-o-subroutine = Sub-rotina
ui-o-io = Entrada/Saída
ui-o-hexagon = Hexágono
ui-o-arrow = Seta
ui-o-open = Linha
ui-o-dotted = Pontilhada
ui-o-thick = Grossa
ui-o-both = Dupla
ui-o-sync = Pedido (->>)
ui-o-reply = Resposta (-->>)
ui-o-async = Assíncrona (-))
ui-o-lost = Perdida (-x)
ui-o-right = À direita de
ui-o-left = À esquerda de
ui-o-over = Sobre
ui-o-inheritance = Herança
ui-o-composition = Composição
ui-o-aggregation = Agregação
ui-o-association = Associação
ui-o-dependency = Dependência
ui-o-realization = Realização
ui-o-one = Exatamente um
ui-o-zero_one = Zero ou um
ui-o-zero_many = Zero ou muitos
ui-o-one_many = Um ou muitos
ui-dir-TB = De cima para baixo (TB)
ui-dir-BT = De baixo para cima (BT)
ui-dir-LR = Da esquerda para a direita (LR)
ui-dir-RL = Da direita para a esquerda (RL)
ui-o-flag = Bandeira
ui-i-node = Editar elemento
ui-i-edge = Editar ligação
ui-i-save = Salvar
ui-i-delete = Excluir
ui-i-confirm = Clique de novo para excluir

## Configurações, atualizações, menu MCP

ui-export = Exportar
ui-settings = Configurações
ui-settings-general = Geral
ui-settings-editor = Editor e diagrama
ui-settings-mcp = MCP
ui-appearance = Aparência
ui-appearance-system = Igual ao sistema
ui-appearance-light = Claro
ui-appearance-dark = Escuro
ui-editor-font = Fonte do editor
ui-editor-font-size = Tamanho da fonte do editor
ui-diagram-font = Fonte do diagrama
ui-diagram-font-hint = Gravada na pasta (.odiagramador/config.json) e usada também pelos agentes de IA, para todos verem o mesmo layout.
ui-font-default = Padrão (Arial)
ui-updates = Atualizações
ui-auto-update = Verificar atualizações automaticamente
ui-check-updates = Verificar atualizações…
ui-version = Versão { $version }
ui-up-to-date = Você já tem a versão mais recente ({ $version }).
ui-update-available = O ODiagramador { $version } está disponível (você tem a { $current }).
ui-update-now = Atualizar agora
ui-update-later = Depois
ui-update-release-page = Ver no GitHub
ui-updating = Baixando a atualização…
ui-update-restart = O app vai reiniciar para concluir a atualização.
ui-update-failed = A atualização falhou: { $detail }
ui-update-manual = Esta cópia não consegue se substituir. Baixe a nova versão no GitHub.
ui-mcp-configure = Configurar MCP
ui-mcp-copy-for = Copiar configuração para:
ui-mcp-other = Outro (JSON)
ui-mcp-settings = Configurações do MCP…
ui-mcp-copied = Configuração do MCP para { $app } copiada.
ui-mcp-copy-failed = Não foi possível usar a área de transferência. Copie a configuração abaixo:
ui-mcp-local = Local (este computador)
ui-mcp-local-hint = Agentes neste computador iniciam o próprio servidor do ODiagramador; use os botões acima para copiar a configuração.
ui-mcp-remote = Acesso remoto
ui-mcp-remote-enable = Expor o MCP em uma porta de rede
ui-mcp-port = Porta
ui-mcp-scope = Quem pode conectar
ui-mcp-scope-local = Somente este computador (127.0.0.1)
ui-mcp-scope-lan = Outros computadores da rede (exige token)
ui-mcp-token = Token de acesso
ui-mcp-regenerate = Novo token
ui-mcp-running = Ativo em { $url }
ui-mcp-stopped = Parado
ui-mcp-error = Não foi possível iniciar: { $detail }
ui-mcp-remote-warning = Quem tiver o token pode ler e alterar os diagramas desta pasta. O tráfego é HTTP sem criptografia: fora da rede local, use um túnel SSH ou uma VPN.
ui-mcp-copy-remote = Copiar configuração remota
ui-auto-install = Instalar atualizações automaticamente (o app reinicia)
ui-mcp-need-folder = Abra primeiro a pasta dos diagramas: a configuração do MCP aponta para ela. Abrir uma pasta agora?
ui-mcp-copy-again = Copiar de novo
ui-mcp-next-step = Próximo passo:
ui-mcp-howto-claude-code = Cole no terminal (PowerShell, Prompt de Comando ou Terminal) e pressione Enter. Depois reinicie o Claude Code e confira com /mcp.
ui-mcp-howto-codex = Cole no arquivo de configuração do Codex: ~/.codex/config.toml (no Windows: %USERPROFILE%\.codex\config.toml). Depois reinicie o Codex.
ui-mcp-howto-cursor = No Cursor: Settings → MCP → Add new MCP server, ou cole em ~/.cursor/mcp.json (no Windows: %USERPROFILE%\.cursor\mcp.json). Se o arquivo já tiver "mcpServers", junte só o bloco "odiagramador".
ui-mcp-howto-claude-desktop = No Claude Desktop: Configurações → Desenvolvedor → Editar configuração. Cole em claude_desktop_config.json (no Windows: %APPDATA%\Claude\claude_desktop_config.json); se já houver "mcpServers", junte só o bloco "odiagramador". Depois reinicie o Claude Desktop.
ui-mcp-howto-json = Cole na configuração MCP do seu cliente (formato "mcpServers").
ui-logs = Diagnóstico
ui-open-logs = Abrir pasta de logs
ui-agent = Agente
ui-agent-question = Pergunta do agente
ui-agent-close = Fechar
ui-agent-reply-placeholder = Escreva sua resposta…
ui-agent-send = Enviar
ui-agent-sent = Resposta enviada
