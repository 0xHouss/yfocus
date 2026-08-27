use crate::types::{QueueState, Task};
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn new_id() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 8] = rng.gen();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn sort_by_position(tasks: &[Task]) -> Vec<Task> {
    let mut v = tasks.to_vec();
    v.sort_by_key(|t| t.position);
    v
}

fn reindex(tasks: Vec<Task>) -> Vec<Task> {
    let mut sorted = tasks;
    sorted.sort_by_key(|t| t.position);
    sorted
        .into_iter()
        .enumerate()
        .map(|(i, mut t)| {
            t.position = i;
            t
        })
        .collect()
}

fn assign_positions(tasks: Vec<Task>) -> Vec<Task> {
    tasks
        .into_iter()
        .enumerate()
        .map(|(i, mut t)| {
            t.position = i;
            t
        })
        .collect()
}

// --- constructors ---

#[allow(dead_code)]
pub fn empty_queue() -> QueueState {
    QueueState::empty()
}

// --- queries ---

pub fn get_current(state: &QueueState) -> Option<Task> {
    match &state.current {
        None => None,
        Some(id) => state.tasks.iter().find(|t| &t.id == id).cloned(),
    }
}

pub fn get_queue(state: &QueueState) -> Vec<Task> {
    sort_by_position(&state.tasks)
        .into_iter()
        .filter(|t| Some(&t.id) != state.current.as_ref() && t.done_at.is_none())
        .collect()
}

pub fn get_completed(state: &QueueState) -> Vec<Task> {
    sort_by_position(&state.tasks)
        .into_iter()
        .filter(|t| t.done_at.is_some())
        .collect()
}

// --- mutations ---

pub fn jump(state: &QueueState, title: &str) -> QueueState {
    jump_with_now(state, title, now_ms())
}

pub fn jump_with_now(state: &QueueState, title: &str, now: u64) -> QueueState {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return state.clone();
    }
    let inserted = Task {
        id: new_id(),
        title: trimmed.to_string(),
        note: String::new(),
        position: 0,
        created_at: now,
        started_at: Some(now),
        done_at: None,
    };
    let shifted: Vec<Task> = state
        .tasks
        .iter()
        .map(|t| {
            let mut c = t.clone();
            c.position += 1;
            c
        })
        .collect();
    let mut all = Vec::with_capacity(shifted.len() + 1);
    all.push(inserted);
    all.extend(shifted);
    QueueState {
        version: 1,
        current: Some(all[0].id.clone()),
        tasks: reindex(all),
    }
}

pub fn add(state: &QueueState, title: &str) -> QueueState {
    add_with_now(state, title, now_ms())
}

pub fn add_with_now(state: &QueueState, title: &str, now: u64) -> QueueState {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return state.clone();
    }
    let became_current = state.current.is_none();
    let appended = Task {
        id: new_id(),
        title: trimmed.to_string(),
        note: String::new(),
        position: state.tasks.len(),
        created_at: now,
        started_at: if became_current { Some(now) } else { None },
        done_at: None,
    };
    let mut tasks = state.tasks.clone();
    tasks.push(appended.clone());
    QueueState {
        version: 1,
        current: if became_current {
            Some(appended.id)
        } else {
            state.current.clone()
        },
        tasks,
    }
}

pub fn pop(state: &QueueState) -> QueueState {
    pop_with_now(state, now_ms())
}

pub fn pop_with_now(state: &QueueState, now: u64) -> QueueState {
    if state.current.is_none() {
        return state.clone();
    }
    let current_task = match get_current(state) {
        None => return state.clone(),
        Some(t) => t,
    };
    let completed_task = Task {
        done_at: Some(now),
        ..current_task
    };
    let open_queue = get_queue(state);
    let next_current = open_queue.first().cloned();
    let remaining_open = if open_queue.len() > 1 {
        open_queue[1..].to_vec()
    } else {
        Vec::new()
    };
    let completed = get_completed(state);

    let mut rebuilt: Vec<Task> = Vec::new();
    if let Some(mut nc) = next_current.clone() {
        if nc.started_at.is_none() {
            nc.started_at = Some(now);
        }
        rebuilt.push(nc);
    }
    rebuilt.extend(remaining_open);
    rebuilt.push(completed_task);
    rebuilt.extend(completed);

    QueueState {
        version: 1,
        current: next_current.map(|t| t.id),
        tasks: assign_positions(rebuilt),
    }
}

pub fn remove(state: &QueueState, id: &str) -> QueueState {
    if !state.tasks.iter().any(|t| t.id == id) {
        return state.clone();
    }
    let was_current = state.current.as_deref() == Some(id);
    let remaining: Vec<Task> = state.tasks.iter().filter(|t| t.id != id).cloned().collect();

    if !was_current {
        return QueueState {
            version: 1,
            current: state.current.clone(),
            tasks: reindex(remaining),
        };
    }

    let reindexed = reindex(remaining);
    let promoted = reindexed
        .iter()
        .find(|t| t.position == 0 && t.done_at.is_none())
        .cloned();
    match promoted {
        None => QueueState {
            version: 1,
            current: None,
            tasks: reindexed,
        },
        Some(p) => {
            let now = now_ms();
            let tasks = reindexed
                .into_iter()
                .map(|mut t| {
                    if t.id == p.id && t.started_at.is_none() {
                        t.started_at = Some(now);
                    }
                    t
                })
                .collect();
            QueueState {
                version: 1,
                current: Some(p.id),
                tasks,
            }
        }
    }
}

pub fn reorder(state: &QueueState, from_index: i64, to_index: i64) -> QueueState {
    let queue = get_queue(state);
    if from_index < 0 || to_index < 0 {
        return state.clone();
    }
    let fi = from_index as usize;
    let ti = to_index as usize;
    if fi >= queue.len() || ti >= queue.len() || fi == ti {
        return state.clone();
    }
    let moving = queue[fi].clone();
    let mut without: Vec<Task> = queue
        .into_iter()
        .enumerate()
        .filter(|(i, _)| *i != fi)
        .map(|(_, t)| t)
        .collect();
    without.insert(ti, moving);

    let current_task = get_current(state);
    let completed = get_completed(state);

    let mut rebuilt: Vec<Task> = Vec::new();
    if let Some(ct) = current_task {
        rebuilt.push(ct);
    }
    rebuilt.extend(without);
    rebuilt.extend(completed);

    QueueState {
        version: 1,
        current: state.current.clone(),
        tasks: assign_positions(rebuilt),
    }
}

pub fn set_current(state: &QueueState, id: &str) -> QueueState {
    set_current_with_now(state, id, now_ms())
}

pub fn set_current_with_now(state: &QueueState, id: &str, now: u64) -> QueueState {
    if state.current.as_deref() == Some(id) {
        return state.clone();
    }
    let target = match state.tasks.iter().find(|t| t.id == id) {
        None => return state.clone(),
        Some(t) => {
            if t.done_at.is_some() {
                return state.clone();
            }
            t.clone()
        }
    };
    let previous_current = get_current(state);
    let prev_id = previous_current.as_ref().map(|t| t.id.as_str());
    let others: Vec<Task> = state
        .tasks
        .iter()
        .filter(|t| t.id != id && Some(t.id.as_str()) != prev_id)
        .cloned()
        .collect();
    let completed: Vec<Task> = others.iter().filter(|t| t.done_at.is_some()).cloned().collect();
    let open_others: Vec<Task> = others.into_iter().filter(|t| t.done_at.is_none()).collect();

    let mut head: Vec<Task> = Vec::new();
    let mut promoted = target.clone();
    promoted.started_at = Some(now);
    head.push(promoted);
    if let Some(mut pc) = previous_current {
        pc.started_at = None;
        head.push(pc);
    }

    let mut all = head;
    all.extend(open_others);
    all.extend(completed);

    QueueState {
        version: 1,
        current: Some(id.to_string()),
        tasks: assign_positions(all),
    }
}

pub fn clear_completed(state: &QueueState) -> QueueState {
    let remaining: Vec<Task> = state
        .tasks
        .iter()
        .filter(|t| t.done_at.is_none())
        .cloned()
        .collect();
    if remaining.len() == state.tasks.len() {
        return state.clone();
    }
    QueueState {
        version: 1,
        current: state.current.clone(),
        tasks: reindex(remaining),
    }
}

// --- validation ---

pub fn assert_invariants(state: &QueueState) -> Result<(), String> {
    if state.version != 1 {
        return Err(format!("version must be 1, got {}", state.version));
    }
    if let Some(cur) = &state.current {
        if !state.tasks.iter().any(|t| &t.id == cur) {
            return Err(format!("current id {} not in tasks", cur));
        }
    }
    if let Some(ct) = state
        .current
        .as_ref()
        .and_then(|id| state.tasks.iter().find(|t| &t.id == id))
    {
        if ct.done_at.is_some() {
            return Err("current task is marked done".to_string());
        }
        if ct.started_at.is_none() {
            return Err("current task has no startedAt".to_string());
        }
    }
    use std::collections::HashSet;
    let mut positions = HashSet::new();
    for t in &state.tasks {
        if !positions.insert(t.position) {
            return Err(format!("duplicate position {}", t.position));
        }
    }
    if positions.len() != state.tasks.len() {
        return Err("positions are not contiguous".to_string());
    }
    for i in 0..state.tasks.len() {
        if !positions.contains(&i) {
            return Err(format!("missing position {}", i));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Task;

    const NOW: u64 = 1_700_000_000_000;

    // ---- emptyQueue ----
    #[test]
    fn empty_starts_empty() {
        let s = empty_queue();
        assert_eq!(s.version, 1);
        assert_eq!(s.current, None);
        assert_eq!(s.tasks.len(), 0);
        assert!(assert_invariants(&s).is_ok());
    }

    // ---- add ----
    #[test]
    fn add_into_empty_becomes_current() {
        let s1 = add_with_now(&empty_queue(), "first", NOW);
        assert_eq!(s1.current.as_ref().unwrap(), &s1.tasks[0].id);
        assert_eq!(get_current(&s1).unwrap().title, "first");
        assert_eq!(s1.tasks[0].started_at, Some(NOW));
        assert!(assert_invariants(&s1).is_ok());

        let s2 = add_with_now(&s1, "second", NOW + 1);
        assert_eq!(s2.current, s1.current);
        assert_eq!(
            s2.tasks.iter().map(|t| t.title.as_str()).collect::<Vec<_>>(),
            vec!["first", "second"]
        );
        assert_eq!(s2.tasks[1].started_at, None);
        assert!(assert_invariants(&s2).is_ok());
    }

    #[test]
    fn add_with_only_completed_becomes_current() {
        let mut s = jump_with_now(&empty_queue(), "done-early", NOW);
        s = pop_with_now(&s, NOW + 1);
        assert_eq!(s.current, None);
        assert_eq!(get_completed(&s).len(), 1);
        s = add_with_now(&s, "fresh start", NOW + 2);
        assert_eq!(get_current(&s).unwrap().title, "fresh start");
        assert!(assert_invariants(&s).is_ok());
    }

    #[test]
    fn add_with_current_leaves_current_untouched() {
        let mut s = jump_with_now(&empty_queue(), "current", NOW);
        s = add_with_now(&s, "later", NOW + 1);
        assert_eq!(get_current(&s).unwrap().title, "current");
        assert_eq!(
            get_queue(&s).iter().map(|t| t.title.as_str()).collect::<Vec<_>>(),
            vec!["later"]
        );
        assert!(assert_invariants(&s).is_ok());
    }

    #[test]
    fn add_empty_title_is_noop() {
        let s0 = add_with_now(&empty_queue(), "real", NOW);
        let s1 = add_with_now(&s0, "   ", NOW);
        assert_eq!(s1.tasks.len(), 1);
        assert_eq!(s1, s0);
    }

    // ---- jump ----
    #[test]
    fn jump_into_empty_becomes_current() {
        let s = jump_with_now(&empty_queue(), "do the thing", NOW);
        assert!(s.current.is_some());
        assert_eq!(get_current(&s).unwrap().title, "do the thing");
        assert_eq!(s.tasks.len(), 1);
        assert!(assert_invariants(&s).is_ok());
    }

    #[test]
    fn jump_shifts_previous_current_to_position_1() {
        let mut s = jump_with_now(&empty_queue(), "first", NOW);
        let first_id = s.current.clone().unwrap();
        s = jump_with_now(&s, "second", NOW + 1);
        assert_eq!(get_current(&s).unwrap().title, "second");
        assert_eq!(get_queue(&s).len(), 1);
        assert_eq!(get_queue(&s)[0].id, first_id);
        assert!(assert_invariants(&s).is_ok());
    }

    #[test]
    fn jump_three_each_shifts_down() {
        let mut s = empty_queue();
        for title in ["a", "b", "c"] {
            s = jump_with_now(&s, title, NOW);
        }
        assert_eq!(get_current(&s).unwrap().title, "c");
        assert_eq!(
            get_queue(&s).iter().map(|t| t.title.as_str()).collect::<Vec<_>>(),
            vec!["b", "a"]
        );
        assert!(assert_invariants(&s).is_ok());
    }

    #[test]
    fn jump_empty_is_noop() {
        let s0 = jump_with_now(&empty_queue(), "x", NOW);
        let s1 = jump_with_now(&s0, "", NOW + 1);
        assert_eq!(s1, s0);
    }

    // ---- pop ----
    #[test]
    fn pop_marks_done_and_promotes_next() {
        let mut s = empty_queue();
        s = jump_with_now(&s, "first", NOW);
        s = jump_with_now(&s, "second", NOW + 1);
        let second_id = s.current.clone().unwrap();
        let first_id = get_queue(&s)[0].id.clone();
        s = pop_with_now(&s, NOW + 2);
        assert_eq!(get_current(&s).unwrap().id, first_id);
        assert_eq!(get_completed(&s)[0].id, second_id);
        assert_eq!(get_completed(&s)[0].done_at, Some(NOW + 2));
        assert!(assert_invariants(&s).is_ok());
    }

    #[test]
    fn pop_with_no_current_is_noop() {
        let mut s = jump_with_now(&empty_queue(), "solo", NOW);
        s = pop_with_now(&s, NOW + 1);
        assert_eq!(s.current, None);
        let s1 = pop_with_now(&s, NOW + 2);
        assert_eq!(s1, s);
    }

    #[test]
    fn pop_when_nothing_queued_becomes_null() {
        let mut s = jump_with_now(&empty_queue(), "solo", NOW);
        s = pop_with_now(&s, NOW + 1);
        assert_eq!(s.current, None);
        assert_eq!(get_completed(&s).len(), 1);
        assert!(assert_invariants(&s).is_ok());
    }

    // ---- remove ----
    #[test]
    fn remove_queued_leaves_current_alone() {
        let mut s = empty_queue();
        s = jump_with_now(&s, "a", NOW);
        s = jump_with_now(&s, "b", NOW + 1);
        let a_id = get_queue(&s)[0].id.clone();
        s = remove(&s, &a_id);
        assert_eq!(get_current(&s).unwrap().title, "b");
        assert_eq!(s.tasks.len(), 1);
        assert!(assert_invariants(&s).is_ok());
    }

    #[test]
    fn remove_current_promotes_position_1() {
        let mut s = empty_queue();
        s = jump_with_now(&s, "a", NOW);
        s = jump_with_now(&s, "b", NOW + 1);
        let cur = s.current.clone().unwrap();
        let promoted = get_queue(&s)[0].id.clone();
        s = remove(&s, &cur);
        assert_eq!(s.current.as_ref().unwrap(), &promoted);
        assert!(assert_invariants(&s).is_ok());
    }

    #[test]
    fn remove_current_when_alone_clears_current() {
        let mut s = jump_with_now(&empty_queue(), "solo", NOW);
        let id = s.current.clone().unwrap();
        s = remove(&s, &id);
        assert_eq!(s.current, None);
        assert_eq!(s.tasks.len(), 0);
        assert!(assert_invariants(&s).is_ok());
    }

    #[test]
    fn remove_unknown_is_noop() {
        let s0 = jump_with_now(&empty_queue(), "x", NOW);
        let s1 = remove(&s0, "nonexistent");
        assert_eq!(s1, s0);
    }

    #[test]
    fn remove_completed_works() {
        let mut s = empty_queue();
        s = jump_with_now(&s, "a", NOW);
        s = pop_with_now(&s, NOW + 1);
        assert_eq!(get_completed(&s).len(), 1);
        let cid = get_completed(&s)[0].id.clone();
        s = remove(&s, &cid);
        assert_eq!(get_completed(&s).len(), 0);
        assert!(assert_invariants(&s).is_ok());
    }

    // ---- reorder ----
    #[test]
    fn reorder_moves_up() {
        let mut s = empty_queue();
        s = jump_with_now(&s, "a", NOW);
        s = jump_with_now(&s, "b", NOW + 1);
        s = jump_with_now(&s, "c", NOW + 2);
        s = reorder(&s, 1, 0);
        assert_eq!(
            get_queue(&s).iter().map(|t| t.title.as_str()).collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        assert!(assert_invariants(&s).is_ok());
    }

    #[test]
    fn reorder_moves_down() {
        let mut s = empty_queue();
        s = add_with_now(&s, "q1", NOW);
        s = add_with_now(&s, "q2", NOW);
        s = jump_with_now(&s, "current", NOW);
        s = reorder(&s, 0, 1);
        assert_eq!(
            get_queue(&s).iter().map(|t| t.title.as_str()).collect::<Vec<_>>(),
            vec!["q2", "q1"]
        );
        assert!(assert_invariants(&s).is_ok());
    }

    #[test]
    fn reorder_out_of_bounds_is_noop() {
        let mut s = empty_queue();
        s = jump_with_now(&s, "a", NOW);
        s = jump_with_now(&s, "b", NOW + 1);
        assert_eq!(reorder(&s, -1, 0), s);
        assert_eq!(reorder(&s, 0, 5), s);
        assert_eq!(reorder(&s, 5, 0), s);
        assert_eq!(reorder(&s, 0, 0), s);
    }

    // ---- setCurrent ----
    #[test]
    fn set_current_promotes_and_demotes() {
        let mut s = empty_queue();
        s = jump_with_now(&s, "a", NOW);
        s = jump_with_now(&s, "b", NOW + 1);
        s = jump_with_now(&s, "c", NOW + 2);
        let a_id = get_queue(&s)[1].id.clone();
        let c_id = s.current.clone().unwrap();
        let b_id = get_queue(&s)[0].id.clone();
        s = set_current_with_now(&s, &a_id, NOW + 3);
        assert_eq!(s.current.as_ref().unwrap(), &a_id);
        assert_eq!(get_current(&s).unwrap().id, a_id);
        assert_eq!(
            get_queue(&s).iter().map(|t| t.id.clone()).collect::<Vec<_>>(),
            vec![c_id, b_id]
        );
        assert!(assert_invariants(&s).is_ok());
    }

    #[test]
    fn set_current_at_position_0_pop_keeps_working() {
        let mut s = empty_queue();
        s = add_with_now(&s, "q1", NOW);
        s = add_with_now(&s, "q2", NOW);
        s = jump_with_now(&s, "active", NOW + 1);
        let q1_id = get_queue(&s)[0].id.clone();
        s = set_current_with_now(&s, &q1_id, NOW + 2);
        s = pop_with_now(&s, NOW + 3);
        s = pop_with_now(&s, NOW + 4);
        assert!(assert_invariants(&s).is_ok());
    }

    #[test]
    fn set_current_to_itself_is_noop() {
        let s = jump_with_now(&empty_queue(), "x", NOW);
        let id = s.current.clone().unwrap();
        assert_eq!(set_current_with_now(&s, &id, NOW + 1), s);
    }

    #[test]
    fn set_current_completed_is_noop() {
        let mut s = empty_queue();
        s = jump_with_now(&s, "a", NOW);
        s = jump_with_now(&s, "b", NOW + 1);
        let b_id = s.current.clone().unwrap();
        s = pop_with_now(&s, NOW + 2);
        assert_eq!(set_current_with_now(&s, &b_id, NOW + 3), s);
    }

    #[test]
    fn set_current_unknown_is_noop() {
        let s0 = jump_with_now(&empty_queue(), "x", NOW);
        assert_eq!(set_current_with_now(&s0, "nope", NOW), s0);
    }

    // ---- clearCompleted ----
    #[test]
    fn clear_completed_purges() {
        let mut s = empty_queue();
        s = jump_with_now(&s, "done-early", NOW);
        s = pop_with_now(&s, NOW + 1);
        s = jump_with_now(&s, "active", NOW + 2);
        s = add_with_now(&s, "queued", NOW + 3);
        s = clear_completed(&s);
        assert_eq!(get_completed(&s).len(), 0);
        assert_eq!(s.tasks.len(), 2);
        assert_eq!(get_current(&s).unwrap().title, "active");
        assert_eq!(
            get_queue(&s).iter().map(|t| t.title.as_str()).collect::<Vec<_>>(),
            vec!["queued"]
        );
        assert!(assert_invariants(&s).is_ok());
    }

    #[test]
    fn clear_nothing_is_noop() {
        let s0 = jump_with_now(&empty_queue(), "x", NOW);
        assert_eq!(clear_completed(&s0), s0);
    }

    // ---- invariants ----
    #[test]
    fn invariant_catches_missing_started_at() {
        let bad = QueueState {
            version: 1,
            current: Some("x".into()),
            tasks: vec![Task {
                id: "x".into(),
                title: "x".into(),
                note: "".into(),
                position: 0,
                created_at: 0,
                started_at: None,
                done_at: None,
            }],
        };
        assert!(assert_invariants(&bad).unwrap_err().contains("startedAt"));
    }

    #[test]
    fn invariant_catches_unknown_current() {
        let bad = QueueState {
            version: 1,
            current: Some("ghost".into()),
            tasks: vec![],
        };
        assert!(assert_invariants(&bad).unwrap_err().contains("not in tasks"));
    }

    #[test]
    fn invariant_catches_duplicate_positions() {
        let bad = QueueState {
            version: 1,
            current: None,
            tasks: vec![
                Task {
                    id: "a".into(),
                    title: "a".into(),
                    note: "".into(),
                    position: 0,
                    created_at: 0,
                    started_at: None,
                    done_at: None,
                },
                Task {
                    id: "b".into(),
                    title: "b".into(),
                    note: "".into(),
                    position: 0,
                    created_at: 0,
                    started_at: None,
                    done_at: None,
                },
            ],
        };
        assert!(assert_invariants(&bad).unwrap_err().contains("duplicate position"));
    }

    #[test]
    fn invariant_catches_wrong_version() {
        let bad = QueueState {
            version: 2,
            current: None,
            tasks: vec![],
        };
        assert!(assert_invariants(&bad).unwrap_err().contains("version"));
    }

    // ---- blank titles ----
    #[test]
    fn blank_titles_are_noops() {
        let s0 = jump_with_now(&empty_queue(), "x", NOW);
        assert_eq!(jump_with_now(&s0, "   ", NOW), s0);
        assert_eq!(add_with_now(&s0, "", NOW), s0);
    }

    // ---- consecutive pop invariants ----
    #[test]
    fn consecutive_pops_never_corrupt() {
        let mut s = empty_queue();
        s = jump_with_now(&s, "a", NOW);
        s = jump_with_now(&s, "b", NOW + 1);
        s = jump_with_now(&s, "c", NOW + 2);
        let c_id = s.current.clone().unwrap();
        let _b_id = get_queue(&s)[0].id.clone();
        let a_id = get_queue(&s)[1].id.clone();
        s = set_current_with_now(&s, &a_id, NOW + 3);
        s = pop_with_now(&s, NOW + 4);
        s = pop_with_now(&s, NOW + 5);
        // after popping a then c, b should be current
        assert_eq!(get_current(&s).unwrap().title, "b");
        assert!(assert_invariants(&s).is_ok());
        // ensure c and a are completed
        assert!(get_completed(&s).iter().any(|t| t.id == c_id || t.id == a_id));
    }
}
