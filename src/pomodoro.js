export const POMODORO_STORAGE_KEY = "countdown-pomodoro";
const defaults = { focus: 25, break: 5 };

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
    this.phase = "focus";
    this.reset();
  }

  get duration() { return this.minutes[this.phase] * 60_000; }
  get running() { return this.deadline !== null; }
  get active() { return this.running || this.remaining < this.duration; }
  get progress() { return 100 * (1 - this.remaining / this.duration); }

  get saved() { return { minutes: this.minutes, day: this.day, rounds: this.rounds }; }

  todayRounds(now) { return this.day === dayKey(now) ? this.rounds : 0; }

  setMinutes(minutes) {
    const { focus, break: rest } = minutes ?? {};
    if (!Number.isInteger(focus) || focus < 1 || focus > 120 ||
        !Number.isInteger(rest) || rest < 1 || rest > 60) return false;
    const reset = this.phase && this.minutes[this.phase] !== minutes[this.phase];
    this.minutes = { focus, break: rest };
    if (reset) this.reset();
    return true;
  }

  reset() {
    this.deadline = null;
    this.remaining = this.duration;
  }

  toggle(now) {
    if (this.running) {
      const finished = this.update(now);
      this.deadline = null;
      return finished;
    }
    this.deadline = now + this.remaining;
    return null;
  }

  update(now) {
    if (!this.running) return null;
    this.remaining = Math.max(0, Math.min(this.duration, this.deadline - now));
    if (this.remaining > 0) return null;
    const finished = this.phase;
    if (finished === "focus") {
      const day = dayKey(this.deadline);
      this.rounds = (this.day === day ? this.rounds : 0) + 1;
      this.day = day;
    }
    this.phase = finished === "focus" ? "break" : "focus";
    this.reset();
    return finished;
  }
}
