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
