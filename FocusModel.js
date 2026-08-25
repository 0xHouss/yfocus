// QML-side mirror of ts/queue.ts. Every mutation here has a matching
// function in the TS file with the same name and semantics; the parity is
// asserted by ts/queue.test.ts (exports match) and the behavioral tests
// run against this file via tests/focusmodel.test.mjs.
//
// The overlays apply mutations in-memory first (instant UI), then fire a
// detached `yfocus` CLI call that performs the same mutation on disk under
// the store's lock. The FileView watch picks up the resulting write and
// reconciles any divergence.

var VERSION = 1

function now() {
  return Date.now()
}

function newId() {
  // 16 hex chars, same shape as the TS implementation. Not crypto-bound
  // here; ids only need to be unique within one human-scaled queue.
  var out = ""
  for (var i = 0; i < 8; i++) {
    var b = Math.floor(Math.random() * 256)
    var s = b.toString(16)
    out += s.length === 1 ? "0" + s : s
  }
  return out
}

function sortByPosition(tasks) {
  return tasks.slice().sort(function (a, b) { return a.position - b.position })
}

function reindex(tasks) {
  var sorted = sortByPosition(tasks)
  return sorted.map(function (t, i) {
    return Object.assign({}, t, { position: i })
  })
}

// Assign fresh sequential positions in array order without re-sorting.
// Mutations that build a deliberate new order must use this — sorting by
// stale positions would resurrect the old ordering (same trap as ts).
function assignPositions(tasks) {
  return tasks.map(function (t, i) {
    return Object.assign({}, t, { position: i })
  })
}

// --- constructors ------------------------------------------------------

function emptyQueue() {
  return { version: VERSION, current: null, tasks: [] }
}

// --- queries -----------------------------------------------------------

function getCurrent(state) {
  if (!state || state.current === null) return null
  for (var i = 0; i < state.tasks.length; i++) {
    if (state.tasks[i].id === state.current) return state.tasks[i]
  }
  return null
}

function getQueue(state) {
  if (!state) return []
  return sortByPosition(state.tasks).filter(function (t) {
    return t.id !== state.current && t.doneAt === null
  })
}

function getCompleted(state) {
  if (!state) return []
  return sortByPosition(state.tasks).filter(function (t) {
    return t.doneAt !== null
  })
}

// --- mutations ---------------------------------------------------------

// Jump: insert at position 0; previous current shifts to position 1;
// everything else shifts down by one. Never destructive.
function jump(state, title) {
  var trimmed = String(title || "").trim()
  if (trimmed.length === 0) return state

  var t = now()
  var inserted = {
    id: newId(),
    title: trimmed,
    note: "",
    position: 0,
    createdAt: t,
    startedAt: t,
    doneAt: null,
  }

  var shifted = state.tasks.map(function (x) {
    return Object.assign({}, x, { position: x.position + 1 })
  })

  return {
    version: VERSION,
    current: inserted.id,
    tasks: reindex([inserted].concat(shifted)),
  }
}

// Add: append to the end of the queue. Does not affect current.
function add(state, title) {
  var trimmed = String(title || "").trim()
  if (trimmed.length === 0) return state

  var t = now()
  var appended = {
    id: newId(),
    title: trimmed,
    note: "",
    position: state.tasks.length,
    createdAt: t,
    startedAt: null,
    doneAt: null,
  }

  return {
    version: VERSION,
    current: state.current,
    tasks: state.tasks.concat([appended]),
  }
}

// Pop: mark current done, promote next queued task. Completed task moves
// to the end of the task list; positions are rebuilt so consecutive pops
// keep working.
function pop(state) {
  if (!state || state.current === null) return state
  var currentTask = getCurrent(state)
  if (!currentTask) return state

  var t = now()
  var completedTask = Object.assign({}, currentTask, { doneAt: t })
  var openQueue = getQueue(state)
  var nextCurrent = openQueue[0] || null
  var remainingOpen = openQueue.slice(1)
  var completed = getCompleted(state)

  var rebuilt = []
  if (nextCurrent) {
    rebuilt.push(Object.assign({}, nextCurrent, { startedAt: nextCurrent.startedAt || t }))
  }
  rebuilt = rebuilt.concat(remainingOpen).concat([completedTask]).concat(completed)

  return {
    version: VERSION,
    current: nextCurrent ? nextCurrent.id : null,
    tasks: assignPositions(rebuilt),
  }
}

// Remove: delete by id. Removing the current promotes the head of the
// queue; removing the last task clears current.
function remove(state, id) {
  var found = false
  for (var i = 0; i < state.tasks.length; i++) {
    if (state.tasks[i].id === id) { found = true; break }
  }
  if (!found) return state

  var wasCurrent = state.current === id
  var remaining = state.tasks.filter(function (x) { return x.id !== id })

  if (!wasCurrent) {
    return Object.assign({}, state, { tasks: reindex(remaining) })
  }

  var openTasks = sortByPosition(remaining).filter(function (x) { return x.doneAt === null })
  var completedTasks = sortByPosition(remaining).filter(function (x) { return x.doneAt !== null })
  var nextCurrent = openTasks[0] || null
  var restOpen = openTasks.slice(1)

  var rebuilt = []
  if (nextCurrent) {
    rebuilt.push(Object.assign({}, nextCurrent, { startedAt: nextCurrent.startedAt || now() }))
  }
  rebuilt = rebuilt.concat(restOpen).concat(completedTasks)

  return Object.assign({}, state, {
    current: nextCurrent ? nextCurrent.id : null,
    tasks: assignPositions(rebuilt),
  })
}

// Reorder: move within the open, non-current queue view. Out-of-bounds or
// non-integer indices are no-ops.
function reorder(state, fromIndex, toIndex) {
  var queue = getQueue(state)
  if (typeof fromIndex !== "number" || typeof toIndex !== "number") return state
  if (!Number.isInteger(fromIndex) || !Number.isInteger(toIndex)) return state
  if (fromIndex < 0 || fromIndex >= queue.length) return state
  if (toIndex < 0 || toIndex >= queue.length) return state
  if (fromIndex === toIndex) return state

  var moving = queue[fromIndex]
  var without = queue.filter(function (_, i) { return i !== fromIndex })
  without.splice(toIndex, 0, moving)

  var currentTask = getCurrent(state)
  var completed = getCompleted(state)

  var rebuilt = []
  if (currentTask) rebuilt.push(currentTask)
  rebuilt = rebuilt.concat(without).concat(completed)

  return Object.assign({}, state, { tasks: assignPositions(rebuilt) })
}

// setCurrent: promote an open task by id. New current takes position 0;
// the demoted previous current lands right behind it (position 1).
function setCurrent(state, id) {
  if (!state || state.current === id) return state
  var target = null
  for (var i = 0; i < state.tasks.length; i++) {
    if (state.tasks[i].id === id) { target = state.tasks[i]; break }
  }
  if (!target || target.doneAt !== null) return state

  var previousCurrent = getCurrent(state)
  var others = state.tasks.filter(function (x) {
    return x.id !== id && x.id !== (previousCurrent ? previousCurrent.id : null)
  })
  var completed = others.filter(function (x) { return x.doneAt !== null })
  var openOthers = others.filter(function (x) { return x.doneAt === null })

  var head = [Object.assign({}, target, { startedAt: now() })]
  if (previousCurrent) {
    head.push(Object.assign({}, previousCurrent, { startedAt: null }))
  }

  return Object.assign({}, state, {
    current: id,
    tasks: assignPositions(head.concat(openOthers).concat(completed)),
  })
}

// clearCompleted: purge finished tasks from history.
function clearCompleted(state) {
  var remaining = state.tasks.filter(function (x) { return x.doneAt === null })
  if (remaining.length === state.tasks.length) return state
  return Object.assign({}, state, { tasks: reindex(remaining) })
}

// --- end ---------------------------------------------------------------

// Note: persistence deliberately lives in FocusOverlay.qml (QML scope),
// not here. Shared JS libraries have no guaranteed access to QML
// singletons like Quickshell, and an execDetached failure there would
// abort submit() before the submitted() signal could close the window.

if (typeof module !== "undefined" && module.exports) {
  module.exports = {
    VERSION: VERSION,
    emptyQueue: emptyQueue,
    getCurrent: getCurrent,
    getQueue: getQueue,
    getCompleted: getCompleted,
    jump: jump,
    add: add,
    pop: pop,
    remove: remove,
    reorder: reorder,
    setCurrent: setCurrent,
    clearCompleted: clearCompleted,
  }
}
