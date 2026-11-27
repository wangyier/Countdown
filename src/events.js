import { formatCountdown } from "./time.js";

export const EVENT_STORAGE_KEY = "countdown-events";
export const WEEKDAYS = ["星期日", "星期一", "星期二", "星期三", "星期四", "星期五", "星期六"];
const DAY = 86_400_000;
const timePattern = /^(?:[01]\d|2[0-3]):[0-5]\d$/;
const atTime = (date, time) => {
  const [hour, minute] = time.split(":").map(Number);
  return new Date(date).setHours(hour, minute, 0, 0);
};

export function loadEvents(storage) {
  try {
    const saved = JSON.parse(storage.getItem(EVENT_STORAGE_KEY) ?? "null");
    return {
      weekend: { enabled: false, weekday: 5, time: "", ...saved?.weekend },
      payday: { enabled: false, day: 15, ...saved?.payday },
      custom: Array.isArray(saved?.custom) ? saved.custom.filter(event =>
        typeof event?.id === "string" && typeof event.title === "string"
        && typeof event.date === "string" && /^\d{4}-\d{2}-\d{2}$/.test(event.date)
        && typeof event.time === "string" && timePattern.test(event.time)
      ) : [],
      priority: saved?.priority ?? "",
    };
  } catch {
    return loadEvents({ getItem: () => null });
  }
}

function nextWeekly(now, weekday, time) {
  const date = new Date(now);
  date.setDate(date.getDate() + (weekday - date.getDay() + 7) % 7);
  if (atTime(date, time) <= now) date.setDate(date.getDate() + 7);
  return atTime(date, time);
}

function monthlyTarget(now, day, offset = 0) {
  const date = new Date(now);
  const target = month => {
    const year = date.getFullYear();
    const end = new Date(year, month + 1, 0).getDate();
    return +new Date(year, month, Math.min(day, end));
  };
  const current = target(date.getMonth());
  const month = date.getMonth() + (current > now ? 0 : 1);
  return target(month + offset);
}

function customTarget(event, now, previous = false) {
  const [year, month, day] = event.date.split("-").map(Number);
  const startYear = event.yearly ? new Date(now).getFullYear() : year;
  for (let offset = 0; offset <= (event.yearly ? 8 : 0); offset++) {
    const date = new Date(startYear + (previous ? -offset : offset), month - 1, day);
    if (date.getMonth() !== month - 1 || date.getDate() !== day) continue;
    const target = atTime(date, event.time);
    if (previous ? target <= now : target > now) return target;
  }
  return null;
}

export function selectEvent(events, stats, now) {
  const candidates = [];
  if (stats.guandan_remaining_seconds > 0 || stats.guandan_ready) {
    candidates.push({
      id: "meal", title: "开饭倒计时", target: atTime(now, stats.schedule.lunch_start_time),
      start: atTime(now, stats.schedule.start_time),
      reached: stats.guandan_ready,
    });
  }
  if (events.weekend.enabled) {
    const { weekday, time } = events.weekend;
    const target = nextWeekly(now, weekday, time || stats.schedule.end_time);
    const start = new Date(target);
    start.setDate(start.getDate() - 7);
    candidates.push({
      id: "weekend", title: "周末倒计时",
      target, start: +start,
    });
  }
  if (events.payday.enabled) {
    const { day } = events.payday;
    candidates.push({ id: "payday", title: "发薪日", target: monthlyTarget(now, day), start: monthlyTarget(now, day, -1) });
  }
  for (const event of events.custom) {
    const target = customTarget(event, now);
    if (target != null) candidates.push({
      id: event.id, title: event.title, target,
      start: event.yearly ? customTarget(event, now, true) : event.createdAt,
    });
  }
  return candidates.find(event => event.id === events.priority)
    ?? candidates.sort((a, b) => a.target - b.target)[0];
}

const targetLabel = new Intl.DateTimeFormat("zh-CN", {
  month: "long", day: "numeric", hour: "2-digit", minute: "2-digit", hour12: false,
});

export function eventDisplay(event, now) {
  if (event.reached) return { time: "00:00:00", detail: "开饭时间到！" };
  const remaining = Math.max(0, event.target - now);
  const days = remaining >= DAY ? Math.ceil(remaining / DAY) : null;
  const time = days != null
    ? `还有 ${days} 天`
    : formatCountdown(Math.ceil(remaining / 1000)).join(":");
  return { time, days, detail: targetLabel.format(event.target) };
}

export function eventProgress(event, now) {
  if (!Number.isFinite(event.start) || event.start >= event.target) return null;
  return Math.floor(Math.max(0, Math.min(1, (now - event.start) / (event.target - event.start))) * 100);
}
