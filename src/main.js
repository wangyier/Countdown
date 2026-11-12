import { validateSchedule, formatCountdown, STATUS_DETAILS } from "./time.js";
import { createPreviewBridge } from "./preview.js";
import { attachWindowDrag } from "./drag.js";

const $ = (selector) => document.querySelector(selector);
const desktop = Boolean(window.__TAURI__?.core?.invoke);
const dialog = $("#settings-dialog");
const form = $("#settings-form");
const timeKeys = ["start_time", "end_time", "lunch_start_time", "lunch_end_time", "reminder_time"];
const toggleKeys = ["autostart_enabled", "always_on_top", "reminder_enabled"];
let invoke;
let config;
let info;
let syncing = false;
let pendingSync = false;
let saving = false;
let noticeTimeout;
let lastSyncTime = 0;
let lastInfoTime = 0;
let connectionError = false;
let stopped = false;
const unlisteners = [];

function notify(message, persistent = false) {
  clearTimeout(noticeTimeout);
  $("#notice").textContent = String(message);
  $("#notice").hidden = false;
  if (!persistent) noticeTimeout = setTimeout(() => { $("#notice").hidden = true; }, 6000);
}

function report(error) {
  notify(error instanceof Error ? error.message : String(error));
}

if (desktop) {
  $("#widget").setAttribute("data-window-drag-region", "");
  unlisteners.push(attachWindowDrag($("#widget"), {
    startDragging: () => window.__TAURI__.window.getCurrentWindow().startDragging(),
    onError: (error) => notify(`窗口拖动失败：${error instanceof Error ? error.message : String(error)}`),
  }));
} else {
  $(".topbar").title = "浏览器预览无法拖动系统窗口，请在桌面版中拖动";
}

function renderConfig(value) {
  config = value;
  const pin = $("#pin-button");
  const label = value.always_on_top ? "取消置顶" : "窗口置顶";
  pin.setAttribute("aria-pressed", String(value.always_on_top));
  pin.setAttribute("aria-label", label);
  pin.title = label;
  $("#reminder-time").textContent = value.reminder_enabled ? value.reminder_time : "已关闭";
  $("#reminder-time").title = value.reminder_enabled ? `每天 ${value.reminder_time} 提醒点外卖` : "外卖提醒已关闭";
}

function renderStats(value) {
  const details = STATUS_DETAILS[value.status];
  if (!details) throw new Error("无法识别工作状态，请重新打开应用");
  $("#widget").dataset.status = value.status;
  $("#status-label").textContent = details.label;
  $("#countdown-label").textContent = details.heading;
  $("#motto").textContent = details.motto;
  const parts = formatCountdown(value.remaining_seconds);
  ["hours", "minutes", "seconds"].forEach((id, index) => { $(`#${id}`).textContent = parts[index]; });
  $("#countdown").setAttribute("aria-label", `${parts[0]}小时${parts[1]}分钟${parts[2]}秒`);
  const guandanParts = formatCountdown(value.guandan_remaining_seconds);
  $("#guandan-countdown").textContent = guandanParts.join(":");
  $("#guandan-countdown").setAttribute("aria-label", `${guandanParts[0]}小时${guandanParts[1]}分钟${guandanParts[2]}秒`);
  const guandanReached = value.guandan_ready;
  $("#guandan-card").dataset.reached = String(guandanReached);
  $("#guandan-status").textContent = guandanReached ? "掼蛋时间到！"
    : value.guandan_remaining_seconds === 0 ? "今日已结束" : `目标 ${value.schedule.lunch_start_time}`;
  const progress = Math.min(100, Math.max(0, value.progress_percent));
  $("#progress-value").replaceChildren(document.createTextNode(progress.toFixed(1)), Object.assign(document.createElement("small"), { textContent: "%" }));
  $("#progress-fill").style.width = `${progress}%`;
  $("#progress").setAttribute("aria-valuenow", progress.toFixed(1));
  const date = new Date(`${value.date}T12:00:00`);
  $("#date-label").textContent = new Intl.DateTimeFormat("zh-CN", { month: "long", day: "numeric", weekday: "long" }).format(date);
  if (value.schedule) renderConfig(value.schedule);
  renderInfo();
}

function renderInfo() {
  if (info) $("#ip-label").textContent = `IP · ${info.local_ip_suffix ?? "—"}`;
  const warnings = info?.warnings || [];
  const footer = $("#footer-label");
  footer.textContent = warnings.length ? "设置状态需要检查 ⓘ"
    : desktop ? "准时收工，生活开场 ✳" : "浏览器预览 · 系统功能未启用";
  footer.title = warnings.join("\n") || (desktop ? "Esc 隐藏 · 托盘点击恢复" : "预览设置仅保存在浏览器，托盘、置顶、通知和自启动请使用桌面版。");
  footer.tabIndex = warnings.length ? 0 : -1;
  $("#schedule-note").textContent = `每天按设置的上下班和午休时间计算。${desktop ? "" : "预览模式不修改系统设置。"}`;
}

async function sync() {
  if (!invoke || stopped) return;
  if (syncing) { pendingSync = true; return; }
  syncing = true;
  try {
    const needsInfo = Date.now() - lastInfoTime > 30000;
    const values = await Promise.all([
      invoke("get_countdown_stats"),
      needsInfo ? invoke("get_app_info") : Promise.resolve(info),
    ]);
    if (values[1]) { info = values[1]; if (needsInfo) lastInfoTime = Date.now(); }
    renderStats(values[0]);
    lastSyncTime = Date.now();
    if (connectionError) { connectionError = false; notify("倒计时已重新同步"); }
  } catch (error) {
    if (!connectionError) { connectionError = true; notify(`同步失败：${String(error)}`, true); }
    $("#status-label").textContent = "等待同步";
  } finally {
    syncing = false;
    if (pendingSync && !stopped) { pendingSync = false; void sync(); }
  }
}

async function openSettings() {
  if (dialog.open || saving || !invoke) return;
  $("#settings-button").disabled = true;
  try {
    const current = await invoke("get_config");
    if (dialog.open) return;
    renderConfig(current);
    for (const key of timeKeys) form.elements.namedItem(key).value = current[key];
    for (const key of toggleKeys) form.elements.namedItem(key).checked = current[key];
    updateReminderInput();
    $("#settings-error").hidden = true;
    dialog.showModal();
  } catch (error) { report(error); }
  finally { $("#settings-button").disabled = false; }
}

function closeSettings() { if (!saving) dialog.close(); }
function updateReminderInput() {
  form.elements.namedItem("reminder_time").disabled = saving || !form.elements.namedItem("reminder_enabled").checked;
}
form.elements.namedItem("reminder_enabled").addEventListener("change", updateReminderInput);
$("#settings-button").addEventListener("click", openSettings);
$("#close-settings").addEventListener("click", closeSettings);
$("#cancel-settings").addEventListener("click", closeSettings);
dialog.addEventListener("cancel", (event) => { if (saving) event.preventDefault(); });

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  if (saving) return;
  const next = { ...config };
  for (const key of timeKeys) next[key] = form.elements.namedItem(key).value;
  for (const key of toggleKeys) next[key] = form.elements.namedItem(key).checked;
  const errorLabel = $("#settings-error");
  errorLabel.hidden = true;
  try {
    validateSchedule(next);
    saving = true;
    for (const element of form.elements) element.disabled = true;
    $("#save-button").textContent = "正在保存…";
    renderConfig(await invoke("save_config", { config: next }));
    dialog.close();
    lastInfoTime = 0;
    await sync();
    notify(desktop ? "设置已保存" : "预览设置已保存，系统设置未修改");
  } catch (error) {
    errorLabel.textContent = error instanceof Error ? error.message : String(error);
    errorLabel.hidden = false;
  } finally {
    saving = false;
    for (const element of form.elements) element.disabled = false;
    updateReminderInput();
    $("#save-button").textContent = "保存设置 ↗";
  }
});

$("#pin-button").addEventListener("click", async () => {
  const button = $("#pin-button");
  button.disabled = true;
  try {
    await invoke("toggle_pin");
    await sync();
    if (!desktop) notify("预览已切换标记；实际置顶请使用桌面版");
  } catch (error) { report(error); }
  finally { button.disabled = false; }
});

async function hide() {
  if (!desktop) { notify("隐藏到托盘请在桌面应用中使用"); return; }
  try { await invoke("hide_window"); } catch (error) { report(error); }
}
$("#hide-button").addEventListener("click", hide);
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && !dialog.open) { event.preventDefault(); void hide(); }
});
$("#footer-label").addEventListener("click", () => {
  if (info?.warnings?.length) notify($("#footer-label").title);
});
$("#footer-label").addEventListener("keydown", (event) => {
  if (event.key === "Enter" || event.key === " ") { event.preventDefault(); $("#footer-label").click(); }
});
document.addEventListener("visibilitychange", () => { if (!document.hidden) void sync(); });
window.addEventListener("focus", () => { void sync(); });

async function start() {
  try {
    invoke = desktop ? window.__TAURI__.core.invoke : await createPreviewBridge();
    if (desktop) {
      const listen = window.__TAURI__.event.listen;
      unlisteners.push(await listen("open-settings", openSettings));
      unlisteners.push(await listen("widget-shown", () => { void sync(); }));
      unlisteners.push(await listen("config-changed", () => { lastInfoTime = 0; void sync(); }));
      unlisteners.push(await listen("app-error", ({ payload }) => notify(payload)));
    }
    await sync();
  } catch (error) { notify(`启动失败：${String(error)}`, true); }
}

const interval = setInterval(() => {
  if (document.hidden) return;
  if (lastSyncTime && Date.now() - lastSyncTime > 5000) {
    $("#status-label").textContent = "等待同步";
  }
  void sync();
}, 1000);
window.addEventListener("pagehide", () => {
  stopped = true;
  clearInterval(interval);
  for (const unlisten of unlisteners) unlisten();
});
void start();
