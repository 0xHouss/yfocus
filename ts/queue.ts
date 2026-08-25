import { QueueState, Task, EMPTY_QUEUE } from "./types.ts";

// --- helpers -------------------------------------------------------------

function sortByPosition(tasks: Task[]): Task[] {
  return [...tasks].sort((a, b) => a.position - b.position);
}

function reindex(tasks: Task[]): Task[] {
  const sorted = sortByPosition(tasks);
  return sorted.map((t, i) => ({ ...t, position: i }));
}

// Assign fresh sequential positions in the given array order. Unlike
// reindex, this never re-sorts: callers that build an array in a deliberate
// new order (pop, reorder, setCurrent) must use this, or stale positions
// would resurrect the old ordering during the sort.
function assignPositions(tasks: Task[]): Task[] {
  return tasks.map((t, i) => ({ ...t, position: i }));
}

function newId(): string {
  // 16 random hex chars; collision-free at human scales without crypto deps.
  return [...crypto.getRandomValues(new Uint8Array(8))]
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

// --- constructors --------------------------------------------------------

export function emptyQueue(): QueueState {
  return { ...EMPTY_QUEUE, tasks: [] };
}

export function fromTasks(tasks: Task[], current: string | null = null): QueueState {
  return { version: 1, current, tasks: reindex(tasks) };
}

// --- queries -------------------------------------------------------------

export function getCurrent(state: QueueState): Task | null {
  if (state.current === null) return null;
  return state.tasks.find((t) => t.id === state.current) ?? null;
}

export function getQueue(state: QueueState): Task[] {
  // open tasks, ordered, excluding current and completed.
  return sortByPosition(state.tasks).filter(
    (t) => t.id !== state.current && t.doneAt === null,
  );
}

export function getCompleted(state: QueueState): Task[] {
  return sortByPosition(state.tasks).filter((t) => t.doneAt !== null);
}

// --- mutations -----------------------------------------------------------

/**
 * Jump: insert a new task at position 0.
 * - If a task is current, it shifts to position 1.
 * - All other tasks shift down by one.
 * - The new task becomes current with startedAt = now.
 * - Never destructive.
 */
export function jump(state: QueueState, title: string, now: number = Date.now()): QueueState {
  const trimmed = title.trim();
  if (trimmed.length === 0) return state;

  const inserted: Task = {
    id: newId(),
    title: trimmed,
    note: "",
    position: 0,
    createdAt: now,
    startedAt: now,
    doneAt: null,
  };

  // All existing tasks move down by one position. The previous current
  // (if any) keeps its slot semantics by virtue of position shifting.
  const shifted = state.tasks.map((t) => ({ ...t, position: t.position + 1 }));

  return {
    version: 1,
    current: inserted.id,
    tasks: reindex([inserted, ...shifted]),
  };
}

/**
 * Add: append a new task to the end of the queue.
 * - Does not affect current, EXCEPT when nothing is current (empty queue,
 *   or only completed tasks left): then the added task starts right away.
 */
export function add(state: QueueState, title: string, now: number = Date.now()): QueueState {
  const trimmed = title.trim();
  if (trimmed.length === 0) return state;

  const becameCurrent = state.current === null;
  const appended: Task = {
    id: newId(),
    title: trimmed,
    note: "",
    position: state.tasks.length,
    createdAt: now,
    startedAt: becameCurrent ? now : null,
    doneAt: null,
  };

  return {
    ...state,
    current: becameCurrent ? appended.id : state.current,
    tasks: [...state.tasks, appended],
  };
}

/**
 * Pop: mark the current task done and promote the task at position 1 to
 * current.
 * - If there is no current, returns the state unchanged.
 * - The completed task moves to the end (completed section); the promoted
 *   task takes position 0 so consecutive pops keep working.
 */
export function pop(state: QueueState, now: number = Date.now()): QueueState {
  if (state.current === null) return state;

  const currentTask = getCurrent(state);
  if (!currentTask) return state;

  const completedTask = { ...currentTask, doneAt: now };
  const openQueue = getQueue(state);
  const nextCurrent = openQueue[0] ?? null;
  const remainingOpen = openQueue.slice(1);
  const completed = getCompleted(state);

  const rebuilt: Task[] = [];
  if (nextCurrent) {
    rebuilt.push({ ...nextCurrent, startedAt: nextCurrent.startedAt ?? now });
  }
  rebuilt.push(...remainingOpen, completedTask, ...completed);

  return {
    version: 1,
    current: nextCurrent ? nextCurrent.id : null,
    tasks: assignPositions(rebuilt),
  };
}

/**
 * Remove: delete a task by id.
 * - If the removed task was current, promote the task at position 1 to
 *   current. If none, current becomes null.
 * - Completed tasks are removed as-is; positions reindex.
 */
export function remove(state: QueueState, id: string): QueueState {
  if (!state.tasks.some((t) => t.id === id)) return state;

  const wasCurrent = state.current === id;
  const remaining = state.tasks.filter((t) => t.id !== id);

  if (!wasCurrent) {
    return { ...state, tasks: reindex(remaining) };
  }

  const reindexed = reindex(remaining);
  // Promote the new position 0 (which was position 1 before removal).
  const promoted = reindexed.find((t) => t.position === 0 && t.doneAt === null);
  if (!promoted) {
    return { ...state, tasks: reindexed, current: null };
  }
  return {
    ...state,
    tasks: reindexed.map((t) =>
      t.id === promoted.id ? { ...t, startedAt: t.startedAt ?? Date.now() } : t,
    ),
    current: promoted.id,
  };
}

/**
 * Reorder: move a task to a new position among open, non-current tasks.
 * Positions are 0-based over the open-and-non-current queue view
 * (current does not participate in reorder). Indices passed to this
 * function are positions in that view. Out-of-bounds is a no-op.
 */
export function reorder(
  state: QueueState,
  fromIndex: number,
  toIndex: number,
): QueueState {
  const queue = getQueue(state);
  if (!Number.isInteger(fromIndex) || !Number.isInteger(toIndex)) return state;
  if (fromIndex < 0 || fromIndex >= queue.length) return state;
  if (toIndex < 0 || toIndex >= queue.length) return state;
  if (fromIndex === toIndex) return state;

  const moving = queue[fromIndex];
  const without = queue.filter((_, i) => i !== fromIndex);
  without.splice(toIndex, 0, moving);

  // Rebuild tasks: current stays at position 0, reordered queue follows,
  // completed tasks keep their relative order at the end.
  const currentTask = getCurrent(state);
  const completed = getCompleted(state);

  const rebuilt: Task[] = [];
  if (currentTask) rebuilt.push(currentTask);
  rebuilt.push(...without, ...completed);
  return { ...state, tasks: assignPositions(rebuilt) };
}

/**
 * setCurrent: pick any open task by id and make it current.
 * - The previous current becomes the head of the queue (position 1).
 * - Already-current id is a no-op.
 * - Completed targets are refused (no-op).
 */
export function setCurrent(state: QueueState, id: string, now: number = Date.now()): QueueState {
  if (state.current === id) return state;
  const target = state.tasks.find((t) => t.id === id);
  if (!target || target.doneAt !== null) return state;

  const previousCurrent = getCurrent(state);
  const others = state.tasks.filter(
    (t) => t.id !== id && t.id !== (previousCurrent?.id ?? null),
  );
  const completed = others.filter((t) => t.doneAt !== null);
  const openOthers = others.filter((t) => t.doneAt === null);

  // New current takes position 0; the demoted task lands right behind it
  // (position 1) with startedAt cleared so it reads as "not started".
  const head: Task[] = [{ ...target, startedAt: now }];
  if (previousCurrent) {
    head.push({ ...previousCurrent, startedAt: null });
  }

  return {
    ...state,
    current: id,
    tasks: assignPositions([...head, ...openOthers, ...completed]),
  };
}

/**
 * clearCompleted: purge all finished tasks from history.
 * No-op when nothing is completed.
 */
export function clearCompleted(state: QueueState): QueueState {
  const remaining = state.tasks.filter((t) => t.doneAt === null);
  if (remaining.length === state.tasks.length) return state;
  return { ...state, tasks: reindex(remaining) };
}

// --- validation ----------------------------------------------------------

/**
 * Invariants are checked by `assertInvariants` on every test, and on
 * every read/write in the store layer.
 */
export function assertInvariants(state: QueueState): void {
  if (state.version !== 1) {
    throw new Error(`version must be 1, got ${state.version}`);
  }
  if (!Array.isArray(state.tasks)) {
    throw new Error(`tasks must be an array`);
  }
  if (state.current !== null && !state.tasks.some((t) => t.id === state.current)) {
    throw new Error(`current id ${state.current} not in tasks`);
  }
  const currentTask = state.tasks.find((t) => t.id === state.current);
  if (currentTask && currentTask.doneAt !== null) {
    throw new Error(`current task is marked done`);
  }
  const positions = new Set<number>();
  for (const t of state.tasks) {
    if (positions.has(t.position)) {
      throw new Error(`duplicate position ${t.position}`);
    }
    positions.add(t.position);
  }
  if (positions.size !== state.tasks.length) {
    throw new Error(`positions are not contiguous`);
  }
  for (let i = 0; i < state.tasks.length; i++) {
    if (!positions.has(i)) {
      throw new Error(`missing position ${i}`);
    }
  }
  if (currentTask && currentTask.startedAt === null) {
    throw new Error(`current task has no startedAt`);
  }
}
