import { EVENT_STORAGE_KEY, WEEKDAYS, loadEvents } from "./events.js";

export function setupEventSettings(onSave) {
  let settings = loadEvents(localStorage);
  const dialog = document.querySelector("#event-settings");
  const form = document.querySelector("#event-form");
  const fields = form.elements;
  const list = document.querySelector("#custom-events");
  const error = document.querySelector("#event-error");
  const template = document.querySelector("#custom-event-template");
  const rows = () => [...list.children];
  const field = (row, name) => row.querySelector(`[name="${name}"]`);
  WEEKDAYS.forEach((label, index) => fields.weekday.add(new Option(label, index)));

  function updatePriorities(selected = fields.priority.value) {
    fields.priority.replaceChildren(new Option("自动选最近的事件", ""));
    for (const [id, label] of [["meal", "开饭倒计时"], ["weekend", "周末倒计时"], ["payday", "发薪日"]]) {
      fields.priority.add(new Option(label, id));
    }
    for (const row of rows()) {
      fields.priority.add(new Option(field(row, "eventTitle").value.trim() || "自定义事件", row.dataset.id));
    }
    fields.priority.value = [...fields.priority.options].some(option => option.value === selected) ? selected : "";
  }

  function addRow(event = {}) {
    const row = template.content.firstElementChild.cloneNode(true);
    row.dataset.id = event.id ?? crypto.randomUUID();
    row.dataset.createdAt = Number.isFinite(event.createdAt) ? event.createdAt : Date.now();
    field(row, "eventTitle").value = event.title ?? "";
    field(row, "eventDate").value = event.date ?? "";
    field(row, "eventTime").value = event.time ?? "00:00";
    field(row, "eventYearly").checked = event.yearly === true;
    field(row, "eventTitle").addEventListener("input", () => updatePriorities());
    row.querySelector("[data-remove-event]").addEventListener("click", () => {
      row.remove();
      updatePriorities();
      document.querySelector("#add-event").focus();
    });
    list.append(row);
    return row;
  }

  document.querySelector("#open-events").addEventListener("click", () => {
    fields.weekend.checked = settings.weekend.enabled;
    fields.weekday.value = settings.weekend.weekday;
    fields.weekendTime.value = settings.weekend.time;
    fields.payday.checked = settings.payday.enabled;
    fields.paydayDate.value = settings.payday.day;
    list.replaceChildren();
    settings.custom.forEach(addRow);
    updatePriorities(settings.priority);
    error.hidden = true;
    dialog.showModal();
  });
  for (const id of ["close-events", "cancel-events"]) {
    document.getElementById(id).addEventListener("click", () => dialog.close());
  }
  document.querySelector("#add-event").addEventListener("click", () => {
    const row = addRow();
    updatePriorities();
    field(row, "eventTitle").focus();
  });
  form.addEventListener("submit", event => {
    event.preventDefault();
    const custom = rows().map(row => ({
      id: row.dataset.id, title: field(row, "eventTitle").value.trim(),
      date: field(row, "eventDate").value, time: field(row, "eventTime").value,
      yearly: field(row, "eventYearly").checked, createdAt: Number(row.dataset.createdAt),
    }));
    if (custom.some(item => !item.title)) {
      error.textContent = "请输入自定义事件名称";
      error.hidden = false;
      return;
    }
    const next = {
      weekend: { enabled: fields.weekend.checked, weekday: Number(fields.weekday.value), time: fields.weekendTime.value },
      payday: { enabled: fields.payday.checked, day: fields.paydayDate.valueAsNumber },
      custom, priority: fields.priority.value,
    };
    try {
      localStorage.setItem(EVENT_STORAGE_KEY, JSON.stringify(next));
    } catch {
      error.textContent = "事件设置保存失败，请重试";
      error.hidden = false;
      return;
    }
    settings = next;
    dialog.close();
    onSave();
  });
  return { get settings() { return settings; }, dialog };
}
