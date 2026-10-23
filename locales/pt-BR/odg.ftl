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

problem-W_FOREIGN_OBJECT = Este tipo de diagrama usa textos HTML; a prévia PNG pode ficar sem texto.
hint-W_FOREIGN_OBJECT = Prefira a prévia SVG para este diagrama.

problem-W_RASTER = Não foi possível gerar a prévia PNG: { $detail }
hint-W_RASTER = Use a prévia SVG.

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
ui-cancel = Cancelar
ui-create = Criar
