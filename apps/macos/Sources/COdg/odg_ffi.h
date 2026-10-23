/*
 * ODiagramador core — C ABI (JSON in, JSON out).
 *
 *   char *r = odg_invoke("list", "{\"workspace\":\"/path/to/folder\"}");
 *   // r = {"ok":true,"result":[...]}  or  {"ok":false,"error":"..."}
 *   odg_free(r);
 *
 * Methods (args are a JSON object):
 *   version                                             -> {version, mermaid, releases_api, releases_page}
 *   update_evaluate  {release_json, current, platform}  -> {available, latest, asset_url, page_url, notes...}
 *   strings          {lang}                             -> {key: text}   (all UI strings)
 *   markdown_blocks  {text}                             -> [{kind, level, number?, spans[{text, bold?, italic?, code?, link?}]}]
 *                    kind: heading | paragraph | bullet | code | quote | rule   (to draw Markdown natively)
 *   t                {lang, key, args?}                 -> string
 *   analyze          {source, raw, lang?, workspace?, path?, save?, log?, actor?} -> Report
 *                    (raw = the "result" object posted by render-kit bridge.js)
 *   annotate_svg     {svg, view_box?, problems}         -> svg string
 *   find_line        {source, id}                       -> line | null
 *   layout_edit      {source, op, id?, x?, y?, mode?, layer?} -> new source
 *                    op: move {id,x,y} | mode {mode} | reset {id?} | layer {id, layer?}
 *   layout_directives {source}                          -> {mode, positions[], layers[]}
 *   construct        {source, construct, params}        -> new source (palette buttons)
 *   edit             {source, action, params}           -> new source (inspector)
 *                    action: rename {id,label} | shape {id,shape} | delete {id}
 *                            | edge_label {from,to,index,label} | edge_delete {from,to,index}
 *   template         {kind, lang?}                      -> starter text | null (any kind from diagram_kinds)
 *   diagram_kinds    {lang?}                            -> [{kind, label, graph}]  every Mermaid diagram type
 *   syntax_help      {topic}                            -> markdown | null
 *   list             {workspace}                        -> DiagramInfo[]
 *   read             {workspace, path}                  -> string
 *   write            {workspace, path, content, actor?} -> relative path
 *   delete           {workspace, path, actor?}
 *   report           {workspace, path}                  -> Report | null
 *   events           {workspace, limit?, path?}         -> Event[] (newest first)
 *   feedback         {workspace, path?, include_resolved?} -> Feedback[]
 *   add_feedback     {workspace, path, text, element_id?, line?, author?} -> Feedback
 *   resolve_feedback {workspace, id, resolution?, actor?} -> Feedback
 *   viewer_inbox     {workspace, cursor?}               -> {cursor, messages[]}  agent messages for the viewer:
 *                    kind show | highlight | view | notify | ask (no cursor = recent + unanswered questions)
 *   answer           {workspace, question_id, text}     -> Message (the human's reply)
 *   messages         {workspace, limit?}                -> Message[] (conversation, oldest first)
 *   set_presence     {workspace, path?, element_id?, zoom?, center_element?, visible_elements?,
 *                    app?, version?, platform?}  (call every ~2 s)
 *   clear_presence   {workspace}                         (viewer closing / switching folder)
 *   config           {workspace}                        -> {lang?, theme?, font?}
 *   set_config       {workspace, lang?, theme?, font?}   (merges; "" clears)
 *   mcp_config       {odg_path, workspace, url?, token?} -> {json, claude_code, codex,
 *                                                          remote_json?, remote_claude_code?, remote_codex?}
 */
#ifndef ODG_FFI_H
#define ODG_FFI_H

#ifdef __cplusplus
extern "C" {
#endif

char *odg_invoke(const char *method, const char *args_json);
void odg_free(char *s);

#ifdef __cplusplus
}
#endif

#endif /* ODG_FFI_H */
