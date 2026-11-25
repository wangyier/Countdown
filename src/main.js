import { validateSchedule, formatCountdown, STATUS_DETAILS, miniDisplay } from "./time.js";
import { createPreviewBridge } from "./preview.js";
import { attachWindowDrag, attachMiniDrag } from "./drag.js";
import { Pomodoro, POMODORO_STORAGE_KEY, formatPomodoro } from "./pomodoro.js";

const $ = (selector) => document.querySelector(selector);
const desktop = Boolean(window.__TAURI__?.core?.invoke);
const dialog = $("#settings-dialog");
const form = $("#settings-form");
const pomodoroDialog = $("#pomodoro-settings");
const pomodoroForm = $("#pomodoro-form");
const timeKeys = ["start_time", "end_time", "lunch_start_time", "lunch_end_time", "reminder_time"];
const toggleKeys = ["autostart_enabled", "always_on_top", "reminder_enabled", "show_ip"];
let invoke;
let config;
let info;
let workMini;
let activeView = "work";
let lastTrayTimer;
let traySync = Promise.resolve();
let syncing = false;
let pendingSync = false;
let saving = false;
let switchingMode = false;
let modeRevision = 0;
let miniMenuReturnFocus;
let noticeTimeout;
let lastSyncTime = 0;
let lastInfoTime = 0;
let connectionError = false;
let stopped = false;
const unlisteners = [];

function loadPomodoro() {
  try { return new Pomodoro(JSON.parse(localStorage.getItem(POMODORO_STORAGE_KEY))); }
  catch { return new Pomodoro(); }
}

const pomodoro = loadPomodoro();

function savePomodoro() {
  try { localStorage.setItem(POMODORO_STORAGE_KEY, JSON.stringify(pomodoro.saved)); }
  catch { notify("番茄钟无法保存，请检查应用存储。"); }
}

savePomodoro();

function syncTrayPomodoro() {
  if (!desktop || !invoke) return;
  const timer = pomodoro.active ? {
    phase: pomodoro.phase,
    clock: pomodoro.running
      ? { kind: "deadline", milliseconds: pomodoro.deadline }
      : { kind: "paused", milliseconds: pomodoro.remaining },
  } : null;
  const signature = JSON.stringify(timer);
  if (signature === lastTrayTimer) return;
  lastTrayTimer = signature;
  traySync = traySync.then(() => invoke("set_pomodoro_tooltip", { timer })).catch(report);
}

function notify(message, persistent = false) {
  clearTimeout(noticeTimeout);
  $("#notice").textContent = String(message);
  $("#notice").hidden = false;
  $("#mini-notice").textContent = String(message);
  $("#mini-notice").hidden = !config?.mini_mode;
  if (!persistent) noticeTimeout = setTimeout(() => { $("#notice").hidden = true; $("#mini-notice").hidden = true; }, 6000);
}

function report(error) {
  notify(error instanceof Error ? error.message : String(error));
}

function selectView(view) {
  activeView = view;
  for (const name of ["work", "pomodoro"]) {
    $(`#${name}-view`).hidden = name !== view;
    $(`#${name}-view-button`).setAttribute("aria-pressed", String(name === view));
  }
  const label = view === "pomodoro" ? "番茄钟设置" : "打开设置";
  $("#settings-button").setAttribute("aria-label", label);
  $("#settings-button").title = label;
  tickPomodoro();
}

function renderMini() {
  const timer = pomodoro.active;
  const focus = pomodoro.phase === "focus";
  const value = timer ? {
    status: `pomodoro-${pomodoro.phase}`,
    label: focus ? "番茄钟 · 专注" : pomodoro.longBreak ? "番茄钟 · 长休息" : "番茄钟 · 短休息",
    time: formatPomodoro(pomodoro.remaining),
    words: false,
    mark: pomodoro.running ? "计时中" : "已暂停",
    progress: pomodoro.progress,
    progressLabel: "当前番茄钟阶段进度",
  } : workMini;
  if (!value) return;
  $("#mini-widget").dataset.status = value.status;
  $("#mini-label").textContent = value.label;
  $("#mini-time").textContent = value.time;
  $("#mini-time").classList.toggle("words", value.words);
  $("#mini-time").setAttribute("aria-label", value.timeLabel ?? value.time);
  $("#mini-percent").textContent = value.mark;
  $("#mini-fill").style.strokeDashoffset = String(175.929 * (1 - value.progress / 100));
  $("#mini-ring").setAttribute("aria-label", value.progressLabel);
  $("#mini-ring").setAttribute("aria-valuenow", value.progress.toFixed(1));
}

function renderPomodoro(now) {
  const focus = pomodoro.phase === "focus";
  const phase = $("#pomodoro-phase");
  const phaseLabel = focus ? "专注时间" : pomodoro.longBreak ? "长休息" : "短休息";
  if (phase.textContent !== phaseLabel) phase.textContent = phaseLabel;
  $("#pomodoro-clock").dataset.phase = pomodoro.phase;
  $("#pomodoro-clock").style.setProperty("--elapsed", `${pomodoro.progress / 100}turn`);
  $("#pomodoro-time").textContent = formatPomodoro(pomodoro.remaining);
  $("#pomodoro-start").textContent = pomodoro.running ? "暂停"
    : pomodoro.active ? "继续" : focus ? "开始专注" : "开始休息";
  $("#pomodoro-rounds").textContent = `今日完成 ${pomodoro.todayRounds(now)} 轮`;
  $("#pomodoro-next").textContent = focus ? `下一段 · ${pomodoro.nextLongBreak ? "长休息" : "短休息"} ${pomodoro.minutes[pomodoro.nextLongBreak ? "longBreak" : "break"]} 分钟`
    : `下一段 · 专注 ${pomodoro.minutes.focus} 分钟`;
  renderMini();
  syncTrayPomodoro();
}

function tickPomodoro() {
  const now = Date.now();
  if (pomodoro.update(now) === "focus") savePomodoro();
  renderPomodoro(now);
}

function openPomodoroSettings() {
  if (pomodoroDialog.open || dialog.open || switchingMode || saving) return;
  tickPomodoro();
  for (const phase of ["focus", "break", "longBreak"]) {
    pomodoroForm.elements.namedItem(phase).value = pomodoro.minutes[phase];
  }
  pomodoroForm.elements.namedItem("interval").value = pomodoro.interval;
  for (const key of ["autoBreak", "autoFocus"]) pomodoroForm.elements.namedItem(key).checked = pomodoro[key];
  $("#pomodoro-settings-note").hidden = !pomodoro.active;
  pomodoroDialog.showModal();
}

$("#work-view-button").addEventListener("click", () => selectView("work"));
$("#pomodoro-view-button").addEventListener("click", () => selectView("pomodoro"));
function togglePomodoro() {
  const now = Date.now();
  if (pomodoro.toggle(now) === "focus") savePomodoro();
  renderPomodoro(now);
}
$("#pomodoro-start").addEventListener("click", togglePomodoro);
$("#pomodoro-reset").addEventListener("click", () => {
  tickPomodoro();
  pomodoro.reset();
  tickPomodoro();
});
$("#pomodoro-settings-close").addEventListener("click", () => pomodoroDialog.close());
$("#pomodoro-settings-cancel").addEventListener("click", () => pomodoroDialog.close());
pomodoroForm.addEventListener("submit", (event) => {
  event.preventDefault();
  tickPomodoro();
  const { elements } = pomodoroForm;
  if (!pomodoro.setMinutes({
    focus: elements.namedItem("focus").valueAsNumber,
    break: elements.namedItem("break").valueAsNumber,
    longBreak: elements.namedItem("longBreak").valueAsNumber,
  })) return;
  pomodoro.interval = elements.namedItem("interval").valueAsNumber;
  pomodoro.autoBreak = elements.namedItem("autoBreak").checked;
  pomodoro.autoFocus = elements.namedItem("autoFocus").checked;
  pomodoroDialog.close();
  savePomodoro();
  tickPomodoro();
});

if (desktop) {
  $("#widget").setAttribute("data-window-drag-region", "");
  unlisteners.push(attachWindowDrag($("#widget"), {
    startDragging: () => window.__TAURI__.window.getCurrentWindow().startDragging(),
    onError: (error) => notify(`窗口拖动失败：${error instanceof Error ? error.message : String(error)}`),
  }));
  unlisteners.push(attachMiniDrag($("#mini-widget"), {
    startDragging: () => window.__TAURI__.window.getCurrentWindow().startDragging(),
    onError: report,
  }));
} else {
  $(".topbar").title = "浏览器预览无法拖动系统窗口，请在桌面版中拖动";
}

function renderConfig(value) {
  const widget = $("#widget");
  if (value.mini_mode) widget.classList.remove("widget-expanded");
  else if (config?.mini_mode) widget.classList.add("widget-expanded");
  config = value;
  document.body.dataset.mini = String(Boolean(value.mini_mode));
  widget.hidden = Boolean(value.mini_mode);
  $("#mini-widget").hidden = !value.mini_mode;
  if (value.mini_mode && dialog.open) dialog.close();
  if (!value.mini_mode) { closeMiniMenu(false); $("#mini-notice").hidden = true; }
  $("#ip-label").hidden = !value.show_ip;
  const pin = $("#pin-button");
  const label = value.always_on_top ? "取消置顶" : "窗口置顶";
  pin.setAttribute("aria-pressed", String(value.always_on_top));
  pin.setAttribute("aria-label", label);
  pin.title = label;
  $("#mini-menu-pin").textContent = value.always_on_top ? "取消置顶" : "置顶";
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
  $("#guandan-status").textContent = guandanReached ? "开饭时间到！"
    : value.guandan_remaining_seconds === 0 ? "今日已结束" : `目标 ${value.schedule.lunch_start_time}`;
  const progress = Math.min(100, Math.max(0, value.progress_percent));
  const mini = miniDisplay(value);
  workMini = {
    ...mini,
    status: value.status,
    timeLabel: mini.words ? mini.time : `${parts[0]}小时${parts[1]}分钟`,
    mark: `${Math.round(progress)}%`,
    progress,
    progressLabel: "有效工时完成比例",
  };
  renderMini();
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
  const revision = modeRevision;
  try {
    const needsInfo = Date.now() - lastInfoTime > 30000;
    const values = await Promise.all([
      invoke("get_countdown_stats"),
      needsInfo ? invoke("get_app_info") : Promise.resolve(info),
    ]);
    if (values[1]) { info = values[1]; if (needsInfo) lastInfoTime = Date.now(); }
    if (revision !== modeRevision) { pendingSync = true; return; }
    renderStats(values[0]);
    lastSyncTime = Date.now();
    if (connectionError) { connectionError = false; notify("倒计时已重新同步"); }
  } catch (error) {
    if (!connectionError) { connectionError = true; notify(`同步失败：${String(error)}`, true); }
    $("#status-label").textContent = "等待同步";
    if (workMini) workMini.label = "等待同步";
    if (!pomodoro.active) $("#mini-label").textContent = "等待同步";
  } finally {
    syncing = false;
    if (pendingSync && !stopped) { pendingSync = false; void sync(); }
  }
}

async function openSettings() {
  if (dialog.open || pomodoroDialog.open || saving || switchingMode || !invoke || $("#settings-button").disabled) return;
  $("#settings-button").disabled = true;
  try {
    if (config?.mini_mode) await setMiniMode(false);
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

async function setMiniMode(enabled, expandedView) {
  if (saving || switchingMode || !invoke) return;
  switchingMode = true;
  closeMiniMenu();
  if (desktop && !enabled) document.body.dataset.expanding = "";
  try {
    const next = await invoke("set_mini_mode", { enabled });
    modeRevision += 1;
    if (expandedView) selectView(expandedView);
    renderConfig(next);
    void sync();
  } finally {
    delete document.body.dataset.expanding;
    switchingMode = false;
  }
}
$("#mini-button").addEventListener("click", () => { void setMiniMode(true).catch(report); });
function expand() {
  void setMiniMode(false, pomodoro.active ? "pomodoro" : undefined).catch(report);
}
$("#mini-expand").addEventListener("click", expand);
$("#mini-menu-expand").addEventListener("click", expand);
$("#mini-widget").addEventListener("dblclick", (event) => { if (!event.target.closest("button, [data-no-drag]")) expand(); });
function openMiniMenu(event) {
  event.preventDefault();
  const menu = $("#mini-menu");
  if (!menu.hidden) return;
  const widget = $("#mini-widget");
  miniMenuReturnFocus = widget.contains(document.activeElement) ? document.activeElement : widget;
  menu.hidden = false;
  $("#mini-menu-expand").focus();
  $("#mini-expand").inert = true;
}

function closeMiniMenu(restoreFocus = true) {
  const menu = $("#mini-menu");
  if (menu.hidden) return;
  menu.hidden = true;
  $("#mini-expand").inert = false;
  if (restoreFocus) miniMenuReturnFocus?.focus();
  miniMenuReturnFocus = undefined;
}

function toggleMiniModeFromTray() {
  if (pomodoroDialog.open) return;
  if (dialog.open) {
    if (!saving) {
      const message = $("#settings-error");
      message.textContent = "请先保存或取消当前设置，再切换迷你模式。";
      message.hidden = false;
    }
    return;
  }
  if (!config || $("#settings-button").disabled) return;
  if (config.mini_mode) expand();
  else void setMiniMode(true).catch(report);
}
$("#mini-widget").addEventListener("contextmenu", openMiniMenu);
$("#mini-widget").addEventListener("keydown", (event) => {
  if (event.key === "ContextMenu" || (event.shiftKey && event.key === "F10")) openMiniMenu(event);
  else if (event.key === "Enter" && event.target === $("#mini-widget")) expand();
});
$("#mini-menu-close").addEventListener("click", () => closeMiniMenu());
$("#mini-menu-settings").addEventListener("click", openSettings);
$("#mini-menu-pin").addEventListener("click", () => { closeMiniMenu(); $("#pin-button").click(); });
$("#mini-menu-hide").addEventListener("click", () => { closeMiniMenu(); void hide(); });
$("#mini-notice").addEventListener("click", () => { $("#mini-notice").hidden = true; });

function closeSettings() { if (!saving) dialog.close(); }
function updateReminderInput() {
  form.elements.namedItem("reminder_time").disabled = saving || !form.elements.namedItem("reminder_enabled").checked;
}
form.elements.namedItem("reminder_enabled").addEventListener("change", updateReminderInput);
$("#settings-button").addEventListener("click", () => {
  if (activeView === "pomodoro") openPomodoroSettings();
  else void openSettings();
});
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
  if (event.key === "Escape" && !$("#mini-menu").hidden) { event.preventDefault(); closeMiniMenu(); return; }
  if (event.key === "Escape" && !dialog.open && !pomodoroDialog.open) { event.preventDefault(); void hide(); }
});
$("#footer-label").addEventListener("click", () => {
  if (info?.warnings?.length) notify($("#footer-label").title);
});
$("#footer-label").addEventListener("keydown", (event) => {
  if (event.key === "Enter" || event.key === " ") { event.preventDefault(); $("#footer-label").click(); }
});
document.addEventListener("visibilitychange", () => {
  if (!document.hidden) { tickPomodoro(); void sync(); }
});
window.addEventListener("focus", () => { tickPomodoro(); void sync(); });

async function start() {
  try {
    invoke = desktop ? window.__TAURI__.core.invoke : await createPreviewBridge();
    if (desktop) {
      const listen = window.__TAURI__.event.listen;
      unlisteners.push(await listen("open-settings", openSettings));
      unlisteners.push(await listen("toggle-mini-mode", toggleMiniModeFromTray));
      unlisteners.push(await listen("toggle-pomodoro", () => {
        tickPomodoro();
        togglePomodoro();
      }));
      unlisteners.push(await listen("widget-shown", () => { tickPomodoro(); void sync(); }));
      unlisteners.push(await listen("config-changed", () => { lastInfoTime = 0; void sync(); }));
      unlisteners.push(await listen("app-error", ({ payload }) => notify(payload)));
    }
    syncTrayPomodoro();
    await sync();
  } catch (error) { notify(`启动失败：${String(error)}`, true); }
}

const interval = setInterval(() => {
  tickPomodoro();
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
tickPomodoro();
void start();
