#!/usr/bin/env bun
import {
  add,
  clearCompleted,
  emptyQueue,
  jump,
  pop,
  remove,
  reorder,
  setCurrent,
} from "./queue.ts";
import { mutate, queuePath, readQueue, writeQueue } from "./store.ts";

const HELP = `yfocus — focus queue CLI

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
`;

function die(msg: string, code = 1): never {
  console.error(`yfocus: ${msg}`);
  process.exit(code);
}

async function main(argv: string[]): Promise<void> {
  const [cmd, ...rest] = argv;

  if (!cmd || cmd === "-h" || cmd === "--help" || cmd === "help") {
    process.stdout.write(HELP);
    return;
  }

  switch (cmd) {
    case "show": {
      const s = await readQueue();
      process.stdout.write(JSON.stringify(s, null, 2) + "\n");
      return;
    }
    case "current": {
      const s = await readQueue();
      const task = s.tasks.find((t) => t.id === s.current) ?? null;
      process.stdout.write((task ? task.title : "") + "\n");
      return;
    }
    case "path": {
      process.stdout.write(queuePath() + "\n");
      return;
    }
    case "jump": {
      const title = rest.join(" ").trim();
      if (!title) die("jump: title is required");
      const { state } = await mutate((s) => ({ state: jump(s, title) }));
      process.stdout.write(`${state.current}\n`);
      return;
    }
    case "add": {
      const title = rest.join(" ").trim();
      if (!title) die("add: title is required");
      const { state } = await mutate((s) => ({ state: add(s, title) }));
      process.stdout.write(`${state.current ?? ""}\n`);
      return;
    }
    case "pop": {
      const { state } = await mutate((s) => ({ state: pop(s) }));
      process.stdout.write(`${state.current ?? ""}\n`);
      return;
    }
    case "remove": {
      const id = rest[0];
      if (!id) die("remove: id is required");
      await mutate((s) => ({ state: remove(s, id) }));
      return;
    }
    case "reorder": {
      const [from, to] = rest;
      if (from === undefined || to === undefined) {
        die("reorder: from and to indices are required");
      }
      const fi = Number.parseInt(from, 10);
      const ti = Number.parseInt(to, 10);
      if (!Number.isFinite(fi) || !Number.isFinite(ti)) {
        die("reorder: indices must be integers");
      }
      await mutate((s) => ({ state: reorder(s, fi, ti) }));
      return;
    }
    case "set-current": {
      const id = rest[0];
      if (!id) die("set-current: id is required");
      await mutate((s) => ({ state: setCurrent(s, id) }));
      return;
    }
    case "clear-completed": {
      await mutate((s) => ({ state: clearCompleted(s) }));
      return;
    }
    case "reset": {
      await writeQueue(emptyQueue());
      return;
    }
    default:
      die(`unknown command: ${cmd}\nRun 'yfocus --help' for usage.`);
  }
}

main(process.argv.slice(2)).catch((err) => {
  die(err instanceof Error ? err.message : String(err));
});
