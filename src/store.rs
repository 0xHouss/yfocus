use crate::queue::assert_invariants;
use crate::types::QueueState;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn queue_path() -> PathBuf {
    if let Ok(base) = std::env::var("XDG_STATE_HOME") {
        if !base.is_empty() {
            return PathBuf::from(base).join("omarchy/yfocus-queue/queue.json");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".local/state/omarchy/yfocus-queue/queue.json")
}

fn lock_dir() -> PathBuf {
    lock_dir_for(&queue_path())
}

fn lock_dir_for(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.lock", path.display()))
}

const LOCK_STALE_MS: u64 = 5000;
const LOCK_POLL_MS: u64 = 25;
const LOCK_TIMEOUT_MS: u64 = 4000;

#[allow(dead_code)]
fn lock_is_stale() -> bool {
    lock_is_stale_for(&lock_dir())
}

fn lock_is_stale_for(lock: &Path) -> bool {
    let marker = lock.join("owner");
    match fs::metadata(&marker) {
        Ok(meta) => {
            if let Ok(mtime) = meta.modified() {
                if let Ok(dur) = SystemTime::now().duration_since(mtime) {
                    return dur.as_millis() as u64 > LOCK_STALE_MS;
                }
            }
            true
        }
        Err(_) => true,
    }
}

#[allow(dead_code)]
fn acquire_lock() -> Result<(), String> {
    acquire_lock_for(&lock_dir())
}

fn acquire_lock_for(lock: &Path) -> Result<(), String> {
    let deadline = SystemTime::now() + Duration::from_millis(LOCK_TIMEOUT_MS);
    // ensure parent exists so create_dir doesn't ENOENT
    if let Some(parent) = lock.parent() {
        let _ = fs::create_dir_all(parent);
    }
    loop {
        match fs::create_dir(lock) {
            Ok(_) => {
                let owner = lock.join("owner");
                let pid = std::process::id();
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                let content = format!("{}@{}", pid, now);
                // ignore write error? just best effort
                let _ = fs::write(&owner, content);
                return Ok(());
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if SystemTime::now() > deadline {
                    if lock_is_stale_for(lock) {
                        let _ = fs::remove_dir_all(lock);
                        continue;
                    }
                    return Err("yfocus: could not acquire queue lock".to_string());
                }
                thread::sleep(Duration::from_millis(LOCK_POLL_MS));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
}

#[allow(dead_code)]
fn release_lock() {
    let _ = fs::remove_dir_all(lock_dir());
}

fn release_lock_for(lock: &Path) {
    let _ = fs::remove_dir_all(lock);
}

fn ensure_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn read_queue() -> Result<QueueState, String> {
    read_queue_at(&queue_path())
}

pub fn read_queue_at(path: &Path) -> Result<QueueState, String> {
    ensure_dir(path)?;
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let empty = QueueState::empty();
            write_queue_at(&empty, path)?;
            return Ok(empty);
        }
        Err(e) => return Err(e.to_string()),
    };
    if text.trim().is_empty() {
        let empty = QueueState::empty();
        write_queue_at(&empty, path)?;
        return Ok(empty);
    }
    let parsed: QueueState =
        serde_json::from_str(&text).map_err(|e| format!("corrupt queue.json: {}", e))?;
    assert_invariants(&parsed).map_err(|e| e)?;
    Ok(parsed)
}

pub fn write_queue(state: &QueueState) -> Result<(), String> {
    write_queue_at(state, &queue_path())
}

pub fn write_queue_at(state: &QueueState, path: &Path) -> Result<(), String> {
    assert_invariants(state).map_err(|e| e)?;
    ensure_dir(path)?;
    let pid = std::process::id();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let tmp = format!("{}.tmp.{}.{}", path.display(), pid, now);
    let tmp_path = PathBuf::from(&tmp);
    {
        let mut f = fs::File::create(&tmp_path).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())? + "\n";
        f.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    fs::rename(&tmp_path, path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn mutate<F>(f: F) -> Result<QueueState, String>
where
    F: FnOnce(&QueueState) -> QueueState,
{
    let path = queue_path();
    mutate_at(f, &path)
}

pub fn mutate_at<F>(f: F, path: &Path) -> Result<QueueState, String>
where
    F: FnOnce(&QueueState) -> QueueState,
{
    let lock = lock_dir_for(path);
    acquire_lock_for(&lock)?;
    let result = (|| -> Result<QueueState, String> {
        let current = read_queue_at(path)?;
        let next = f(&current);
        // no-op detection via equality (mirrors JS reference check + invariant)
        if next == current {
            return Ok(current);
        }
        write_queue_at(&next, path)?;
        Ok(next)
    })();
    release_lock_for(&lock);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::{add_with_now, assert_invariants, jump_with_now};

    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn tmp_path(dir: &TempDir) -> PathBuf {
        dir.path().join("queue.json")
    }

    #[test]
    fn read_missing_creates_empty() {
        let dir = TempDir::new().unwrap();
        let p = tmp_path(&dir);
        let s = read_queue_at(&p).unwrap();
        assert_eq!(s.current, None);
        assert_eq!(s.tasks.len(), 0);
        assert!(p.exists());
        assert!(assert_invariants(&s).is_ok());
    }

    #[test]
    fn read_empty_file_creates_empty() {
        let dir = TempDir::new().unwrap();
        let p = tmp_path(&dir);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, "   \n").unwrap();
        let s = read_queue_at(&p).unwrap();
        assert_eq!(s.current, None);
        assert!(assert_invariants(&s).is_ok());
    }

    #[test]
    fn read_corrupt_errors() {
        let dir = TempDir::new().unwrap();
        let p = tmp_path(&dir);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, "not json").unwrap();
        let err = read_queue_at(&p).unwrap_err();
        assert!(err.contains("corrupt queue.json"));
    }

    #[test]
    fn read_violated_invariants_errors() {
        let dir = TempDir::new().unwrap();
        let p = tmp_path(&dir);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        // current points to ghost
        let bad = r#"{"version":1,"current":"ghost","tasks":[]}"#;
        fs::write(&p, bad).unwrap();
        let err = read_queue_at(&p).unwrap_err();
        assert!(err.contains("not in tasks"));
    }

    #[test]
    fn write_and_read_roundtrip() {
        let dir = TempDir::new().unwrap();
        let p = tmp_path(&dir);
        let mut s = crate::types::QueueState::empty();
        s = jump_with_now(&s, "hello", 1000);
        write_queue_at(&s, &p).unwrap();
        let s2 = read_queue_at(&p).unwrap();
        assert_eq!(s, s2);
    }

    #[test]
    fn write_rejects_bad_invariants() {
        let dir = TempDir::new().unwrap();
        let p = tmp_path(&dir);
        let bad = crate::types::QueueState {
            version: 2,
            current: None,
            tasks: vec![],
        };
        let err = write_queue_at(&bad, &p).unwrap_err();
        assert!(err.contains("version"));
    }

    #[test]
    fn mutate_persists_and_noop_skips_write() {
        let dir = TempDir::new().unwrap();
        let p = tmp_path(&dir);
        // first mutate: add
        let s1 = mutate_at(|s| jump_with_now(s, "first", 1000), &p).unwrap();
        assert!(s1.current.is_some());
        let mtime1 = fs::metadata(&p).unwrap().modified().unwrap();
        // no-op mutate: empty title => same state, should not rewrite file
        std::thread::sleep(std::time::Duration::from_millis(10));
        let s2 = mutate_at(|s| add_with_now(s, "   ", 1001), &p).unwrap();
        assert_eq!(s1, s2);
        let mtime2 = fs::metadata(&p).unwrap().modified().unwrap();
        assert_eq!(mtime1, mtime2);
        // mutating add with real title does change file
        let s3 = mutate_at(|s| add_with_now(s, "second", 1002), &p).unwrap();
        assert_eq!(s3.tasks.len(), 2);
        let s4 = read_queue_at(&p).unwrap();
        assert_eq!(s3, s4);
    }

    #[test]
    fn queue_path_respects_xdg() {
        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap();
        // SAFETY: guarded by mutex, single-threaded access
        unsafe {
            std::env::set_var("XDG_STATE_HOME", "/tmp/test-xdg");
        }
        assert_eq!(
            queue_path(),
            PathBuf::from("/tmp/test-xdg/omarchy/yfocus-queue/queue.json")
        );
        unsafe {
            std::env::remove_var("XDG_STATE_HOME");
        }
        // fallback contains .local/state
        let p = queue_path();
        assert!(p.to_string_lossy().contains(".local/state"));
    }

    #[test]
    fn atomic_write_uses_pretty_json_with_newline() {
        let dir = TempDir::new().unwrap();
        let p = tmp_path(&dir);
        let s = jump_with_now(&crate::types::QueueState::empty(), "t", 123);
        write_queue_at(&s, &p).unwrap();
        let text = fs::read_to_string(&p).unwrap();
        assert!(text.ends_with('\n'));
        assert!(text.contains("\"version\": 1"));
        // no stale tmp files
        let entries = fs::read_dir(dir.path()).unwrap().count();
        assert_eq!(entries, 1); // only queue.json
    }

    #[test]
    fn invariant_catches_current_done_via_write() {
        let dir = TempDir::new().unwrap();
        let p = tmp_path(&dir);
        let mut s = jump_with_now(&crate::types::QueueState::empty(), "x", 1);
        // manually mark done while still current
        s.tasks[0].done_at = Some(2);
        let err = write_queue_at(&s, &p).unwrap_err();
        assert!(err.contains("marked done"));
    }
}
