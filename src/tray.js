document.addEventListener("contextmenu", (event) => event.preventDefault());

const panel = document.querySelector("#panel");
const ip = document.querySelector("#ip");
const rows = ["pomodoro", "work", "lunch"].map(key => {
  const section = document.getElementById(key);
  return {
    key, section,
    detail: section.querySelector("small"),
    time: section.querySelector("b"),
    progress: section.querySelector("progress"),
  };
});

function render(data) {
  ip.textContent = data.ip ?? "";
  ip.hidden = data.ip == null;
  for (const row of rows) {
    const value = data[row.key];
    row.section.hidden = !value;
    if (value) {
      row.detail.textContent = value.detail;
      row.time.textContent = value.time;
      row.progress.value = value.progress;
    }
  }
  panel.hidden = false;
}

await window.__TAURI__?.event.listen("tray-panel", ({ payload }) => render(payload));
