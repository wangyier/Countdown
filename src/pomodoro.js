export const POMODORO_STORAGE_KEY = "countdown-pomodoro";
const defaults = { focus: 25, break: 5, longBreak: 15 };
const MINUTE = 60_000;
const validCount = value => Number.isSafeInteger(value) && value >= 0 ? value : 0;

export function focusSchedule(config, now, minutes) {
  if (!config) return null;
  const time = key => {
    const [hour, minute] = config[key].split(":").map(Number);
    return new Date(now).setHours(hour, minute, 0, 0);
  };
  const start = time("start_time");
  const lunchStart = time("lunch_start_time");
  const lunchEnd = time("lunch_end_time");
  const end = time("end_time");
  const morning = Math.max(0, lunchStart - Math.max(now, start));
  const afternoon = Math.max(0, end - Math.max(now, lunchEnd));
  const block = (minutes.focus + minutes.break) * MINUTE;
  const working = now >= start && (now < lunchStart || now >= lunchEnd && now < end);
  const boundary = now < lunchStart ? lunchStart : end;
  return {
    rounds: Math.floor(morning / block) + Math.floor(afternoon / block),
    available: working ? boundary - now : 0,
    cutoff: now < lunchStart ? "午休" : "下班",
    unavailable: now < start ? "上班后可开始专注" : now < lunchEnd ? "午休结束后可开始专注" : "今天已下班",
  };
}

function dayKey(timestamp) {
  const date = new Date(timestamp);
  return [date.getFullYear(), date.getMonth() + 1, date.getDate()].join("-");
}

export function formatPomodoro(milliseconds) {
  const seconds = Math.ceil(milliseconds / 1000);
  return [Math.floor(seconds / 60), seconds % 60].map(part => String(part).padStart(2, "0")).join(":");
}

export class Pomodoro {
  constructor(saved) {
    this.minutes = { ...defaults };
    this.setMinutes(saved?.minutes);
    this.day = typeof saved?.day === "string" ? saved.day : "";
    this.rounds = Number.isSafeInteger(saved?.rounds) && saved.rounds >= 0 ? saved.rounds : 0;
    this.cycle = Number.isSafeInteger(saved?.cycle) && saved.cycle >= 0 ? saved.cycle : 0;
    this.activity = {
      day: typeof saved?.activity?.day === "string" ? saved.activity.day : "",
      focusMs: validCount(saved?.activity?.focusMs),
      interruptions: validCount(saved?.activity?.interruptions),
    };
    this.interval = Number.isInteger(saved?.interval) && saved.interval >= 1 && saved.interval <= 20 ? saved.interval : 4;
    // Older versions defaulted to manual breaks; migrate that default once.
    this.autoBreak = saved?.settingsVersion >= 1 ? saved.autoBreak !== false : true;
    this.autoFocus = saved?.autoFocus === true;
    this.phase = "focus";
    this.longBreak = false;
    this.reset();
  }

  get duration() { return this.wrapUpDuration ?? this.minutes[this.phase === "break" && this.longBreak ? "longBreak" : this.phase] * MINUTE; }
  get running() { return this.deadline !== null; }
  get active() { return this.running || this.remaining < this.duration; }
  get progress() { return 100 * (1 - this.remaining / this.duration); }

  get saved() { return { settingsVersion: 1, minutes: this.minutes, day: this.day, rounds: this.rounds, cycle: this.cycle, interval: this.interval, autoBreak: this.autoBreak, autoFocus: this.autoFocus, activity: this.activity }; }

  get nextLongBreak() { return this.cycle + 1 >= this.interval; }

  todayRounds(now) { return this.day === dayKey(now) ? this.rounds : 0; }

  todayActivity(now) {
    const today = this.activity.day === dayKey(now);
    const midnight = new Date(now).setHours(0, 0, 0, 0);
    const live = this.focusStarted == null ? 0
      : Math.max(0, Math.min(now, this.deadline) - Math.max(midnight, this.focusStarted));
    return {
      minutes: Math.floor(((today ? this.activity.focusMs : 0) + live) / MINUTE),
      interruptions: today ? this.activity.interruptions : 0,
    };
  }

  activityOn(now) {
    const day = dayKey(now);
    if (this.activity.day !== day) this.activity = { day, focusMs: 0, interruptions: 0 };
    return this.activity;
  }

  recordFocus(end, interrupted = false) {
    if (this.focusStarted == null) return;
    const midnight = new Date(end).setHours(0, 0, 0, 0);
    const elapsed = Math.max(0, end - Math.max(midnight, this.focusStarted));
    if (elapsed || interrupted) {
      const activity = this.activityOn(end);
      activity.focusMs += elapsed;
      if (interrupted) activity.interruptions++;
    }
    this.focusStarted = null;
  }

  setMinutes(minutes, now) {
    const { focus, break: rest, longBreak = this.minutes.longBreak } = minutes ?? {};
    if (!Number.isInteger(focus) || focus < 1 || focus > 120 ||
        !Number.isInteger(rest) || rest < 1 || rest > 60 ||
        !Number.isInteger(longBreak) || longBreak < 1 || longBreak > 60) return false;
    const key = this.phase === "break" && this.longBreak ? "longBreak" : this.phase;
    const reset = key && this.minutes[key] !== { focus, break: rest, longBreak }[key];
    this.minutes = { focus, break: rest, longBreak };
    if (reset) this.reset(now);
    return true;
  }

  reset(now) {
    if (now != null && this.running) this.recordFocus(Math.min(now, this.deadline), now < this.deadline);
    this.focusStarted = null;
    this.wrapUpDuration = null;
    this.deadline = null;
    this.remaining = this.duration;
  }

  toggle(now, focusLimit = () => Infinity) {
    if (this.running) {
      const finished = this.update(now, focusLimit);
      if (this.running && this.focusStarted != null) this.recordFocus(now, now > this.focusStarted);
      this.deadline = null;
      return finished;
    }
    if (this.phase === "focus" && this.remaining > focusLimit(now)) return null;
    this.start(now);
    return null;
  }

  start(now) {
    this.deadline = now + this.remaining;
    if (this.phase === "focus") this.focusStarted = now;
  }

  startWrapUp(now, minutes) {
    this.wrapUpDuration = minutes * MINUTE;
    this.remaining = this.wrapUpDuration;
    this.start(now);
  }

  update(now, focusLimit = () => Infinity) {
    if (!this.running) return null;
    let completed = null;
    while (this.running && now >= this.deadline) {
      const endedAt = this.deadline;
      const finished = this.phase;
      if (finished === "focus") {
        this.recordFocus(endedAt);
        if (this.wrapUpDuration != null) {
          this.reset();
          return "wrap-up";
        }
        const day = dayKey(endedAt);
        this.rounds = (this.day === day ? this.rounds : 0) + 1;
        this.day = day;
        this.longBreak = this.nextLongBreak;
        this.cycle = this.longBreak ? 0 : this.cycle + 1;
        completed = "focus";
      } else if (!completed) completed = "break";
      this.phase = finished === "focus" ? "break" : "focus";
      this.reset();
      if (finished === "focus" ? this.autoBreak : this.autoFocus && this.duration <= focusLimit(endedAt)) this.start(endedAt);
    }
    if (this.running) this.remaining = Math.max(0, Math.min(this.duration, this.deadline - now));
    return completed;
  }
}
