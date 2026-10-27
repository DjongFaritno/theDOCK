use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

/// Only components rendered by the first prototype are represented here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub start: bool,
    pub pinned_apps: bool,
    pub running_apps: bool,
    pub clock: bool,
    pub date: bool,
    pub calendar: bool,
    pub music: bool,
    pub cpu: bool,
    pub ram: bool,
    pub battery: bool,
    pub tray: bool,
    pub glass: bool,
    pub hide_taskbar: bool,
    pub pins: Vec<PinnedApp>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedApp {
    pub label: String,
    pub target: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            start: true,
            pinned_apps: true,
            running_apps: true,
            clock: true,
            date: true,
            calendar: true,
            music: false,
            cpu: false,
            ram: true,
            battery: true,
            tray: true,
            glass: true,
            hide_taskbar: true,
            pins: vec![PinnedApp {
                label: "Explorer".into(),
                target: "explorer.exe".into(),
            }],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    Start,
    PinnedApps,
    RunningApps,
    Clock,
    Date,
    Calendar,
    Music,
    Cpu,
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
            Setting::PinnedApps => &mut self.pinned_apps,
            Setting::RunningApps => &mut self.running_apps,
            Setting::Clock => &mut self.clock,
            Setting::Date => &mut self.date,
            Setting::Calendar => &mut self.calendar,
            Setting::Music => &mut self.music,
            Setting::Cpu => &mut self.cpu,
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
            Setting::PinnedApps => self.pinned_apps,
            Setting::RunningApps => self.running_apps,
            Setting::Clock => self.clock,
            Setting::Date => self.date,
            Setting::Calendar => self.calendar,
            Setting::Music => self.music,
            Setting::Cpu => self.cpu,
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

    pub fn pin(&mut self, label: impl Into<String>, target: impl Into<String>) -> bool {
        let target = target.into();
        if target.trim().is_empty()
            || self
                .pins
                .iter()
                .any(|pin| pin.target.eq_ignore_ascii_case(&target))
        {
            return false;
        }
        self.pins.push(PinnedApp {
            label: label.into().chars().take(32).collect(),
            target,
        });
        true
    }

    pub fn unpin(&mut self, index: usize) -> bool {
        if index >= self.pins.len() {
            return false;
        }
        self.pins.remove(index);
        true
    }

    pub fn move_pin(&mut self, index: usize, direction: isize) -> bool {
        let destination = index as isize + direction;
        if index >= self.pins.len() || destination < 0 || destination >= self.pins.len() as isize {
            return false;
        }
        self.pins.swap(index, destination as usize);
        true
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

    #[test]
    fn pinned_apps_are_unique_and_reorderable() {
        let mut settings = Settings::default();
        assert!(settings.pin("Editor", r"C:\Tools\editor.exe"));
        assert!(!settings.pin("Copy", r"c:\tools\EDITOR.exe"));
        assert!(settings.move_pin(1, -1));
        assert_eq!(settings.pins[0].label, "Editor");
        assert!(settings.unpin(0));
        assert_eq!(settings.pins.len(), 1);
    }
}
