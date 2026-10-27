use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

/// Only components rendered by the first prototype are represented here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub start: bool,
    pub clock: bool,
    pub date: bool,
    pub ram: bool,
    pub battery: bool,
    pub tray: bool,
    pub glass: bool,
    pub hide_taskbar: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            start: true,
            clock: true,
            date: true,
            ram: true,
            battery: true,
            tray: true,
            glass: true,
            hide_taskbar: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    Start,
    Clock,
    Date,
    Ram,
    Battery,
    Tray,
    Glass,
    HideTaskbar,
}

impl Settings {
    pub fn toggle(&mut self, setting: Setting) {
        let value = match setting {
            Setting::Start => &mut self.start,
            Setting::Clock => &mut self.clock,
            Setting::Date => &mut self.date,
            Setting::Ram => &mut self.ram,
            Setting::Battery => &mut self.battery,
            Setting::Tray => &mut self.tray,
            Setting::Glass => &mut self.glass,
            Setting::HideTaskbar => &mut self.hide_taskbar,
        };
        *value = !*value;
    }

    pub fn enabled(&self, setting: Setting) -> bool {
        match setting {
            Setting::Start => self.start,
            Setting::Clock => self.clock,
            Setting::Date => self.date,
            Setting::Ram => self.ram,
            Setting::Battery => self.battery,
            Setting::Tray => self.tray,
            Setting::Glass => self.glass,
            Setting::HideTaskbar => self.hide_taskbar,
        }
    }

    pub fn load(path: &Path) -> Self {
        fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, bytes)?;
        fs::rename(temporary, path)
    }
}

#[cfg(test)]
mod tests {
    use super::{Setting, Settings};

    #[test]
    fn toggled_settings_survive_serialization() {
        let mut settings = Settings::default();
        settings.toggle(Setting::Tray);
        settings.toggle(Setting::HideTaskbar);
        let encoded = serde_json::to_string(&settings).unwrap();
        let decoded: Settings = serde_json::from_str(&encoded).unwrap();
        assert!(!decoded.tray);
        assert!(!decoded.hide_taskbar);
        assert!(decoded.clock);
    }

    #[test]
    fn older_settings_gain_new_defaults() {
        let decoded: Settings = serde_json::from_str(r#"{"clock":false}"#).unwrap();
        assert!(!decoded.clock);
        assert!(decoded.hide_taskbar);
        assert!(decoded.tray);
        assert!(decoded.start);
    }

    #[test]
    fn saving_replaces_previous_settings() {
        let path = std::env::temp_dir().join(format!(
            "thedock-settings-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut settings = Settings::default();
        settings.save(&path).unwrap();
        settings.toggle(Setting::HideTaskbar);
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path), settings);
        std::fs::remove_file(path).unwrap();
    }
}
