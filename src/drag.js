const INTERACTIVE = "button, a, input, select, textarea, label, dialog, [role='button'], [data-no-drag]";

export function attachWindowDrag(region, { startDragging, onError }) {
  const onMouseDown = (event) => {
    if (event.defaultPrevented || event.button !== 0 || event.buttons !== 1 || event.detail > 1) return;
    if (event.target?.closest(INTERACTIVE)) return;
    event.preventDefault();
    try {
      Promise.resolve(startDragging()).catch(onError);
    } catch (error) {
      onError(error);
    }
  };
  region.addEventListener("mousedown", onMouseDown);
  return () => region.removeEventListener("mousedown", onMouseDown);
}

// Start native dragging only after movement, so stationary double clicks can expand mini mode.
export function attachMiniDrag(region, { startDragging, onError }) {
  let origin = null;
  const down = (event) => {
    if (event.button !== 0 || event.defaultPrevented || event.target?.closest(INTERACTIVE)) return;
    origin = { x: event.clientX, y: event.clientY };
  };
  const move = (event) => {
    if (!origin) return;
    if (!(event.buttons & 1)) { origin = null; return; }
    if (Math.hypot(event.clientX - origin.x, event.clientY - origin.y) < 4) return;
    origin = null;
    try { Promise.resolve(startDragging()).catch(onError); } catch (error) { onError(error); }
  };
  const reset = () => { origin = null; };
  const handlers = { mousedown: down, mousemove: move, mouseup: reset, mouseleave: reset };
  for (const [type, handler] of Object.entries(handlers)) region.addEventListener(type, handler);
  return () => { for (const [type, handler] of Object.entries(handlers)) region.removeEventListener(type, handler); };
}
