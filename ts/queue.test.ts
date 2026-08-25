import { describe, expect, test } from "bun:test";
import {
  add,
  assertInvariants,
  clearCompleted,
  emptyQueue,
  getCompleted,
  getCurrent,
  getQueue,
  jump,
  pop,
  remove,
  reorder,
  setCurrent,
} from "./queue.ts";

const NOW = 1_700_000_000_000;

describe("emptyQueue", () => {
  test("starts empty", () => {
    const s = emptyQueue();
    expect(s.version).toBe(1);
    expect(s.current).toBeNull();
    expect(s.tasks).toHaveLength(0);
    assertInvariants(s);
  });
});

describe("add", () => {
  test("into an empty queue becomes current", () => {
    const s1 = add(emptyQueue(), "first", NOW);
    expect(s1.current).toBe(s1.tasks[0].id);
    expect(getCurrent(s1)?.title).toBe("first");
    expect(s1.tasks[0].startedAt).toBe(NOW);
    assertInvariants(s1);

    // A second add does not steal current; it queues behind.
    const s2 = add(s1, "second", NOW + 1);
    expect(s2.current).toBe(s1.current);
    expect(s2.tasks.map((t) => t.title)).toEqual(["first", "second"]);
    expect(s2.tasks[1].startedAt).toBeNull();
    assertInvariants(s2);
  });

  test("with only completed tasks left becomes current", () => {
    let s = jump(emptyQueue(), "done-early", NOW);
    s = pop(s, NOW + 1);
    expect(s.current).toBeNull();
    expect(getCompleted(s)).toHaveLength(1);

    s = add(s, "fresh start", NOW + 2);
    expect(getCurrent(s)?.title).toBe("fresh start");
    assertInvariants(s);
  });

  test("add with current leaves current untouched", () => {
    let s = jump(emptyQueue(), "current", NOW);
    s = add(s, "later", NOW + 1);
    expect(getCurrent(s)?.title).toBe("current");
    expect(getQueue(s).map((t) => t.title)).toEqual(["later"]);
    assertInvariants(s);
  });

  test("empty/whitespace title is a no-op", () => {
    const s0 = add(emptyQueue(), "real", NOW);
    const s1 = add(s0, "   ", NOW);
    expect(s1.tasks).toHaveLength(1);
    expect(s1).toBe(s0); // same reference returned
  });
});

describe("jump", () => {
  test("into empty queue becomes current", () => {
    const s = jump(emptyQueue(), "do the thing", NOW);
    expect(s.current).not.toBeNull();
    expect(getCurrent(s)?.title).toBe("do the thing");
    expect(s.tasks).toHaveLength(1);
    assertInvariants(s);
  });

  test("with current: previous current shifts to position 1", () => {
    let s = jump(emptyQueue(), "first", NOW);
    const firstId = s.current!;
    s = jump(s, "second", NOW + 1);
    expect(getCurrent(s)?.title).toBe("second");
    expect(getQueue(s)).toHaveLength(1);
    expect(getQueue(s)[0].id).toBe(firstId);
    assertInvariants(s);
  });

  test("three jumps: each previous shifts down by one, never lost", () => {
    let s = emptyQueue();
    for (const title of ["a", "b", "c"]) {
      s = jump(s, title, NOW);
    }
    expect(getCurrent(s)?.title).toBe("c");
    expect(getQueue(s).map((t) => t.title)).toEqual(["b", "a"]);
    assertInvariants(s);
  });

  test("empty title is a no-op", () => {
    const s0 = jump(emptyQueue(), "x", NOW);
    const s1 = jump(s0, "", NOW + 1);
    expect(s1).toBe(s0);
  });
});

describe("pop", () => {
  test("marks current done and promotes next", () => {
    let s = emptyQueue();
    s = jump(s, "first", NOW);
    s = jump(s, "second", NOW + 1);
    const secondId = s.current!;
    const firstId = getQueue(s)[0].id;
    s = pop(s, NOW + 2);
    expect(getCurrent(s)?.id).toBe(firstId);
    expect(getCompleted(s)[0].id).toBe(secondId);
    expect(getCompleted(s)[0].doneAt).toBe(NOW + 2);
    assertInvariants(s);
  });

  test("pop with no current is a no-op", () => {
    // Build a current-less state: pop the last open task so only
    // history remains.
    let s = jump(emptyQueue(), "solo", NOW);
    s = pop(s, NOW + 1);
    expect(s.current).toBeNull();
    const s1 = pop(s, NOW + 2);
    expect(s1).toBe(s);
  });

  test("pop when nothing queued: current becomes null", () => {
    let s = jump(emptyQueue(), "solo", NOW);
    s = pop(s, NOW + 1);
    expect(s.current).toBeNull();
    expect(getCompleted(s)).toHaveLength(1);
    assertInvariants(s);
  });
});

describe("remove", () => {
  test("removing a queued task leaves current alone", () => {
    let s = emptyQueue();
    s = jump(s, "a", NOW);
    s = jump(s, "b", NOW + 1);
    const aId = getQueue(s)[0].id;
    s = remove(s, aId);
    expect(getCurrent(s)?.title).toBe("b");
    expect(s.tasks).toHaveLength(1);
    assertInvariants(s);
  });

  test("removing current promotes position 1", () => {
    let s = emptyQueue();
    s = jump(s, "a", NOW);
    s = jump(s, "b", NOW + 1);
    const currentId = s.current!;
    const promotedId = getQueue(s)[0].id;
    s = remove(s, currentId);
    expect(s.current).toBe(promotedId);
    assertInvariants(s);
  });

  test("removing current when alone sets current to null", () => {
    let s = jump(emptyQueue(), "solo", NOW);
    const id = s.current!;
    s = remove(s, id);
    expect(s.current).toBeNull();
    expect(s.tasks).toHaveLength(0);
    assertInvariants(s);
  });

  test("removing unknown id is a no-op", () => {
    const s0 = jump(emptyQueue(), "x", NOW);
    const s1 = remove(s0, "nonexistent");
    expect(s1).toBe(s0);
  });

  test("removing a completed task works", () => {
    let s = emptyQueue();
    s = jump(s, "a", NOW);
    s = pop(s, NOW + 1);
    expect(getCompleted(s)).toHaveLength(1);
    s = remove(s, getCompleted(s)[0].id);
    expect(getCompleted(s)).toHaveLength(0);
    assertInvariants(s);
  });
});

describe("reorder", () => {
  test("moves a queued task up", () => {
    let s = emptyQueue();
    s = jump(s, "a", NOW);
    s = jump(s, "b", NOW + 1);
    s = jump(s, "c", NOW + 2);
    // queue is [b, a]
    s = reorder(s, 1, 0);
    expect(getQueue(s).map((t) => t.title)).toEqual(["a", "b"]);
    assertInvariants(s);
  });

  test("moves a queued task down", () => {
    let s = emptyQueue();
    s = add(s, "q1", NOW);
    s = add(s, "q2", NOW);
    s = jump(s, "current", NOW);
    // queue is [q1, q2]
    s = reorder(s, 0, 1);
    expect(getQueue(s).map((t) => t.title)).toEqual(["q2", "q1"]);
    assertInvariants(s);
  });

  test("out-of-bounds and non-integer are no-ops", () => {
    let s = emptyQueue();
    s = jump(s, "a", NOW);
    s = jump(s, "b", NOW + 1);
    expect(reorder(s, -1, 0)).toBe(s);
    expect(reorder(s, 0, 5)).toBe(s);
    expect(reorder(s, 5, 0)).toBe(s);
    expect(reorder(s, 0.5, 1)).toBe(s);
    expect(reorder(s, 0, 0)).toBe(s);
  });
});

describe("setCurrent", () => {
  test("promotes a queued task; previous current demotes to head of queue", () => {
    let s = emptyQueue();
    s = jump(s, "a", NOW);
    s = jump(s, "b", NOW + 1);
    s = jump(s, "c", NOW + 2);
    // current is c, queue is [b, a]
    const aId = getQueue(s)[1].id;
    const cId = s.current!;
    const bId = getQueue(s)[0].id;
    s = setCurrent(s, aId, NOW + 3);
    expect(s.current).toBe(aId);
    expect(getCurrent(s)?.id).toBe(aId);
    // Order: promoted task first (current), then demoted c, then b.
    expect(getQueue(s).map((t) => t.id)).toEqual([cId, bId]);
    assertInvariants(s);
  });

  test("promoted current sits at position 0 so pop keeps working", () => {
    let s = emptyQueue();
    s = add(s, "q1", NOW);
    s = add(s, "q2", NOW);
    s = jump(s, "active", NOW + 1);
    const q1Id = getQueue(s)[0].id;
    s = setCurrent(s, q1Id, NOW + 2);
    // Consecutive pops must not throw now that positions were rebuilt.
    expect(() => {
      s = pop(s, NOW + 3);
      s = pop(s, NOW + 4);
    }).not.toThrow();
    assertInvariants(s);
  });

  test("setting current to itself is a no-op", () => {
    let s = jump(emptyQueue(), "x", NOW);
    const id = s.current!;
    expect(setCurrent(s, id)).toBe(s);
  });

  test("cannot set current to a completed task", () => {
    let s = emptyQueue();
    s = jump(s, "a", NOW);
    s = jump(s, "b", NOW + 1);
    const bId = s.current!;
    s = pop(s, NOW + 2);
    expect(setCurrent(s, bId)).toBe(s);
  });

  test("unknown id is a no-op", () => {
    const s0 = jump(emptyQueue(), "x", NOW);
    expect(setCurrent(s0, "nope")).toBe(s0);
  });
});

describe("clearCompleted", () => {
  test("purges completed tasks, keeps open ones", () => {
    let s = emptyQueue();
    s = jump(s, "done-early", NOW);
    s = pop(s, NOW + 1);
    s = jump(s, "active", NOW + 2);
    s = add(s, "queued", NOW + 3);
    s = clearCompleted(s);
    expect(getCompleted(s)).toHaveLength(0);
    expect(s.tasks).toHaveLength(2);
    expect(getCurrent(s)?.title).toBe("active");
    expect(getQueue(s).map((t) => t.title)).toEqual(["queued"]);
    assertInvariants(s);
  });

  test("nothing completed is a no-op (same reference)", () => {
    const s0 = jump(emptyQueue(), "x", NOW);
    expect(clearCompleted(s0)).toBe(s0);
  });
});

describe("invariants", () => {
  test("assertInvariants catches missing startedAt on current", () => {
    const bad = {
      version: 1,
      current: "x",
      tasks: [
        { id: "x", title: "x", note: "", position: 0, createdAt: 0, startedAt: null, doneAt: null },
      ],
    } as any;
    expect(() => assertInvariants(bad)).toThrow(/startedAt/);
  });

  test("assertInvariants catches unknown current id", () => {
    const bad = { version: 1, current: "ghost", tasks: [] } as any;
    expect(() => assertInvariants(bad)).toThrow(/not in tasks/);
  });

  test("assertInvariants catches duplicate positions", () => {
    const bad = {
      version: 1,
      current: null,
      tasks: [
        { id: "a", title: "a", note: "", position: 0, createdAt: 0, startedAt: null, doneAt: null },
        { id: "b", title: "b", note: "", position: 0, createdAt: 0, startedAt: null, doneAt: null },
      ],
    } as any;
    expect(() => assertInvariants(bad)).toThrow(/duplicate position/);
  });

  test("assertInvariants catches wrong version", () => {
    const bad = { version: 2 as any, current: null, tasks: [] };
    expect(() => assertInvariants(bad)).toThrow(/version/);
  });
});

describe("TS <-> JS mirror parity", () => {
  test("FocusModel.js exports every operation ts/queue.ts exposes", async () => {
    // @ts-ignore — plain JS with CommonJS guard
    const jsModel = await import("../FocusModel.js");
    const mod = await import("./queue.ts");
    const sharedOps = [
      "emptyQueue",
      "getCurrent",
      "getQueue",
      "getCompleted",
      "jump",
      "add",
      "pop",
      "remove",
      "reorder",
      "setCurrent",
      "clearCompleted",
    ];
    for (const fn of sharedOps) {
      expect(typeof (mod as any)[fn]).toBe("function");
      expect(typeof (jsModel as any)[fn]).toBe("function");
    }
  });
});
