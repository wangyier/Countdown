import test from "node:test";
import assert from "node:assert/strict";
import { DEFAULT_CONFIG, validateSchedule, formatCountdown } from "../src/time.js";
import { previewStats } from "../src/preview.js";
import { attachWindowDrag } from "../src/drag.js";

test("reminder settings validate separately from working hours", () => {
  assert.equal(DEFAULT_CONFIG.reminder_enabled, true);
  assert.equal(DEFAULT_CONFIG.reminder_time, "10:35");
  for (const reminder_time of ["00:00", "08:00", "23:59"]) {
    assert.doesNotThrow(() => validateSchedule({ ...DEFAULT_CONFIG, reminder_time }));
  }
  for (const reminder_time of ["", "9:00", "24:00", "10:60"]) {
    assert.throws(() => validateSchedule({ ...DEFAULT_CONFIG, reminder_time }));
  }
});

const stats = (date, config = DEFAULT_CONFIG) => previewStats(config, new Date(date.replace(" ", "T")));

test("native dragging accepts left mouse presses and preserves interactive controls", () => {
  let handler;
  let calls = 0;
  const errors = [];
  const region = {
    addEventListener(type, listener) { assert.equal(type, "mousedown"); handler = listener; },
    removeEventListener(type, listener) { assert.equal(type, "mousedown"); assert.equal(listener, handler); handler = null; },
  };
  const detach = attachWindowDrag(region, { startDragging() { calls++; }, onError(error) { errors.push(error); } });
  const mouse = (overrides = {}) => ({ button: 0, buttons: 1, detail: 1, defaultPrevented: false,
    target: { closest: () => null }, preventDefault() { this.defaultPrevented = true; }, ...overrides });
  const normal = mouse();
  handler(normal);
  assert.equal(calls, 1);
  assert.equal(normal.defaultPrevented, true);
  for (const event of [mouse({ button: 2, buttons: 2 }), mouse({ detail: 2 }), mouse({ defaultPrevented: true }), mouse({ target: { closest: () => ({ tagName: "BUTTON" }) } })]) {
    handler(event);
  }
  assert.equal(calls, 1);
  assert.deepEqual(errors, []);
  detach();
  assert.equal(handler, null);
});

test("native drag failures are reported instead of silently ignored", async () => {
  let handler;
  const errors = [];
  const region = { addEventListener(type, listener) { handler = listener; }, removeEventListener() {} };
  const failure = new Error("drag denied");
  attachWindowDrag(region, { startDragging: () => Promise.reject(failure), onError: (error) => errors.push(error) });
  handler({ button: 0, buttons: 1, detail: 1, target: { closest: () => null }, preventDefault() {} });
  await Promise.resolve();
  assert.deepEqual(errors, [failure]);
});

test("default schedule has seven effective hours and countdown includes lunch", () => {
  const [start, lunchStart, lunchEnd, end] = validateSchedule(DEFAULT_CONFIG);
  assert.equal(lunchStart - start + end - lunchEnd, 7 * 3600);
  assert.equal(stats("2026-11-11 09:00:00").remaining_seconds, 8.5 * 3600);
});

test("workday boundary states and exact final second", () => {
  for (const [time, state, remaining] of [
    ["08:59:59", "before_work", 30601], ["09:00:00", "working", 30600],
    ["11:44:59", "working", 20701], ["11:45:00", "lunch_break", 20700],
    ["13:14:59", "lunch_break", 15301], ["13:15:00", "working", 15300],
    ["17:29:59", "working", 1], ["17:30:00", "after_work", 0], ["23:59:59", "after_work", 0],
  ]) {
    const result = stats(`2026-11-11 ${time}`);
    assert.equal(result.status, state, time);
    assert.equal(result.remaining_seconds, remaining, time);
  }
});

test("lunch freezes progress while countdown continues", () => {
  const start = stats("2026-11-11 11:45:00");
  const end = stats("2026-11-11 13:15:00");
  assert.equal(start.progress_percent, end.progress_percent);
  assert.equal(start.remaining_seconds - end.remaining_seconds, 90 * 60);
  assert.ok(stats("2026-11-11 13:15:01").progress_percent > end.progress_percent);
});

test("guandan counts down to 11:45, stays at zero after arrival and resets next workday", () => {
  for (const [time, remaining] of [["00:00:00", 42300], ["09:00:00", 9900], ["11:44:59", 1], ["11:45:00", 0], ["23:59:59", 0]]) {
    assert.equal(stats(`2026-11-11 ${time}`).guandan_remaining_seconds, remaining, time);
  }
  assert.equal(stats("2026-11-12 00:00:00").guandan_remaining_seconds, 42300);
  assert.equal(stats("2026-11-29 09:00:00").guandan_remaining_seconds, 9900);
  assert.equal(stats("2026-11-11 11:44:59", { ...DEFAULT_CONFIG, lunch_start_time: "12:00" }).guandan_remaining_seconds, 901);
});

test("guandan arrival message is active from 11:45 up to 13:15 exclusively", () => {
  for (const [time, ready] of [["11:44:59", false], ["11:45:00", true], ["13:14:59", true], ["13:15:00", false], ["13:15:01", false], ["23:59:59", false]]) {
    assert.equal(stats(`2026-11-11 ${time}`).guandan_ready, ready, time);
  }
  assert.equal(stats("2026-11-29 12:00:00").guandan_ready, true);
  assert.equal(stats("2026-11-12 00:00:00").guandan_ready, false);
  assert.equal(stats("2026-11-11 13:15:00", { ...DEFAULT_CONFIG, lunch_end_time: "14:00" }).guandan_ready, true);
});

test("weekends and holidays use the same daily schedule", () => {
  assert.equal(stats("2026-11-28 12:00:00").status, "lunch_break");
  for (const date of ["2026-11-21", "2026-11-29", "2026-09-26", "2026-10-01", "2026-02-14", "2026-01-04"]) {
    const result = stats(`${date} 12:00:00`);
    assert.equal(result.status, "lunch_break", date);
    assert.equal(result.remaining_seconds, 19800, date);
    assert.equal(result.progress_percent, 9900 / 25200 * 100, date);
  }
});

test("midnight and year changes reset without a calendar; leap days work", () => {
  assert.equal(stats("2026-11-30 23:59:59").progress_percent, 100);
  assert.equal(stats("2026-12-01 00:00:00").progress_percent, 0);
  assert.equal(stats("2026-12-26 09:00:00").status, "working");
  assert.equal(stats("2027-01-01 09:00:00").status, "working");
  assert.equal(stats("2028-02-29 09:00:00").status, "working");
});

test("invalid and overnight schedules cannot produce invalid progress", () => {
  for (const end_time of ["08:00", "13:15", "25:00", "9:00", "", "17:30:00"]) {
    assert.throws(() => validateSchedule({ ...DEFAULT_CONFIG, end_time }), undefined, end_time);
  }
  assert.throws(() => validateSchedule({ ...DEFAULT_CONFIG, lunch_start_time: "13:15" }));
  assert.deepEqual(formatCountdown(-1), ["00", "00", "00"]);
  assert.deepEqual(formatCountdown(30601), ["08", "30", "01"]);
});

test("all seconds of a workday remain bounded; progress only grows during work", () => {
  let previous;
  for (let second = 0; second < 86400; second++) {
    const now = new Date(2026, 10, 11, 0, 0, second);
    const current = previewStats(DEFAULT_CONFIG, now);
    assert.ok(current.progress_percent >= 0 && current.progress_percent <= 100);
    assert.ok(current.remaining_seconds >= 0);
    if (previous) {
      assert.ok(current.progress_percent >= previous.progress_percent);
      assert.ok(current.remaining_seconds <= previous.remaining_seconds);
      if (current.status === "lunch_break" && previous.status === "lunch_break") {
        assert.equal(current.progress_percent, previous.progress_percent);
      }
    }
    previous = current;
  }
});
