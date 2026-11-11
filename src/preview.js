// Browser-only preview. The desktop app always uses Rust for its calculations.
import { DEFAULT_CONFIG, validateSchedule } from "./time.js";

export function localDateString(date) {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

export function previewStats(config, calendar, now = new Date()) {
  const [start, lunchStart, lunchEnd, end] = validateSchedule(config);
  const date = localDateString(now);
  const weekday = now.getDay();
  const nextSaturday = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 7);
  const lastSaturday = weekday === 6 && nextSaturday.getMonth() !== now.getMonth();
  const holiday = calendar.ranges.some(([start, end]) => start <= date && date <= end);
  const workday = !holiday && ((weekday >= 1 && weekday <= 5) || lastSaturday);
  const time = now.getHours() * 3600 + now.getMinutes() * 60 + now.getSeconds();
  const total = lunchStart - start + end - lunchEnd;
  let status = "day_off";
  let elapsed = 0;
  if (workday) {
    if (time < start) status = "before_work";
    else if (time < lunchStart) { status = "working"; elapsed = time - start; }
    else if (time < lunchEnd) { status = "lunch_break"; elapsed = lunchStart - start; }
    else if (time < end) { status = "working"; elapsed = lunchStart - start + time - lunchEnd; }
    else { status = "after_work"; elapsed = total; }
  }
  return {
    status, date, is_workday: workday,
    calendar_known: calendar.years.includes(now.getFullYear()),
    progress_percent: Math.min(100, Math.max(0, elapsed / total * 100)),
    remaining_seconds: workday ? Math.max(0, end - time) : 0,
    guandan_remaining_seconds: workday ? Math.max(0, 11 * 3600 + 45 * 60 - time) : 0,
    guandan_ready: workday && time >= 11 * 3600 + 45 * 60 && time < 13 * 3600 + 15 * 60,
    schedule: { ...config },
  };
}

export async function createPreviewBridge() {
  const response = await fetch("./data/holidays.json");
  if (!response.ok) throw new Error("预览日历加载失败");
  const calendar = await response.json();
  let config = { ...DEFAULT_CONFIG };
  try {
    const saved = JSON.parse(localStorage.getItem("countdown-preview-config") || "null");
    if (saved) { validateSchedule(saved); config = { ...DEFAULT_CONFIG, ...saved }; }
  } catch { /* Ignore invalid preview-only storage; the desktop config is unaffected. */ }
  return async function invoke(command, args = {}) {
    switch (command) {
      case "get_config": return { ...config };
      case "get_countdown_stats": return previewStats(config, calendar);
      case "get_app_info": return { local_ip_suffix: null, warnings: [], calendar_years: calendar.years };
      case "save_config":
        validateSchedule(args.config);
        localStorage.setItem("countdown-preview-config", JSON.stringify(args.config));
        config = { ...args.config };
        return { ...config };
      case "toggle_pin":
        config = { ...config, always_on_top: !config.always_on_top };
        localStorage.setItem("countdown-preview-config", JSON.stringify(config));
        return config.always_on_top;
      default: throw new Error("此功能请在桌面应用中使用");
    }
  };
}
