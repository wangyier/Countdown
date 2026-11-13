// Browser-only preview. The desktop app always uses Rust for its calculations.
import { DEFAULT_CONFIG, validateSchedule } from "./time.js";

export function localDateString(date) {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

export function previewStats(config, now = new Date()) {
  const [start, lunchStart, lunchEnd, end] = validateSchedule(config);
  const date = localDateString(now);
  const time = now.getHours() * 3600 + now.getMinutes() * 60 + now.getSeconds();
  const total = lunchStart - start + end - lunchEnd;
  let status;
  let elapsed = 0;
  if (time < start) status = "before_work";
  else if (time < lunchStart) { status = "working"; elapsed = time - start; }
  else if (time < lunchEnd) { status = "lunch_break"; elapsed = lunchStart - start; }
  else if (time < end) { status = "working"; elapsed = lunchStart - start + time - lunchEnd; }
  else { status = "after_work"; elapsed = total; }
  return {
    status, date,
    progress_percent: Math.min(100, Math.max(0, elapsed / total * 100)),
    remaining_seconds: Math.max(0, end - time),
    guandan_remaining_seconds: Math.max(0, lunchStart - time),
    guandan_ready: time >= lunchStart && time < lunchEnd,
    schedule: { ...config },
  };
}

export async function createPreviewBridge() {
  let config = { ...DEFAULT_CONFIG };
  try {
    const saved = JSON.parse(localStorage.getItem("countdown-preview-config") || "null");
    if (saved) { const restored = { ...DEFAULT_CONFIG, ...saved }; validateSchedule(restored); config = restored; }
  } catch { /* Ignore invalid preview-only storage; the desktop config is unaffected. */ }
  return async function invoke(command, args = {}) {
    switch (command) {
      case "get_config": return { ...config };
      case "get_countdown_stats": return previewStats(config);
      case "get_app_info": return { local_ip_suffix: null, warnings: [] };
      case "set_mini_mode":
        config = { ...config, mini_mode: Boolean(args.enabled) };
        localStorage.setItem("countdown-preview-config", JSON.stringify(config));
        return { ...config };
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
