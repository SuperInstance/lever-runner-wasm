// Cross-language validation for lever-runner-wasm (BLAKE2b-128)
// Standalone Node.js test — no external dependencies needed

const fs = require("fs");
const path = require("path");
const crypto = require("crypto");

// ── Test vectors ────────────────────────────────────────────────────────────

const vectors = [
  { id: 0, state: ".........", blake2b_128: "89fa3a927e254bea7218c405823999aa" },
  { id: 1, state: "X........", blake2b_128: "bd5885cc73b761cec505154b238f6234" },
  { id: 2, state: "XO.X.O.XO", blake2b_128: "33ecca5033d17fba55745ae13d73b461" },
  { id: 3, state: "XXX......", blake2b_128: "66c58ca508531138acbeee77d7eba49c" },
  { id: 4, state: "OOO......", blake2b_128: "2cd3b5ad916491d7077c4582e15bf053" },
  { id: 5, state: "X.OX.OX.O", blake2b_128: "2a0db01e3e1939602d78fe2ed0d4302d" },
  { id: 6, state: "XOXOXOXOX", blake2b_128: "97e3dd13e4a3b0e400eb37936fa6a27b" },
  { id: 7, state: "", blake2b_128: "cae66941d9efbd404e4d88758ea67670" },
  { id: 8, state: "a", blake2b_128: "27c35e6e9373877f29e562464e46497e" },
  { id: 9, state: " negotiation:accept ", blake2b_128: "c66444be4724b637789f86b75272b180" },
];

// ── BLAKE2b-128 using Node.js built-in blake2b256 + truncate ──────────────
// Node >= 21 has blake2b256. For older versions we fall back to the WASM module.

function blake2b128_node(input) {
  // Node 21+ supports 'blake2b256' in createHash
  // For broader compat we try it; otherwise use WASM
  try {
    const hash = crypto.createHash("blake2b256").update(input).digest("hex");
    return hash.slice(0, 32); // first 16 bytes = 32 hex chars
  } catch {
    return null;
  }
}

// ── Main ────────────────────────────────────────────────────────────────────

async function main() {
  let pass = 0, fail = 0, total = 0;

  console.log("══════════════════════════════════════════════════════════");
  console.log("🌐 JS/WASM Cross-Language Validation (lever-runner-wasm)");
  console.log("══════════════════════════════════════════════════════════");

  // ── Phase 1: Validate with WASM module ──────────────────────────────────
  console.log("\n📦 Phase 1: WASM module hash_intent()");
  let wasm = null;
  try {
    wasm = require("../pkg/lever_runner_wasm.js");
    console.log("  ✅ WASM module loaded");
  } catch (e) {
    console.log(`  ⚠️  Could not load WASM module: ${e.message}`);
    console.log("  Skipping WASM tests (rebuild with: wasm-pack build --target nodejs)");
  }

  if (wasm) {
    for (const v of vectors) {
      total++;
      const got = wasm.hash_intent(v.state);
      if (got === v.blake2b_128) {
        pass++;
        console.log(`  ✅ Vector ${v.id} (${JSON.stringify(v.state).padEnd(24)}): ${got}`);
      } else {
        fail++;
        console.log(`  ❌ Vector ${v.id}: expected ${v.blake2b_128}, got ${got}`);
      }
    }
  }

  // ── Phase 2: Node.js crypto BLAKE2b (if available) ──────────────────────
  console.log("\n🔐 Phase 2: Node.js crypto.createHash('blake2b256') reference");
  let node_blake2b = false;
  const testHash = blake2b128_node(Buffer.from("........."));
  if (testHash) {
    node_blake2b = true;
    for (const v of vectors) {
      total++;
      const got = blake2b128_node(Buffer.from(v.state));
      if (got === v.blake2b_128) {
        pass++;
        console.log(`  ✅ Node crypto Vector ${v.id}: ${got}`);
      } else {
        fail++;
        console.log(`  ❌ Node crypto Vector ${v.id}: expected ${v.blake2b_128}, got ${got}`);
      }
    }
  } else {
    console.log("  ⚠️  Node.js blake2b256 not available (need Node >= 21)");
    console.log("  Skipping Node crypto reference tests");
  }

  // ── Phase 3: WASM embedding / pipeline smoke tests ──────────────────────
  if (wasm) {
    console.log("\n🧪 Phase 3: WASM embedding & pipeline smoke tests");

    // Embedding dimensions
    total++;
    const emb = wasm.embed_intent("check disk usage");
    if (emb.length === 64) {
      pass++;
      console.log(`  ✅ embed_intent returns 64 dimensions`);
    } else {
      fail++;
      console.log(`  ❌ embed_intent returned ${emb.length} dims, expected 64`);
    }

    // Embedding empty → zeros
    total++;
    const embEmpty = wasm.embed_intent("");
    const allZero = embEmpty.every(v => v === 0);
    if (allZero) {
      pass++;
      console.log(`  ✅ embed_intent("") → all zeros`);
    } else {
      fail++;
      console.log(`  ❌ embed_intent("") should be all zeros`);
    }

    // Hash determinism
    total++;
    const h1 = wasm.hash_intent("test string");
    const h2 = wasm.hash_intent("test string");
    if (h1 === h2) {
      pass++;
      console.log(`  ✅ hash_intent is deterministic`);
    } else {
      fail++;
      console.log(`  ❌ hash_intent not deterministic: ${h1} vs ${h2}`);
    }

    // Hash length
    total++;
    if (h1.length === 32) {
      pass++;
      console.log(`  ✅ hash returns 32 hex chars (BLAKE2b-128)`);
    } else {
      fail++;
      console.log(`  ❌ hash returned ${h1.length} hex chars, expected 32`);
    }

    // Different inputs → different hashes
    total++;
    const h3 = wasm.hash_intent("different string");
    if (h1 !== h3) {
      pass++;
      console.log(`  ✅ Different inputs produce different hashes`);
    } else {
      fail++;
      console.log(`  ❌ Different inputs produced same hash`);
    }

    // Pipeline test
    total++;
    wasm.clear_commands();
    wasm.teach("cross_lang_test_exact", "echo CROSS_LANG_HIT");
    const result = wasm.gate_pipeline("cross_lang_test_exact");
    const parsed = JSON.parse(result);
    if (parsed.gate === 1 && parsed.command === "echo CROSS_LANG_HIT") {
      pass++;
      console.log(`  ✅ Gate 1 exact match works`);
    } else {
      fail++;
      console.log(`  ❌ Gate 1 failed: ${result}`);
    }

    // Vector search
    total++;
    wasm.clear_commands();
    wasm.teach("check disk space", "df -h");
    wasm.teach("show memory usage", "free -h");
    const queryEmb = wasm.embed_intent("check disk space");
    const results = JSON.parse(wasm.vector_search(queryEmb, 2));
    if (results.length > 0 && results[0].command === "df -h") {
      pass++;
      console.log(`  ✅ Vector search returns correct top result`);
    } else {
      fail++;
      console.log(`  ❌ Vector search unexpected: ${JSON.stringify(results)}`);
    }
  }

  // ── Summary ─────────────────────────────────────────────────────────────
  console.log("\n══════════════════════════════════════════════════════════");
  console.log(`  Results: ${pass} passed, ${fail} failed out of ${total} tests`);
  console.log("══════════════════════════════════════════════════════════");

  process.exit(fail > 0 ? 1 : 0);
}

main().catch(e => { console.error(e); process.exit(1); });
