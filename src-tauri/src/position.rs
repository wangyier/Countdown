use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct SavedPosition {
    pub x: i32,
    pub y: i32,
}

impl SavedPosition {
    pub fn fits(
        &self,
        area: &tauri::PhysicalRect<i32, u32>,
        size: tauri::PhysicalSize<u32>,
    ) -> bool {
        let x = i64::from(self.x);
        let y = i64::from(self.y);
        x >= i64::from(area.position.x)
            && y >= i64::from(area.position.y)
            && x + i64::from(size.width) <= i64::from(area.position.x) + i64::from(area.size.width)
            && y + i64::from(size.height)
                <= i64::from(area.position.y) + i64::from(area.size.height)
    }
}

pub fn restore(window: &tauri::WebviewWindow, directory: &std::path::Path) -> tauri::Result<()> {
    let size = window.outer_size()?;
    let saved = std::fs::read(directory.join("window-position.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<SavedPosition>(&bytes).ok());
    if let Some(saved) = saved {
        if window
            .available_monitors()?
            .iter()
            .any(|monitor| saved.fits(monitor.work_area(), size))
        {
            return window.set_position(tauri::PhysicalPosition::new(saved.x, saved.y));
        }
    }
    if let Some(monitor) = window.primary_monitor()? {
        let area = monitor.work_area();
        let margin = (24.0 * monitor.scale_factor()).round() as i64;
        let x = i64::from(area.position.x)
            + (i64::from(area.size.width) - i64::from(size.width) - margin).max(0);
        let y = i64::from(area.position.y)
            + (i64::from(area.size.height) - i64::from(size.height) - margin).max(0);
        window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32))?;
    }
    Ok(())
}

pub fn mode_size(mini: bool) -> tauri::LogicalSize<f64> {
    if mini {
        tauri::LogicalSize::new(220.0, 104.0)
    } else {
        tauri::LogicalSize::new(360.0, 480.0)
    }
}

// Preserve the bottom-right corner when switching sizes, then keep the window reachable.
pub fn resize(window: &tauri::WebviewWindow, mini: bool) -> tauri::Result<()> {
    let old_position = window.outer_position()?;
    let old_size = window.outer_size()?;
    let monitor = window.current_monitor()?;
    window.set_size(mode_size(mini))?;
    let size = window.outer_size()?;
    let mut x = i64::from(old_position.x) + i64::from(old_size.width) - i64::from(size.width);
    let mut y = i64::from(old_position.y) + i64::from(old_size.height) - i64::from(size.height);
    if let Some(monitor) = monitor {
        let area = monitor.work_area();
        let left = i64::from(area.position.x);
        let top = i64::from(area.position.y);
        x = x.clamp(
            left,
            left + i64::from(area.size.width.saturating_sub(size.width)),
        );
        y = y.clamp(
            top,
            top + i64::from(area.size.height.saturating_sub(size.height)),
        );
    }
    window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_position_must_fit_current_monitor_work_area() {
        let area = tauri::PhysicalRect {
            position: tauri::PhysicalPosition::new(-1920, 0),
            size: tauri::PhysicalSize::new(1920, 1040),
        };
        let size = tauri::PhysicalSize::new(360, 480);
        assert!(SavedPosition { x: -400, y: 500 }.fits(&area, size));
        assert!(!SavedPosition { x: 100, y: 500 }.fits(&area, size));
        assert!(!SavedPosition { x: -400, y: 600 }.fits(&area, size));
    }
}
