# lever-runner-wasm

WebAssembly build of the [lever-runner](https://github.com/SuperInstance/lever-runner) carapace for browser deployment.

Runs the three-gate intent→command pipeline entirely in the browser with near-native speed — no server, no API keys, no network required.

## Architecture

```
┌──────────────┐     ┌───────────────────────────────┐
│  JavaScript  │────▶│  WASM (Rust, compiled cdylib)  │
│  index.js    │◀────│                                │
└──────────────┘     │  ┌─ Gate 1: Exact hash match   │
                     │  ├─ Gate 2: Vector similarity   │
                     │  └─ Gate 3: Keyword fallback    │
                     └───────────────────────────────┘
```

## WASM API

```javascript
import init, {
    hash_intent, embed_intent, vector_search,
    gate_pipeline, teach, seed_commands, export_commands, command_count
} from './pkg/lever_runner_wasm.js';

await init();
seed_commands(); // Load default commands

// Hash an intent string → 16-char hex
const hash = hash_intent("check disk usage");

// Embed intent → Float64Array (64-dim)
const embedding = embed_intent("check disk usage");

// Vector search
const results = JSON.parse(vector_search(embedding, 5));

// Full three-gate pipeline
const result = JSON.parse(gate_pipeline("check disk usage"));
// → { gate: 1, command: "df -h", confidence: 1.0, latency_us: 48 }

// Teach new commands
teach("show my ip", "curl ifconfig.me");

// Export all commands
const commands = export_commands();
```

## Building

```bash
# Install wasm-pack if needed
cargo install wasm-pack

# Build for web target
wasm-pack build --target web

# Run native tests
cargo test
```

## Demo

After building, serve the `www/` directory:

```bash
# Any static file server, e.g.:
npx serve .
# or
python3 -m http.server 8000
```

Then open `www/index.html` in your browser.

## Embedding Approach

Pure math — no neural network or torch dependency:

| Dimensions | Content | Method |
|-----------|---------|--------|
| 0–39 | Character frequency | Position-weighted bucket hashing, L2-normalized |
| 40–55 | Bigram frequency | Character pair hashing into 16 buckets, L2-normalized |
| 56–63 | Structural features | Log-length, word count, char diversity, digit/path flags |

## Size Target

- WASM binary: < 100KB gzipped
- Zero external dependencies at runtime (no torch, no onnx)
- Command database in WASM linear memory

## Browser Support

Chrome · Firefox · Safari · Edge (all modern versions with WASM support)

## License

MIT
