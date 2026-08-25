// Behavioral parity tests for FocusModel.js (the QML-side mirror).
// These mirror the core scenarios of ts/queue.test.ts; if both suites
// pass, the two implementations agree.
import { describe, expect, test } from "bun:test";
import Model from "../FocusModel.js";

const NOW = 1_700_000_000_000;

describe("FocusModel.js mirrors ts/queue.ts", () => {
  test("emptyQueue", () => {
    const s = Model.emptyQueue();
    expect(s.version).toBe(1);
    expect(s.current).toBeNull();
    expect(s.tasks).toHaveLength(0);
  });

  test("jump into empty becomes current", () => {
    const s = Model.jump(Model.emptyQueue(), "thing");
    expect(Model.getCurrent(s)?.title).toBe("thing");
  });

  test("jump shifts previous current to head of queue", () => {
    let s = Model.jump(Model.emptyQueue(), "first");
    const firstId = s.current;
    s = Model.jump(s, "second");
    expect(Model.getCurrent(s)?.title).toBe("second");
    expect(Model.getQueue(s).map((t) => t.id)).toEqual([firstId]);
  });

  test("add appends without touching current", () => {
    let s = Model.jump(Model.emptyQueue(), "cur");
    s = Model.add(s, "later");
    expect(Model.getCurrent(s)?.title).toBe("cur");
    expect(Model.getQueue(s).map((t) => t.title)).toEqual(["later"]);
  });

  test("pop marks done, promotes next, consecutive pops work", () => {
    let s = Model.emptyQueue();
    s = Model.jump(s, "a");
    s = Model.jump(s, "b"); // current=b, queue=[a]
    const bId = s.current;
    const aId = Model.getQueue(s)[0].id;
    s = Model.pop(s);
    expect(Model.getCurrent(s)?.id).toBe(aId);
    expect(Model.getCompleted(s)[0].id).toBe(bId);
    // positions rebuilt so another pop works
    s = Model.pop(s);
    expect(s.current).toBeNull();
    expect(Model.getCompleted(s)).toHaveLength(2);
  });

  test("remove current promotes queue head", () => {
    let s = Model.emptyQueue();
    s = Model.jump(s, "a");
    s = Model.jump(s, "b");
    const curId = s.current;
    const nextId = Model.getQueue(s)[0].id;
    s = Model.remove(s, curId);
    expect(s.current).toBe(nextId);
  });

  test("reorder moves within queue view", () => {
    let s = Model.emptyQueue();
    s = Model.jump(s, "a");
    s = Model.jump(s, "b");
    s = Model.jump(s, "c"); // current=c, queue=[b,a]
    s = Model.reorder(s, 1, 0);
    expect(Model.getQueue(s).map((t) => t.title)).toEqual(["a", "b"]);
  });

  test("setCurrent promotes target to position 0, demotes previous", () => {
    let s = Model.emptyQueue();
    s = Model.add(s, "q1");
    s = Model.add(s, "q2");
    s = Model.jump(s, "active"); // current=active, queue=[q1,q2]
    const q1Id = Model.getQueue(s)[0].id;
    const activeId = s.current;
    s = Model.setCurrent(s, q1Id);
    expect(s.current).toBe(q1Id);
    // new current at position 0, demoted previous right behind
    expect(s.tasks[0].id).toBe(q1Id);
    expect(s.tasks[1].id).toBe(activeId);
    // consecutive pops still work
    s = Model.pop(s);
    s = Model.pop(s);
    // q1 done, active done -> q2 promoted, queue never corrupted
    expect(Model.getCurrent(s)?.title).toBe("q2");
  });

  test("clearCompleted purges only finished tasks", () => {
    let s = Model.emptyQueue();
    s = Model.jump(s, "done-early");
    s = Model.pop(s);
    s = Model.jump(s, "active");
    s = Model.clearCompleted(s);
    expect(Model.getCompleted(s)).toHaveLength(0);
    expect(Model.getCurrent(s)?.title).toBe("active");
  });

  test("blank titles are no-ops (same reference)", () => {
    const s0 = Model.jump(Model.emptyQueue(), "x");
    expect(Model.jump(s0, "   ")).toBe(s0);
    expect(Model.add(s0, "")).toBe(s0);
  });
});
