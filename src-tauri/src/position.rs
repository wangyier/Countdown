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
