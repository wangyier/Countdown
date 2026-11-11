export const DEFAULT_CONFIG = Object.freeze({
  start_time: "09:00",
  end_time: "17:30",
  lunch_start_time: "11:45",
  lunch_end_time: "13:15",
  autostart_enabled: true,
  always_on_top: false,
});

export function secondsOfDay(value) {
  if (!/^([01]\d|2[0-3]):[0-5]\d$/.test(value)) {
    throw new Error("请输入有效时间（00:00—23:59）");
  }
  const [hours, minutes] = value.split(":").map(Number);
  return hours * 3600 + minutes * 60;
}

export function validateSchedule(config) {
  const values = [config.start_time, config.lunch_start_time, config.lunch_end_time, config.end_time].map(secondsOfDay);
  if (!values.every((value, index) => index === 0 || values[index - 1] < value)) {
    throw new Error("时间顺序须为：上班 < 午休开始 < 午休结束 < 下班");
  }
  return values;
}

export function formatCountdown(seconds) {
  const value = Math.max(0, Math.floor(seconds));
  return [Math.floor(value / 3600), Math.floor(value / 60) % 60, value % 60]
    .map((part) => String(part).padStart(2, "0"));
}

export const STATUS_DETAILS = Object.freeze({
  before_work: { label: "上班前", heading: "距离下班还有", motto: "新的一天，按自己的节奏出发。" },
  working: { label: "工作中", heading: "距离下班还有", motto: "认真工作，也别忘了准时生活。" },
  lunch_break: { label: "午休中", heading: "距离下班还有", motto: "好好吃饭，午休不必争分夺秒。" },
  after_work: { label: "已下班", heading: "今日工作已结束", motto: "今天辛苦了，接下来是你的时间。" },
  day_off: { label: "休息日", heading: "休息日 / 充能中", motto: "今天的待办：好好享受生活。" },
});
