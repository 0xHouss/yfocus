use std::process::Command;
use std::path::PathBuf;
use tempfile::TempDir;
use serde_json::Value;

fn yfocus_bin() -> PathBuf {
    // CARGO_BIN_EXE_yfocus is set by cargo for integration tests
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_yfocus") {
        return PathBuf::from(p);
    }
    // fallback for `cargo test --bin yfocus` or manual run
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/yfocus")
}

fn run(dir: &TempDir, args: &[&str]) -> std::process::Output {
    Command::new(yfocus_bin())
        .args(args)
        .env("XDG_STATE_HOME", dir.path())
        .output()
        .expect("failed to spawn yfocus")
}

fn run_ok(dir: &TempDir, args: &[&str]) -> String {
    let out = run(dir, args);
    assert!(
        out.status.success(),
        "yfocus {:?} failed: stderr={}, stdout={}",
        args,
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    String::from_utf8(out.stdout).unwrap()
}

fn run_err(dir: &TempDir, args: &[&str]) -> (i32, String) {
    let out = run(dir, args);
    assert!(!out.status.success(), "expected failure for {:?}", args);
    let code = out.status.code().unwrap_or(1);
    let stderr = String::from_utf8(out.stderr).unwrap();
    (code, stderr)
}

fn read_queue(dir: &TempDir) -> Value {
    let p = dir.path().join("omarchy/yfocus/queue.json");
    let text = std::fs::read_to_string(&p).unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn help_is_printed() {
    let dir = TempDir::new().unwrap();
    for args in [vec!["--help"], vec!["-h"], vec!["help"], vec![]] {
        let out = run(&dir, &args.iter().map(|s| *s).collect::<Vec<_>>());
        assert!(out.status.success());
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert!(stdout.contains("yfocus — focus queue CLI"));
        assert!(stdout.contains("yfocus show"));
    }
}

#[test]
fn unknown_command_fails() {
    let dir = TempDir::new().unwrap();
    let (code, stderr) = run_err(&dir, &["foobar"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("unknown command: foobar"));
}

#[test]
fn show_empty_and_path() {
    let dir = TempDir::new().unwrap();
    let out = run_ok(&dir, &["show"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["version"], 1);
    assert_eq!(v["current"], Value::Null);
    assert_eq!(v["tasks"].as_array().unwrap().len(), 0);

    let path = run_ok(&dir, &["path"]);
    assert!(path.trim().ends_with("omarchy/yfocus/queue.json"));
    assert!(path.contains(dir.path().to_string_lossy().as_ref()));
}

#[test]
fn current_empty_is_blank_line() {
    let dir = TempDir::new().unwrap();
    let out = run_ok(&dir, &["current"]);
    assert_eq!(out, "\n");
}

#[test]
fn jump_add_pop_flow() {
    let dir = TempDir::new().unwrap();
    // jump first
    let id1 = run_ok(&dir, &["jump", "first"]).trim().to_string();
    assert_eq!(id1.len(), 16);
    let q = read_queue(&dir);
    assert_eq!(q["current"].as_str().unwrap(), id1);
    assert_eq!(q["tasks"].as_array().unwrap().len(), 1);

    // add second does not steal current
    let id_still = run_ok(&dir, &["add", "second"]).trim().to_string();
    assert_eq!(id_still, id1);
    let q = read_queue(&dir);
    assert_eq!(q["tasks"].as_array().unwrap().len(), 2);
    assert_eq!(q["current"].as_str().unwrap(), id1);

    // current prints first
    let cur = run_ok(&dir, &["current"]);
    assert_eq!(cur.trim(), "first");

    // jump urgent becomes current
    let id2 = run_ok(&dir, &["jump", "urgent"]).trim().to_string();
    assert_ne!(id2, id1);
    let cur = run_ok(&dir, &["current"]);
    assert_eq!(cur.trim(), "urgent");

    // pop marks urgent done, promotes first
    let promoted = run_ok(&dir, &["pop"]).trim().to_string();
    assert_eq!(promoted, id1);
    let q = read_queue(&dir);
    assert_eq!(q["current"].as_str().unwrap(), id1);
    // completed task has doneAt
    let tasks = q["tasks"].as_array().unwrap();
    let urgent = tasks.iter().find(|t| t["id"] == id2).unwrap();
    assert!(urgent["doneAt"].is_number());
    assert_eq!(read_queue(&dir)["tasks"].as_array().unwrap().len(), 3);
}

#[test]
fn pop_no_current_is_noop() {
    let dir = TempDir::new().unwrap();
    run_ok(&dir, &["jump", "solo"]);
    run_ok(&dir, &["pop"]);
    let q = read_queue(&dir);
    assert!(q["current"].is_null());
    // second pop should succeed and print blank line
    let out = run_ok(&dir, &["pop"]);
    assert_eq!(out.trim(), "");
}

#[test]
fn remove_and_reorder() {
    let dir = TempDir::new().unwrap();
    run_ok(&dir, &["jump", "a"]);
    run_ok(&dir, &["jump", "b"]);
    run_ok(&dir, &["jump", "c"]); // current=c, queue=[b,a]
    let q = read_queue(&dir);
    let current = q["current"].as_str().unwrap().to_string();
    assert_eq!(
        q["tasks"].as_array().unwrap().iter().find(|t| t["id"] == current).unwrap()["title"],
        "c"
    );
    // find b's id
    let tasks: Vec<Value> = q["tasks"].as_array().unwrap().clone();
    let _b_id = tasks.iter().find(|t| t["title"] == "b").unwrap()["id"].as_str().unwrap().to_string();
    // reorder 1->0 swaps a,b => queue becomes [a,b]
    run_ok(&dir, &["reorder", "1", "0"]);
    let q = read_queue(&dir);
    // open queue is tasks where id != current and doneAt null, sorted by position
    let mut open: Vec<&Value> = q["tasks"].as_array().unwrap().iter().filter(|t| t["id"] != q["current"] && t["doneAt"].is_null()).collect();
    open.sort_by_key(|t| t["position"].as_u64().unwrap());
    assert_eq!(open[0]["title"], "a");
    assert_eq!(open[1]["title"], "b");

    // remove current promotes next
    run_ok(&dir, &["remove", &current]);
    let q = read_queue(&dir);
    assert_eq!(q["current"].as_str().unwrap(), open[0]["id"].as_str().unwrap());

    // remove unknown is no-op (should succeed)
    let before = read_queue(&dir);
    run_ok(&dir, &["remove", "nonexistent"]);
    assert_eq!(read_queue(&dir), before);
}

#[test]
fn set_current_and_clear_completed() {
    let dir = TempDir::new().unwrap();
    run_ok(&dir, &["jump", "cur"]);
    run_ok(&dir, &["add", "q1"]);
    run_ok(&dir, &["add", "q2"]);
    let q = read_queue(&dir);
    let q1_id = q["tasks"].as_array().unwrap().iter().find(|t| t["title"]=="q1").unwrap()["id"].as_str().unwrap().to_string();
    run_ok(&dir, &["set-current", &q1_id]);
    let q = read_queue(&dir);
    assert_eq!(q["current"].as_str().unwrap(), q1_id);
    // pop q1, then clear completed should purge it
    run_ok(&dir, &["pop"]);
    let q = read_queue(&dir);
    assert_eq!(q["tasks"].as_array().unwrap().iter().filter(|t| !t["doneAt"].is_null()).count(), 1);
    run_ok(&dir, &["clear-completed"]);
    let q = read_queue(&dir);
    assert_eq!(q["tasks"].as_array().unwrap().iter().filter(|t| !t["doneAt"].is_null()).count(), 0);
    // current should still be there
    assert!(q["current"].is_string());
}

#[test]
fn reset_wipes_queue() {
    let dir = TempDir::new().unwrap();
    run_ok(&dir, &["jump", "a"]);
    run_ok(&dir, &["add", "b"]);
    run_ok(&dir, &["reset"]);
    let q = read_queue(&dir);
    assert!(q["current"].is_null());
    assert_eq!(q["tasks"].as_array().unwrap().len(), 0);
}

#[test]
fn validation_errors() {
    let dir = TempDir::new().unwrap();
    let (_, e) = run_err(&dir, &["jump"]);
    assert!(e.contains("jump: title is required"));
    let (_, e) = run_err(&dir, &["jump", "   "]);
    assert!(e.contains("jump: title is required"));
    let (_, e) = run_err(&dir, &["add"]);
    assert!(e.contains("add: title is required"));
    let (_, e) = run_err(&dir, &["remove"]);
    assert!(e.contains("remove: id is required"));
    let (_, e) = run_err(&dir, &["set-current"]);
    assert!(e.contains("set-current: id is required"));
    let (_, e) = run_err(&dir, &["reorder", "1"]);
    assert!(e.contains("from and to indices are required"));
    let (_, e) = run_err(&dir, &["reorder", "a", "b"]);
    assert!(e.contains("indices must be integers"));
    let (_, e) = run_err(&dir, &["reorder", "0.5", "1"]);
    assert!(e.contains("indices must be integers"));
}

#[test]
fn titles_with_spaces_joined() {
    let dir = TempDir::new().unwrap();
    // rest.join(" ") behavior
    run_ok(&dir, &["jump", "hello", "world"]);
    let cur = run_ok(&dir, &["current"]);
    assert_eq!(cur.trim(), "hello world");
}

#[test]
fn corrupt_queue_reports_error() {
    let dir = TempDir::new().unwrap();
    run_ok(&dir, &["show"]); // creates file
    let p = dir.path().join("omarchy/yfocus/queue.json");
    std::fs::write(&p, "not json").unwrap();
    let (code, stderr) = run_err(&dir, &["show"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("corrupt queue.json"));
    // reset recovers
    run_ok(&dir, &["reset"]);
    let out = run_ok(&dir, &["show"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["tasks"].as_array().unwrap().len(), 0);
}
