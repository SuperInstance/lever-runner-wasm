# Future Integration: lever-runner-wasm

## Current State
WebAssembly build of lever-runner's carapace for browser deployment. Runs the three-gate intent→command pipeline entirely in the browser with near-native speed — no server, no API keys, no network required. Exposes hash_intent, embed_intent, vector_search, gate_pipeline, teach, seed_commands to JavaScript.

## Integration Opportunities

### With BrowserRoom
lever-runner-wasm IS the BrowserRoom's command engine. When a room runs in the browser (WASM, zero install), lever-runner-wasm provides the teach/learn/match pipeline. Users teach the room commands, the room executes them forever. No server, no install, no API keys — the room runs entirely in the browser tab.

### With Spreadsheet-moment
Spreadsheet-moment's Univer UI can embed lever-runner-wasm for command matching within the spreadsheet. "Show me dying cells" → gate pipeline matches to a filter action → cells are filtered in real-time. Natural language control of the spreadsheet via WASM.

### With superinstance-spreadsheet
The browser demo already runs in a single HTML file. Adding lever-runner-wasm gives it natural language control: type "evolve for 100 generations" and the three-gate pipeline matches it to the evolve() function call. No LLM needed for known commands.

## Dormant Ideas Now Unlockable
The WASM build was for shell commands in the browser. Now it's for room control in the browser. The same pipeline, different domain. Browser-based rooms are now possible because lever-runner-wasm provides the control layer.

## Potential in Mature Systems
Every browser-accessible room has lever-runner-wasm embedded. Users control rooms through natural language, the WASM pipeline matches to actions, and rooms respond instantly. The browser becomes a first-class room client — no install, no server, full control.

## Cross-Pollination Ideas
- **lever-runner**: Python API mirrors WASM API — same skills, different runtime
- **lever-runner-carapace**: Shared Rust codebase compiles to both native and WASM
- **Spreadsheet-moment**: WASM pipeline embedded in Univer UI for spreadsheet control

## Dependencies for Next Steps
- Room command vocabulary (beyond shell commands)
- Integration with BrowserRoom WASM runtime
- JavaScript API for room control via natural language
