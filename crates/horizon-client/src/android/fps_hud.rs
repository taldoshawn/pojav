//! Preferences shared with the native FPS overlay; no network or Java mod is needed.
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub struct Preferences {
    pub enabled: bool,
    pub corner: usize,
    pub lows: bool,
}
impl Preferences {
    const CORNERS: [&str; 4] = ["top_left", "top_right", "bottom_left", "bottom_right"];
    pub fn directory(profile: &Path) -> PathBuf {
        profile.join("horizon-mods/.horizon-mod-data/fps_overlay")
    }
    pub fn load(profile: &Path, installed_and_enabled: bool) -> Self {
        let directory = Self::directory(profile);
        let read = |key| {
            std::fs::read_to_string(directory.join(key))
                .unwrap_or_default()
                .trim()
                .to_owned()
        };
        let flag = |value: &str, default| match value {
            "true" => true,
            "false" => false,
            _ => default,
        };
        let corner = read("corner");
        Self {
            enabled: installed_and_enabled && flag(&read("enabled"), true),
            corner: Self::CORNERS
                .iter()
                .position(|name| *name == corner)
                .unwrap_or(0),
            lows: flag(&read("show_1pct_low"), true),
        }
    }
    pub fn corner_label(&self) -> &'static str {
        ["Top left", "Top right", "Bottom left", "Bottom right"][self.corner % 4]
    }
    pub fn save(&self, profile: &Path) -> std::io::Result<()> {
        let directory = Self::directory(profile);
        std::fs::create_dir_all(&directory)?;
        for (key, value) in [
            ("enabled", if self.enabled { "true" } else { "false" }),
            ("corner", Self::CORNERS[self.corner % 4]),
            ("show_1pct_low", if self.lows { "true" } else { "false" }),
        ] {
            let temp = directory.join(format!("{key}.partial"));
            std::fs::write(&temp, value)?;
            std::fs::rename(temp, directory.join(key))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_round_trip_and_stay_in_the_selected_profile() {
        let root = std::env::temp_dir().join(format!(
            "pojav-fps-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let one = root.join("one");
        let two = root.join("two");
        let mut prefs = Preferences::load(&one, false);
        assert!(!prefs.enabled);
        prefs.enabled = true;
        prefs.corner = 3;
        prefs.lows = false;
        prefs.save(&one).unwrap();
        assert_eq!(Preferences::load(&one, true), prefs);
        assert!(Preferences::load(&two, true).lows); // untouched defaults
        assert!(!Preferences::load(&one, false).enabled); // disabled mod cannot be enabled by stale settings
        let dir = Preferences::directory(&one);
        std::fs::write(dir.join("corner"), "../../elsewhere").unwrap();
        assert_eq!(Preferences::load(&one, true).corner, 0);
        assert!(!dir.join("enabled.partial").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
