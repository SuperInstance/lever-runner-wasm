//! Native Rust tests for lever-runner-wasm (run with `cargo test`).

use lever_runner_wasm::*;

#[test]
fn test_hash_intent_deterministic() {
    let h1 = hash_intent("check disk usage");
    let h2 = hash_intent("check disk usage");
    assert_eq!(h1, h2, "hash must be deterministic");
    assert_eq!(h1.len(), 32, "hash must be 32 hex chars (BLAKE2b-128)");
}

#[test]
fn test_hash_intent_different_inputs() {
    let h1 = hash_intent("check disk usage");
    let h2 = hash_intent("show memory");
    assert_ne!(h1, h2, "different inputs must produce different hashes");
}

#[test]
fn test_embed_intent_dimensions() {
    let emb = embed_intent("hello world");
    assert_eq!(emb.len(), 64, "embedding must be 64-dimensional");
}

#[test]
fn test_embed_intent_empty() {
    let emb = embed_intent("");
    assert_eq!(emb.len(), 64);
    assert!(emb.iter().all(|&v| v == 0.0), "empty string → zero embedding");
}

#[test]
fn test_embed_intent_normalized() {
    let emb = embed_intent("test input string");
    let norm: f64 = emb[..40].iter().map(|x| x * x).sum::<f64>().sqrt();
    assert!((norm - 1.0).abs() < 0.01 || norm == 0.0, "dims 0-40 should be normalized, got norm={}", norm);
}

#[test]
fn test_teach_and_search() {
    clear_commands();
    teach("xcheck_disk_space_x", "df -h");
    teach("xshow_memory_usage_x", "free -h");
    teach("xlist_processes_x", "ps aux");

    let emb = embed_intent("xcheck_disk_space_x");
    let results = vector_search(&emb, 3);
    assert!(!results.is_empty());
    assert!(results.contains("df -h"), "should find df -h, got: {}", results);
}

#[test]
fn test_gate_pipeline_exact() {
    clear_commands();
    teach("xunique_exact_test_42", "echo EXACT_HIT");
    let result = gate_pipeline("xunique_exact_test_42");
    assert!(result.contains("\"gate\":1"), "exact match → gate 1, got: {}", result);
    assert!(result.contains("echo EXACT_HIT"));
}

#[test]
fn test_gate_pipeline_similar() {
    clear_commands();
    teach("xcheck_disk_usage_sim", "df -h");
    let result = gate_pipeline("xcheck_disk_usage_sim similar");
    assert!(result.contains("\"gate\":") && result.contains("\"command\":"));
}

#[test]
fn test_gate_pipeline_fallback() {
    clear_commands();
    let result = gate_pipeline("xyzzy_plugh_nothing_matchable");
    assert!(result.contains("\"gate\":3"), "unknown intent → gate 3, got: {}", result);
}

#[test]
fn test_seed_commands() {
    clear_commands();
    let count = seed_commands();
    assert!(count >= 25, "should seed at least 25 commands, got {}", count);
    assert_eq!(command_count(), count, "count should match after seed");
}

#[test]
fn test_export_commands() {
    clear_commands();
    teach("xexport_test", "echo export");
    let exported = export_commands();
    assert!(exported.contains("xexport_test"));
    assert!(exported.contains("echo export"));
}

#[test]
fn test_load_commands_json() {
    clear_commands();
    let json = r#"[{"intent":"xload1","command":"echo 1"},{"intent":"xload2","command":"echo 2"}]"#;
    let count = load_commands(json);
    assert_eq!(count, 2);
    assert_eq!(command_count(), 2);
}

#[test]
fn test_load_commands_wrapped() {
    clear_commands();
    let json = r#"{"commands":[{"intent":"xwrap1","command":"echo w"}]}"#;
    let count = load_commands(json);
    assert_eq!(count, 1);
}
