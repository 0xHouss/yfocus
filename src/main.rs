mod queue;
mod store;
mod types;

use store::{mutate, queue_path, read_queue, write_queue};
use types::QueueState;

const HELP: &str = r#"yfocus — focus queue CLI

Usage:
  yfocus show                          Print the queue as JSON
  yfocus current                       Print the current task title
  yfocus jump <title>                  Insert at top; becomes current
  yfocus add <title>                   Append to end of queue
  yfocus pop                           Mark current done; promote next
  yfocus remove <id>                   Delete a task by id
  yfocus reorder <from> <to>           Reorder open queue (0-based)
  yfocus set-current <id>              Promote a queued task to current
  yfocus clear-completed               Purge all finished tasks
  yfocus reset                         Wipe the queue
  yfocus path                          Print the queue.json path
  yfocus --help                        Show this help

Vocabulary:
  jump   insert at position 0 (previous current shifts to position 1)
  add    append at the end of the queue
  pop    mark current done and promote the next queued task
"#;

fn die(msg: &str, code: i32) -> ! {
    eprintln!("yfocus: {}", msg);
    std::process::exit(code);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.get(0).map(|s| s.as_str()).unwrap_or("");
    let rest = if args.len() > 1 { &args[1..] } else { &[] };

    if cmd.is_empty() || cmd == "-h" || cmd == "--help" || cmd == "help" {
        print!("{}", HELP);
        return;
    }

    match cmd {
        "show" => {
            let s = read_queue().unwrap_or_else(|e| die(&e, 1));
            let json = serde_json::to_string_pretty(&s).unwrap();
            println!("{}", json);
        }
        "current" => {
            let s = read_queue().unwrap_or_else(|e| die(&e, 1));
            let title = s
                .tasks
                .iter()
                .find(|t| Some(&t.id) == s.current.as_ref())
                .map(|t| t.title.as_str())
                .unwrap_or("");
            println!("{}", title);
        }
        "path" => {
            println!("{}", queue_path().display());
        }
        "jump" => {
            let title = rest.join(" ").trim().to_string();
            if title.is_empty() {
                die("jump: title is required", 1);
            }
            let state = mutate(|s| queue::jump(s, &title)).unwrap_or_else(|e| die(&e, 1));
            println!("{}", state.current.unwrap_or_default());
        }
        "add" => {
            let title = rest.join(" ").trim().to_string();
            if title.is_empty() {
                die("add: title is required", 1);
            }
            let state = mutate(|s| queue::add(s, &title)).unwrap_or_else(|e| die(&e, 1));
            println!("{}", state.current.unwrap_or_default());
        }
        "pop" => {
            let state = mutate(|s| queue::pop(s)).unwrap_or_else(|e| die(&e, 1));
            println!("{}", state.current.unwrap_or_default());
        }
        "remove" => {
            let id = rest.get(0).map(|s| s.as_str()).unwrap_or("");
            if id.is_empty() {
                die("remove: id is required", 1);
            }
            mutate(|s| queue::remove(s, id)).unwrap_or_else(|e| die(&e, 1));
        }
        "reorder" => {
            if rest.len() < 2 {
                die("reorder: from and to indices are required", 1);
            }
            let from_str = &rest[0];
            let to_str = &rest[1];
            // Parse as f64 first to detect floats like 0.5, then validate integer.
            let from_f: f64 = from_str.parse().unwrap_or(f64::NAN);
            let to_f: f64 = to_str.parse().unwrap_or(f64::NAN);
            if from_f.is_nan() || to_f.is_nan() || from_f.fract() != 0.0 || to_f.fract() != 0.0 {
                die("reorder: indices must be integers", 1);
            }
            let fi = from_f as i64;
            let ti = to_f as i64;
            mutate(|s| queue::reorder(s, fi, ti)).unwrap_or_else(|e| die(&e, 1));
        }
        "set-current" => {
            let id = rest.get(0).map(|s| s.as_str()).unwrap_or("");
            if id.is_empty() {
                die("set-current: id is required", 1);
            }
            mutate(|s| queue::set_current(s, id)).unwrap_or_else(|e| die(&e, 1));
        }
        "clear-completed" => {
            mutate(|s| queue::clear_completed(s)).unwrap_or_else(|e| die(&e, 1));
        }
        "reset" => {
            write_queue(&QueueState::empty()).unwrap_or_else(|e| die(&e, 1));
        }
        _ => {
            die(
                &format!("unknown command: {}\nRun 'yfocus --help' for usage.", cmd),
                1,
            );
        }
    }
}
