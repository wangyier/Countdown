use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct WorkConfig {
    pub start_time: String,
    pub end_time: String,
    pub lunch_start_time: String,
    pub lunch_end_time: String,
    pub autostart_enabled: bool,
    pub always_on_top: bool,
    pub reminder_enabled: bool,
    pub reminder_time: String,
}

impl Default for WorkConfig {
    fn default() -> Self {
        Self {
            start_time: "09:00".into(),
            end_time: "17:30".into(),
            lunch_start_time: "11:45".into(),
            lunch_end_time: "13:15".into(),
            autostart_enabled: true,
            always_on_top: false,
            reminder_enabled: true,
            reminder_time: "10:35".into(),
        }
    }
}

impl WorkConfig {
    pub fn reminder_time(&self) -> Result<NaiveTime, String> {
        let value = &self.reminder_time;
        if value.len() != 5
            || value.as_bytes()[2] != b':'
            || ![0, 1, 3, 4]
                .into_iter()
                .all(|i| value.as_bytes()[i].is_ascii_digit())
        {
            return Err("外卖提醒时间请使用 HH:MM 格式".into());
        }
        NaiveTime::parse_from_str(value, "%H:%M")
            .map_err(|_| "请输入有效的外卖提醒时间（00:00—23:59）".into())
    }

    pub fn times(&self) -> Result<[NaiveTime; 4], String> {
        self.reminder_time()?;
        let parse = |value: &str| {
            if value.len() != 5
                || value.as_bytes()[2] != b':'
                || ![0, 1, 3, 4]
                    .into_iter()
                    .all(|index| value.as_bytes()[index].is_ascii_digit())
            {
                return Err("时间请使用 HH:MM 格式".to_string());
            }
            NaiveTime::parse_from_str(value, "%H:%M")
                .map_err(|_| "请输入有效时间（00:00—23:59）".to_string())
        };
        let times = [
            parse(&self.start_time)?,
            parse(&self.lunch_start_time)?,
            parse(&self.lunch_end_time)?,
            parse(&self.end_time)?,
        ];
        if !times.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err("时间顺序须为：上班 < 午休开始 < 午休结束 < 下班".into());
        }
        Ok(times)
    }
}

pub fn load(path: &Path) -> Result<WorkConfig, String> {
    let config = match fs::read(path) {
        Ok(bytes) => serde_json::from_slice::<WorkConfig>(&bytes)
            .map_err(|e| format!("配置文件读取失败：{e}"))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => WorkConfig::default(),
        Err(e) => return Err(format!("配置文件读取失败：{e}")),
    };
    config.times()?;
    Ok(config)
}

// Write and flush a sibling temporary file before replacing the saved configuration.
pub fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    let parent = path.parent().ok_or("无效的配置路径")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temporary = path.with_extension("tmp");
    let result = (|| {
        let mut file = fs::File::create(&temporary).map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        fs::rename(&temporary, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_configs_receive_reminder_defaults_and_new_settings_roundtrip() {
        let old: WorkConfig = serde_json::from_str(r#"{"start_time":"08:30"}"#).unwrap();
        assert!(old.reminder_enabled);
        assert_eq!(old.reminder_time, "10:35");
        let changed = WorkConfig {
            reminder_enabled: false,
            reminder_time: "12:20".into(),
            ..old
        };
        let restored: WorkConfig =
            serde_json::from_str(&serde_json::to_string(&changed).unwrap()).unwrap();
        assert_eq!(changed, restored);
        for value in ["", "9:00", "24:00", "10:60", "10:35:00"] {
            let config = WorkConfig {
                reminder_time: value.into(),
                ..WorkConfig::default()
            };
            assert!(config.times().is_err(), "{value}");
        }
    }

    #[test]
    fn rejects_invalid_and_cross_day_schedules() {
        for end in ["08:00", "13:15", "25:00", "9:00", "oops"] {
            let config = WorkConfig {
                end_time: end.into(),
                ..WorkConfig::default()
            };
            assert!(config.times().is_err(), "{end}");
        }
        assert!(WorkConfig::default().times().is_ok());
    }
}
