/*
 * ODiagramador core — C ABI (JSON in, JSON out).
 *
 *   char *r = odg_invoke("list", "{\"workspace\":\"/path/to/folder\"}");
 *   // r = {"ok":true,"result":[...]}  or  {"ok":false,"error":"..."}
 *   odg_free(r);
 *
 * Methods (args are a JSON object):
 *   version                                             -> {version, mermaid}
 *   strings          {lang}                             -> {key: text}   (all UI strings)
 *   t                {lang, key, args?}                 -> string
 *   analyze          {source, raw, lang?, workspace?, path?, save?, log?, actor?} -> Report
 *                    (raw = the "result" object posted by render-kit bridge.js)
 *   annotate_svg     {svg, view_box?, problems}         -> svg string
 *   find_line        {source, id}                       -> line | null
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
 *   config           {workspace}                        -> {lang?, theme?}
 *   set_config       {workspace, lang?, theme?}
 *   mcp_config       {odg_path, workspace}              -> {json, claude_code, codex}
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
