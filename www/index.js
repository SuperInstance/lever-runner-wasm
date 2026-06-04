import init, {
    hash_intent,
    embed_intent,
    vector_search,
    gate_pipeline,
    teach,
    seed_commands,
    export_commands,
    command_count,
} from '../pkg/lever_runner_wasm.js';

let wasmReady = false;

async function loadWasm() {
    await init();
    const count = seed_commands();
    wasmReady = true;
    updateDbCount();
    console.log(`⚡ lever-runner-wasm loaded with ${count} commands`);
}

function updateDbCount() {
    document.getElementById('db-count').textContent = command_count();
}

function runPipeline(intent) {
    if (!wasmReady || !intent.trim()) return;

    const hash = hash_intent(intent);
    const embedding = embed_intent(intent);
    const result = JSON.parse(gate_pipeline(intent));

    document.getElementById('results').style.display = 'block';
    document.getElementById('result-hash').textContent = hash;
    document.getElementById('result-command').textContent = result.command;
    document.getElementById('result-latency').textContent = `${result.latency_us} µs`;

    // Gate badge
    const gateEl = document.getElementById('result-gate');
    gateEl.textContent = `Gate ${result.gate}`;
    gateEl.className = `gate-badge gate-${result.gate}`;

    // Confidence bar
    const conf = result.confidence;
    const confFill = document.getElementById('result-confidence-fill');
    const confText = document.getElementById('result-confidence-text');
    confFill.style.width = `${conf * 100}%`;
    confFill.style.background = conf > 0.8 ? 'var(--green)' : conf > 0.5 ? 'var(--accent)' : 'var(--yellow)';
    confText.textContent = `${(conf * 100).toFixed(1)}%`;

    // Embedding preview
    const embPreview = embedding.slice(0, 8).map(v => v.toFixed(3)).join(', ');
    document.getElementById('result-embedding').textContent = `[${embPreview}, ...] (${embedding.length} dims)`;
}

// ── Event listeners ─────────────────────────────────────────────────────────

document.addEventListener('DOMContentLoaded', () => {
    loadWasm();

    const input = document.getElementById('intent-input');
    const runBtn = document.getElementById('run-btn');

    runBtn.addEventListener('click', () => runPipeline(input.value));
    input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') runPipeline(input.value);
    });

    // Example chips
    document.querySelectorAll('.chip').forEach(chip => {
        chip.addEventListener('click', () => {
            input.value = chip.dataset.intent;
            runPipeline(chip.dataset.intent);
        });
    });

    // Teach
    document.getElementById('teach-btn').addEventListener('click', () => {
        const intentVal = document.getElementById('teach-intent').value.trim();
        const cmdVal = document.getElementById('teach-command').value.trim();
        const status = document.getElementById('teach-status');
        if (!intentVal || !cmdVal) {
            status.textContent = '⚠ Fill in both fields';
            status.className = 'status err';
            return;
        }
        teach(intentVal, cmdVal);
        status.textContent = `✓ Taught "${intentVal}" → ${cmdVal}`;
        status.className = 'status ok';
        updateDbCount();
        document.getElementById('teach-intent').value = '';
        document.getElementById('teach-command').value = '';
    });

    // Seed / Export
    document.getElementById('seed-btn').addEventListener('click', () => {
        seed_commands();
        updateDbCount();
    });

    document.getElementById('export-btn').addEventListener('click', () => {
        const out = document.getElementById('export-output');
        out.textContent = export_commands();
        out.style.display = out.style.display === 'none' ? 'block' : 'none';
    });
});
