//! # lever-runner-wasm
//!
//! WebAssembly build of the lever-runner carapace for browser deployment.
//! Provides near-native performance for intent hashing, embedding, vector search,
//! and the three-gate pipeline directly in the browser.

use wasm_bindgen::prelude::*;
use blake2::{Blake2b, Digest, digest::consts::U16};
use std::sync::RwLock;

/// Cross-platform microsecond timer.
fn now_us() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() * 1000.0) as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }
}

// ── Internal state ──────────────────────────────────────────────────────────

/// A command entry in the database.
#[derive(Clone)]
struct CommandEntry {
    intent: String,
    command: String,
    embedding: Vec<f64>,
}

/// Global command database, lazily initialized.
static COMMAND_DB: RwLock<Option<Vec<CommandEntry>>> = RwLock::new(None);

// ── Hashing ─────────────────────────────────────────────────────────────────

/// Hash an intent string with BLAKE2b → 16-char hex string.
#[wasm_bindgen]
pub fn hash_intent(intent: &str) -> String {
    let mut hasher = Blake2b::<U16>::new();
    hasher.update(intent.as_bytes());
    let result = hasher.finalize();
    // Full 16 bytes → 32 hex chars (BLAKE2b-128)
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

// ── Embedding ───────────────────────────────────────────────────────────────

/// Character set for position-aware encoding.
const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789 _-./";

/// Get character index in charset.
fn char_index(c: char) -> usize {
    let lc = c.to_ascii_lowercase() as u8;
    CHARSET.iter().position(|&b| b == lc).unwrap_or(CHARSET.len() - 1)
}

/// Embed an intent string into a 64-dimensional Float64Array.
///
/// Pure math embedding using position-aware character frequency encoding:
/// - Dims 0-39: Character frequency + positional weighting
/// - Dims 40-55: Bigram frequency
/// - Dims 56-63: Structural features (length, word count, etc.)
#[wasm_bindgen]
pub fn embed_intent(intent: &str) -> Vec<f64> {
    let text = intent.to_ascii_lowercase();
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut embedding = vec![0.0f64; 64];

    if n == 0 {
        return embedding;
    }

    // Dims 0-39: Character frequency with positional decay
    let charset_len = CHARSET.len();
    let bucket_size = (charset_len as f64 / 40.0).ceil() as usize;
    for (pos, &c) in chars.iter().enumerate() {
        let idx = char_index(c);
        let bucket = (idx / bucket_size).min(39);
        let positional_weight = 1.0 + 2.0 / (1.0 + pos as f64); // earlier chars matter more
        embedding[bucket] += positional_weight / (n as f64).sqrt();
    }

    // Normalize dims 0-39
    let norm_0_40: f64 = embedding[..40].iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_0_40 > 0.0 {
        for v in embedding[..40].iter_mut() {
            *v /= norm_0_40;
        }
    }

    // Dims 40-55: Bigram frequency (16 buckets via simple hash)
    if n >= 2 {
        for i in 0..n - 1 {
            let bigram_hash = ((chars[i] as usize) * 31 + (chars[i + 1] as usize)) % 16;
            embedding[40 + bigram_hash] += 1.0 / (n as f64).sqrt();
        }
        let norm_bigram: f64 = embedding[40..56].iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm_bigram > 0.0 {
            for v in embedding[40..56].iter_mut() {
                *v /= norm_bigram;
            }
        }
    }

    // Dims 56-63: Structural features
    let word_count = text.split_whitespace().count() as f64;
    let avg_word_len = if word_count > 0.0 {
        text.split_whitespace().map(|w| w.len()).sum::<usize>() as f64 / word_count
    } else {
        0.0
    };
    let has_digits = text.chars().any(|c| c.is_ascii_digit()) as u8 as f64;
    let has_path = text.contains('/') || text.contains('.');
    let unique_chars = {
        let mut s: std::collections::HashSet<char> = std::collections::HashSet::new();
        chars.iter().for_each(|c| { s.insert(*c); });
        s.len() as f64
    };

    embedding[56] = (n as f64).ln_1p() / 5.0;            // log-normalized length
    embedding[57] = word_count / 10.0;                      // word count (capped)
    embedding[58] = avg_word_len / 10.0;                    // avg word length
    embedding[59] = unique_chars / 26.0;                    // character diversity
    embedding[60] = has_digits;                              // contains digits
    embedding[61] = has_path as usize as f64;                // contains path chars
    embedding[62] = (n as f64 / word_count.max(1.0)) / 5.0; // chars per word
    embedding[63] = text.matches(&[' ', '-'][..]).count() as f64 / (n as f64).max(1.0); // separator ratio

    embedding
}

// ── Vector Search ───────────────────────────────────────────────────────────

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Search the command database for similar intents.
///
/// Returns JSON array of { intent, command, score } sorted by similarity.
#[wasm_bindgen]
pub fn vector_search(query_embedding: &[f64], top_k: usize) -> String {
    let db = COMMAND_DB.read().unwrap();
    let db = match db.as_ref() {
        Some(d) => d,
        None => return "[]".to_string(),
    };

    let mut results: Vec<serde_json::Value> = db.iter()
        .filter_map(|entry| {
            let score = cosine_similarity(query_embedding, &entry.embedding);
            if score > 0.01 {
                Some((entry, score))
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|(entry, score)| {
            serde_json::json!({
                "intent": entry.intent,
                "command": entry.command,
                "score": (score * 1000.0).round() / 1000.0,
            })
        })
        .collect();

    // Sort by score descending
    results.sort_by(|a, b| {
        b["score"].as_f64().unwrap_or(0.0)
            .partial_cmp(&a["score"].as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results.truncate(top_k);
    serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string())
}

// ── Three-Gate Pipeline ─────────────────────────────────────────────────────

/// Run the full three-gate pipeline on an intent string.
///
/// Gate 1: Exact hash match (instant, ~0µs)
/// Gate 2: Vector similarity search (~10-50µs)
/// Gate 3: Fuzzy/heuristic fallback (~50-200µs)
///
/// Returns JSON: { gate, command, confidence, latency_us }
#[wasm_bindgen]
pub fn gate_pipeline(intent: &str) -> String {
    let start = now_us();

    let hash = hash_intent(intent);
    let embedding = embed_intent(intent);

    // Gate 1: Exact hash match
    {
        let db = COMMAND_DB.read().unwrap();
        if let Some(ref db) = *db {
            for entry in db {
                let entry_hash = hash_intent(&entry.intent);
                if entry_hash == hash {
                    let elapsed = now_us() - start;
                    return serde_json::json!({
                        "gate": 1,
                        "command": entry.command,
                        "confidence": 1.0,
                        "latency_us": elapsed,
                        "hash": hash,
                    }).to_string();
                }
            }
        }
    }

    // Gate 2: Vector similarity (threshold 0.75)
    {
        let db = COMMAND_DB.read().unwrap();
        if let Some(ref db) = *db {
            let mut best: Option<(&CommandEntry, f64)> = None;
            for entry in db {
                let score = cosine_similarity(&embedding, &entry.embedding);
                if score > 0.75 {
                    match best {
                        Some((_, best_score)) if score <= best_score => {}
                        _ => best = Some((entry, score)),
                    }
                }
            }
            if let Some((entry, score)) = best {
                let elapsed = now_us() - start;
                return serde_json::json!({
                    "gate": 2,
                    "command": entry.command,
                    "confidence": (score * 1000.0).round() / 1000.0,
                    "latency_us": elapsed,
                    "hash": hash,
                }).to_string();
            }
        }
    }

    // Gate 3: Keyword heuristic fallback
    let intent_lower = intent.to_ascii_lowercase();
    let command = keyword_fallback(&intent_lower);
    let elapsed = now_us() - start;

    serde_json::json!({
        "gate": 3,
        "command": command,
        "confidence": 0.3,
        "latency_us": elapsed,
        "hash": hash,
    }).to_string()
}

/// Simple keyword-based command fallback for Gate 3.
fn keyword_fallback(intent_lower: &str) -> String {
    let keywords: &[(&[&str], &str)] = &[
        (&["disk", "space", "usage", "storage"], "df -h"),
        (&["memory", "ram", "mem", "usage"], "free -h"),
        (&["ip", "address", "my ip"], "curl ifconfig.me"),
        (&["process", "running", "ps"], "ps aux"),
        (&["port", "listening", "open port"], "ss -tlnp"),
        (&["network", "connection", "netstat"], "ss -tunap"),
        (&["file", "find", "search file"], "find . -name "),
        (&["docker", "container", "images"], "docker ps"),
        (&["git", "status", "repo"], "git status"),
        (&["log", "logs", "tail"], "tail -f /var/log/syslog"),
        (&["cpu", "processor", "top"], "top -bn1 | head -20"),
        (&["who", "users", "logged"], "who"),
        (&["uptime", "load"], "uptime"),
        (&["uname", "kernel", "system info"], "uname -a"),
        (&["ping", "test network"], "ping -c 4 google.com"),
        (&["curl", "download", "fetch"], "curl -sL "),
        (&["ls", "list", "dir", "directory"], "ls -lah"),
        (&["cat", "read", "show", "view"], "cat "),
        (&["grep", "search", "find text"], "grep -r "),
        (&["chmod", "permission", "permissions"], "chmod "),
        (&["kill", "stop", "terminate"], "kill "),
        (&["ssh", "remote", "connect"], "ssh "),
        (&["scp", "copy remote"], "scp "),
        (&["tar", "archive", "compress", "zip"], "tar czf "),
        (&["sudo", "admin", "root"], "sudo "),
        (&["apt", "install", "package"], "apt install "),
        (&["npm", "node", "package"], "npm install "),
        (&["python", "pip", "python3"], "python3 "),
        (&["make", "build", "compile"], "make"),
        (&["systemctl", "service", "daemon"], "systemctl status "),
        (&["cron", "schedule", "timer"], "crontab -l"),
        (&["env", "environment", "variable"], "env | grep "),
        (&["password", "passwd", "change password"], "passwd"),
        (&["firewall", "iptables", "ufw"], "ufw status"),
        (&["mount", "drive", "disk"], "mount | column -t"),
        (&["df", "disk free"], "df -h"),
    ];

    for (keys, cmd) in keywords {
        if keys.iter().any(|k| intent_lower.contains(k)) {
            return cmd.to_string();
        }
    }

    format!("echo 'Unknown intent: {}'", intent_lower)
}

// ── Teaching / Database Management ──────────────────────────────────────────

/// Teach the system a new intent → command mapping.
#[wasm_bindgen]
pub fn teach(intent: &str, command: &str) {
    let embedding = embed_intent(intent);
    let entry = CommandEntry {
        intent: intent.to_string(),
        command: command.to_string(),
        embedding,
    };

    let mut db = COMMAND_DB.write().unwrap();
    match db.as_mut() {
        Some(entries) => entries.push(entry),
        None => *db = Some(vec![entry]),
    }
}

/// Load a batch of commands from a JSON string.
///
/// JSON format: { "commands": [{ "intent": "...", "command": "..." }, ...] }
/// or just an array: [{ "intent": "...", "command": "..." }, ...]
#[wasm_bindgen]
pub fn load_commands(json: &str) -> usize {
    let entries: Vec<CommandEntry> = if let Ok(arr) = serde_json::from_str::<Vec<SerdeEntry>>(json) {
        arr.into_iter().map(|e| CommandEntry {
            embedding: embed_intent(&e.intent),
            ..e.into()
        }).collect()
    } else if let Ok(wrapped) = serde_json::from_str::<SerdeCommands>(json) {
        wrapped.commands.into_iter().map(|e| CommandEntry {
            embedding: embed_intent(&e.intent),
            ..e.into()
        }).collect()
    } else {
        return 0;
    };

    let count = entries.len();
    let mut db = COMMAND_DB.write().unwrap();
    match db.as_mut() {
        Some(existing) => existing.extend(entries),
        None => *db = Some(entries),
    }
    count
}

/// Export all commands as JSON.
#[wasm_bindgen]
pub fn export_commands() -> String {
    let db = COMMAND_DB.read().unwrap();
    match db.as_ref() {
        Some(entries) => {
            let out: Vec<serde_json::Value> = entries.iter().map(|e| {
                serde_json::json!({
                    "intent": e.intent,
                    "command": e.command,
                })
            }).collect();
            serde_json::to_string(&out).unwrap_or_else(|_| "[]".to_string())
        }
        None => "[]".to_string(),
    }
}

/// Clear the command database.
#[wasm_bindgen]
pub fn clear_commands() {
    let mut db = COMMAND_DB.write().unwrap();
    *db = None;
}

/// Get the number of commands in the database.
#[wasm_bindgen]
pub fn command_count() -> usize {
    let db = COMMAND_DB.read().unwrap();
    db.as_ref().map(|e| e.len()).unwrap_or(0)
}

/// Seed the database with common commands.
#[wasm_bindgen]
pub fn seed_commands() -> usize {
    let seed_json = r#"[
        {"intent": "check disk usage", "command": "df -h"},
        {"intent": "show memory usage", "command": "free -h"},
        {"intent": "list running processes", "command": "ps aux"},
        {"intent": "show my ip address", "command": "curl ifconfig.me"},
        {"intent": "check open ports", "command": "ss -tlnp"},
        {"intent": "show network connections", "command": "ss -tunap"},
        {"intent": "git status", "command": "git status"},
        {"intent": "docker list containers", "command": "docker ps"},
        {"intent": "check system uptime", "command": "uptime"},
        {"intent": "show system info", "command": "uname -a"},
        {"intent": "list files", "command": "ls -lah"},
        {"intent": "check cpu usage", "command": "top -bn1 | head -20"},
        {"intent": "show logged in users", "command": "who"},
        {"intent": "find large files", "command": "find . -type f -size +100M"},
        {"intent": "check firewall status", "command": "ufw status"},
        {"intent": "view system logs", "command": "journalctl -u"},
        {"intent": "restart service", "command": "systemctl restart "},
        {"intent": "check environment variables", "command": "env | grep "},
        {"intent": "ping test", "command": "ping -c 4 google.com"},
        {"intent": "download file", "command": "curl -sLO "},
        {"intent": "search text in files", "command": "grep -r "},
        {"intent": "create archive", "command": "tar czf archive.tar.gz "},
        {"intent": "install package", "command": "apt install "},
        {"intent": "check cron jobs", "command": "crontab -l"},
        {"intent": "mount disks", "command": "mount | column -t"},
        {"intent": "show directory tree", "command": "find . -type d | head -50"},
        {"intent": "kill process", "command": "kill "},
        {"intent": "ssh connect", "command": "ssh "},
        {"intent": "file permissions", "command": "chmod "},
        {"intent": "compress files", "command": "tar czf "}
    ]"#;
    load_commands(seed_json)
}

// ── Serde helpers ───────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct SerdeEntry {
    intent: String,
    command: String,
}

impl From<SerdeEntry> for CommandEntry {
    fn from(e: SerdeEntry) -> Self {
        CommandEntry {
            intent: e.intent,
            command: e.command,
            embedding: vec![0.0; 64], // placeholder, will be overwritten
        }
    }
}

#[derive(serde::Deserialize)]
struct SerdeCommands {
    commands: Vec<SerdeEntry>,
}

// ── WASM lifecycle ──────────────────────────────────────────────────────────

#[wasm_bindgen(start)]
pub fn init() {
    // Set up panic hook for better error messages in browser console
    console_error_panic_hook::set_once();
}
