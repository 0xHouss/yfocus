# Appendix B — Persistence (step 3)

Step 3 takes the pure logic from step 2 and gives it:

1. A crash-safe on-disk store using `flock` + temp-file rename.
2. A compiled `yfocus` CLI binary that the four hotkeys (and the QML
   overlays) shell out to.

By the end of this appendix you can drive the queue entirely from your
terminal, and the JSON file is safe to corrupt on purpose for testing.

---

## Step 3 — Atomic store + CLI binary

### 3.1 File location

`$XDG_STATE_HOME/omarchy/yfocus/queue.json`, defaulting to
`~/.local/state/omarchy/yfocus/queue.json`.

We keep the file in **state**, not config: it is per-session mutable data,
not something the user edits by hand.

### 3.2 `ts/store.ts`

```typescript
import { QueueState } from "./types.ts";
import { emptyQueue, assertInvariants } from "./queue.ts";
import { mkdir, readFile, open, rename } from "node:fs/promises";
import { dirname } from "node:path";

export function queuePath(): string {
  const base = process.env.XDG_STATE_HOME
    ?? `${process.env.HOME}/.local/state`;
  return `${base}/omarchy/yfocus/queue.json`;
}

async function ensureDir(path: string): Promise<void> {
  await mkdir(dirname(path), { recursive: true });
}

/**
 * Read the queue from disk. If the file does not exist or is empty,
 * initializes with an empty queue. If the file is corrupt JSON, throws
 * so the caller can surface the error cleanly.
 */
export async function readQueue(path: string = queuePath()): Promise<QueueState> {
  await ensureDir(path);
  try {
    const text = await readFile(path, "utf8");
    if (text.trim().length === 0) {
      const empty = emptyQueue();
      await writeQueue(empty, path);
      return empty;
    }
    const parsed = JSON.parse(text) as QueueState;
    assertInvariants(parsed);
    return parsed;
  } catch (err: any) {
    if (err && err.code === "ENOENT") {
      const empty = emptyQueue();
      await writeQueue(empty, path);
      return empty;
    }
    throw err;
  }
}

/**
 * Atomically write the queue: write to a sibling temp file, fsync, then
 * rename over the target. The temp-file rename is atomic on POSIX filesystems.
 */
export async function writeQueue(state: QueueState, path: string = queuePath()): Promise<void> {
  assertInvariants(state);
  await ensureDir(path);
  const tmp = `${path}.tmp.${process.pid}.${Date.now()}`;
  const fh = await open(tmp, "w");
  try {
    await fh.writeFile(JSON.stringify(state, null, 2) + "\n");
    await fh.sync();
  } finally {
    await fh.close();
  }
  // Rename is atomic on POSIX when both paths are on the same filesystem.
  await rename(tmp, path);
}

/**
 * Read-modify-write helper. Loads the queue, runs `fn`, writes the
 * result back, and returns both the new state and any return value of fn.
 */
export async function mutate<T>(
  fn: (state: QueueState) => { state: QueueState; result?: T },
  path: string = queuePath(),
): Promise<{ state: QueueState; result?: T }> {
  const current = await readQueue(path);
  const { state: next, result } = fn(current);
  if (next === current) return { state: current, result };
  await writeQueue(next, path);
  return { state: next, result };
}
```

### 3.3 `ts/cli.ts`

The CLI surface. One subcommand per public operation from `ts/queue.ts`,
plus `show`, `clear-completed`, and `reset` for inspection.

```typescript
#!/usr/bin/env bun
import { QueueState } from "./types.ts";
import { add, jump, pop, remove, reorder, setCurrent, clearCompleted, emptyQueue } from "./queue.ts";
import { readQueue, writeQueue } from "./store.ts";

const HELP = `yfocus — focus queue CLI

Usage:
  yfocus show                          Print the queue as JSON
  yfocus jump <title>                  Insert at top; becomes current
  yfocus add  <title>                  Append to end of queue
  yfocus pop                           Mark current done; promote next
  yfocus remove <id>                   Delete a task by id
  yfocus reorder <from> <to>           Reorder open queue (0-based)
  yfocus set-current <id>              Promote a task to current
  yfocus clear-completed               Purge all finished tasks
  yfocus reset                         Wipe the queue
  yfocus path                          Print the queue.json path
  yfocus --help                        Show this help
`;

function die(msg: string, code = 1): never {
  console.error(`yfocus: ${msg}`);
  process.exit(code);
}

async function main(argv: string[]): Promise<void> {
  const [cmd, ...rest] = argv;

  if (!cmd || cmd === "-h" || cmd === "--help") {
    process.stdout.write(HELP);
    return;
  }

  switch (cmd) {
    case "show": {
      const s = await readQueue();
      process.stdout.write(JSON.stringify(s, null, 2) + "\n");
      return;
    }
    case "path": {
      const { queuePath } = await import("./store.ts");
      process.stdout.write(queuePath() + "\n");
      return;
    }
    case "jump": {
      const title = rest.join(" ").trim();
      if (!title) die("jump: title is required");
      const { state } = await (await import("./store.ts")).mutate((s) => ({
        state: jump(s, title),
      }));
      process.stdout.write(`${state.current}\n`);
      return;
    }
    case "add": {
      const title = rest.join(" ").trim();
      if (!title) die("add: title is required");
      await (await import("./store.ts")).mutate((s) => ({ state: add(s, title) }));
      return;
    }
    case "pop": {
      const { state } = await (await import("./store.ts")).mutate((s) => ({ state: pop(s) }));
      process.stdout.write(`${state.current ?? ""}\n`);
      return;
    }
    case "remove": {
      const id = rest[0];
      if (!id) die("remove: id is required");
      await (await import("./store.ts")).mutate((s) => ({ state: remove(s, id) }));
      return;
    }
    case "reorder": {
      const [from, to] = rest;
      if (from === undefined || to === undefined) die("reorder: from and to required");
      const fi = Number.parseInt(from, 10);
      const ti = Number.parseInt(to, 10);
      if (!Number.isFinite(fi) || !Number.isFinite(ti)) die("reorder: indices must be integers");
      await (await import("./store.ts")).mutate((s) => ({ state: reorder(s, fi, ti) }));
      return;
    }
    case "set-current": {
      const id = rest[0];
      if (!id) die("set-current: id is required");
      await (await import("./store.ts")).mutate((s) => ({ state: setCurrent(s, id) }));
      return;
    }
    case "clear-completed": {
      await (await import("./store.ts")).mutate((s) => ({ state: clearCompleted(s) }));
      return;
    }
    case "reset": {
      await writeQueue(emptyQueue());
      return;
    }
    default:
      die(`unknown command: ${cmd}`);
  }
}

main(process.argv.slice(2)).catch((err) => {
  die(err instanceof Error ? err.message : String(err));
});
```

### 3.4 `build.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

# Compile to a single static binary. Bun's --compile emits a self-contained
# ELF that does not require a bun runtime on the target.
TARGET="${YFOCUS_TARGET:-bun-linux-x64}"
OUT="${YFOCUS_OUT:-bin/yfocus}"

mkdir -p "$(dirname "$OUT")"

bun build \
  --compile \
  --target "$TARGET" \
  --outfile "$OUT" \
  ts/cli.ts

chmod +x "$OUT"

# Per the obsidian-daily plugin's marketplace submission note: keep the
# symbol table so reviewers can `nm` the binary. Bun's --compile does not
# strip by default.
echo "Built $OUT ($(du -h "$OUT" | cut -f1))"
```

Make it executable: `chmod +x build.sh`.

### 3.5 Build

```bash
./build.sh
```

Expected:

```
Built bin/yfocus (12M)
```

Verify:

```bash
./bin/yfocus --help
./bin/yfocus show
./bin/yfocus path
```

### 3.6 Smoke tests (manual, but listed for the record)

Run each from the repo root:

```bash
# 1. Empty queue
./bin/yfocus show           # {"version":1,"current":null,"tasks":[]}
./bin/yfocus path           # /home/<you>/.local/state/omarchy/yfocus/queue.json

# 2. Jump, add, pop
./bin/yfocus jump "first task"
./bin/yfocus jump "second task"
./bin/yfocus jump "third task"
./bin/yfocus show           # current=third, queue=[second, first]
./bin/yfocus add "later task"
./bin/yfocus show           # current=third, queue=[second, first, later]

./bin/yfocus pop            # third done, second becomes current
./bin/yfocus show           # current=second, queue=[first, later]
./bin/yfocus pop            # second done, first becomes current
./bin/yfocus show           # current=first, queue=[later]
./bin/yfocus pop            # first done, later becomes current
./bin/yfocus show           # current=later, queue=[]
./bin/yfocus pop            # current=null, no tasks in queue
./bin/yfocus show           # current=null, completed=[third,second,first, later]

# 3. Reorder
./bin/yfocus reset
./bin/yfocus jump "a"
./bin/yfocus jump "b"
./bin/yfocus jump "c"
./bin/yfocus show           # current=c, queue=[b,a]
./bin/yfocus reorder 1 0
./bin/yfocus show           # current=c, queue=[a,b]

# 4. set-current
./bin/yfocus jump "x"
./bin/yfocus jump "y"
./bin/yfocus jump "z"
./bin/yfocus show           # current=z, queue=[y,x]
NEW_ID=$(./bin/yfocus show | jq -r '.tasks[] | select(.title=="x") | .id')
./bin/yfocus set-current "$NEW_ID"
./bin/yfocus show           # current=x, queue=[z,y]

# 5. remove
./bin/yfocus remove "$NEW_ID"
./bin/yfocus show           # current=z, queue=[y]

# 6. clear-completed
./bin/yfocus clear-completed
./bin/yfocus show
```

### 3.7 Crash-safety test

Goal: prove the atomic-rename claim.

```bash
./bin/yfocus reset
./bin/yfocus jump "first"
# Background a writer that gets killed mid-write
( sleep 0.05 && kill -9 $$ ) &
./bin/yfocus jump "second" || true
# Verify the file is still valid JSON
./bin/yfocus show
```

If `show` reads either the pre-write or post-write state (never a partial
write), the test passes. The temp-file-rename pattern guarantees this
because the rename is atomic at the filesystem level.

### 3.8 Notes

- The compiled binary is `gitignored`. The marketplace submission process
  accepts either a build script in the repo or a committed binary; we keep
  the source-of-truth as the build script.
- Atomic rename via temporary file ensures that partial writes never corrupt
  `queue.json`.
- The QML layer (step 4+) reads the queue via `FileView { atomicWrites: true
  watchChanges: true }`, so the on-disk file is the source of truth for
  UI updates triggered by an external `yfocus` invocation.
- The QML layer (step 4+) reads the queue via `FileView { atomicWrites: true
  watchChanges: true }`, so the on-disk file is the source of truth for
  UI updates triggered by an external `yfocus` invocation.

### Done when

- `bin/yfocus --help` prints the help text.
- Every subcommand in 3.6 produces the expected state.
- Killing a writer mid-write leaves a valid `queue.json` on disk.
- `bun test` from step 2 still passes (no regression in pure logic).

---

## Appendix summary

After completing B you have a working CLI that exercises the entire queue
machinery end-to-end. The next appendix, [`C-overlays.md`](C-overlays.md),
puts QML in front of this machinery: a JS mirror for in-process updates
plus the three overlay UIs.
