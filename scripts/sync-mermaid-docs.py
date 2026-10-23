#!/usr/bin/env python3
"""Offline syntax digests of Mermaid's official docs (MIT, © Mermaid contributors), one per diagram type.

Keeps headings, the first paragraph after each heading, bullet lists, tables and every example, and drops
the rest (badges, rendered duplicates, long prose). Used by get_syntax_help and as starter templates.

    scripts/sync-mermaid-docs.py [MERMAID_VERSION]      (default: render-kit/vendor/MERMAID_VERSION)
"""
import pathlib
import re
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "docs" / "syntax" / "mermaid"
VERSION = sys.argv[1] if len(sys.argv) > 1 else (ROOT / "render-kit/vendor/MERMAID_VERSION").read_text().strip()

# doc file -> ODiagramador kind id (also the get_syntax_help topic)
DOCS = {
    "flowchart": "flowchart", "sequenceDiagram": "sequence", "classDiagram": "class", "stateDiagram": "state",
    "entityRelationshipDiagram": "er", "userJourney": "journey", "gantt": "gantt", "pie": "pie",
    "quadrantChart": "quadrant", "requirementDiagram": "requirement", "gitgraph": "gitgraph", "c4": "c4",
    "mindmap": "mindmap", "timeline": "timeline", "zenuml": "zenuml", "sankey": "sankey", "xyChart": "xychart",
    "block": "block", "packet": "packet", "kanban": "kanban", "architecture": "architecture", "radar": "radar",
    "eventmodeling": "eventmodeling", "treemap": "treemap", "venn": "venn", "ishikawa": "ishikawa",
    "wardley": "wardley", "cynefin": "cynefin", "treeView": "treeview", "usecase": "usecase",
    "swimlanes": "swimlane", "railroad": "railroad", "agentflow": "agentflow",
}


def digest(text: str) -> str:
    lines = text.splitlines()
    out, i, after_heading = [], 0, False
    while i < len(lines):
        line = lines[i]
        if re.match(r"#{1,4} ", line):
            out += ["", line]
            after_heading = True
        elif line.strip() in ("```mermaid-example", "```mermaid"):
            fence = line.strip()
            j, block = i + 1, []
            while j < len(lines) and lines[j].strip() != "```":
                block.append(lines[j])
                j += 1
            # docs show each example twice (code, then the rendered "```mermaid"): keep the code only
            if fence == "```mermaid-example" or not any(b == block for b in getattr(digest, "seen", [])):
                out += ["```mermaid", *block, "```"]
            digest.seen = getattr(digest, "seen", []) + [block]
            i = j
        elif line.startswith("```"):
            j = i + 1
            block = [line]
            while j < len(lines) and not lines[j].startswith("```"):
                block.append(lines[j])
                j += 1
            if len(block) <= 12:  # short non-Mermaid snippets (syntax forms) are useful
                out += [*block, "```"]
            i = j
        elif re.match(r"\s*([-*] |\d+\. |\|)", line):
            out.append(line)
        elif line.strip() and after_heading and not line.startswith(("<", ">", "!")):
            out.append(line if len(line) < 400 else line[:400] + "…")
            after_heading = False
        i += 1
    digest.seen = []
    return re.sub(r"\n{3,}", "\n\n", "\n".join(out)).strip() + "\n"


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for doc, kind in DOCS.items():
        url = f"https://raw.githubusercontent.com/mermaid-js/mermaid/mermaid@{VERSION}/docs/syntax/{doc}.md"
        text = urllib.request.urlopen(url, timeout=30).read().decode("utf-8")
        body = digest(text)
        header = (f"<!-- Digest of Mermaid {VERSION} docs/syntax/{doc}.md — MIT © Mermaid contributors "
                  f"(render-kit/vendor/mermaid.LICENSE). Regenerate with scripts/sync-mermaid-docs.py -->\n")
        (OUT / f"{kind}.md").write_text(header + body)
        print(f"{kind:14} {len(body):6} chars")


if __name__ == "__main__":
    main()
