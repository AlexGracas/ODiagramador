// Bundle entry for render-kit/vendor/mermaid-odg.min.js (built by scripts/build-render-kit.sh).
// Exposes the same `window.mermaid` global the bridge always used, plus ODiagramador's own
// layouts (ELK, interactive ELK, exact positions) registered through Mermaid's layout API.
import mermaid, { createCommonLayoutRenderer } from "mermaid";
import { createLayouts, layoutHints } from "./odg-layouts.mjs";

mermaid.registerLayoutLoaders(createLayouts(createCommonLayoutRenderer));

window.mermaid = mermaid;
window.odgLayout = { hints: layoutHints };
