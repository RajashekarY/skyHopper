use bevy::prelude::*;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Quality {
    Low,
    #[default]
    Medium,
    High,
}

impl Quality {
    pub const ALL: [Quality; 3] = [Quality::Low, Quality::Medium, Quality::High];

    pub fn name(self) -> &'static str {
        match self {
            Quality::Low => "low",
            Quality::Medium => "medium",
            Quality::High => "high",
        }
    }
}

#[derive(Resource, Clone, PartialEq, Debug)]
pub struct Prefs {
    pub quality: Quality,
    pub ask_on_launch: bool,
    pub vsync: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self { quality: Quality::Medium, ask_on_launch: true, vsync: false }
    }
}

impl Prefs {
    /// Unknown keys or bad values fall back to the default for that field.
    pub fn parse(text: &str) -> Self {
        let mut p = Self::default();
        for line in text.lines() {
            match line.trim().split_once('=') {
                Some(("quality", v)) => {
                    if let Some(q) = Quality::ALL.into_iter().find(|q| q.name() == v) {
                        p.quality = q;
                    }
                }
                Some(("ask_on_launch", v)) => p.ask_on_launch = v != "false",
                Some(("vsync", v)) => p.vsync = v == "true",
                _ => {}
            }
        }
        p
    }

    pub fn to_text(&self) -> String {
        format!(
            "quality={}\nask_on_launch={}\nvsync={}\n",
            self.quality.name(),
            self.ask_on_launch,
            self.vsync
        )
    }
}

fn path() -> Option<PathBuf> {
    directories::ProjectDirs::from("dev", "skyhop", "Skyhop").map(|d| d.config_dir().join("prefs.txt"))
}

pub fn load() -> Prefs {
    path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| Prefs::parse(&s))
        .unwrap_or_default()
}

pub fn save(prefs: &Prefs) {
    let Some(path) = path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = std::fs::write(&path, prefs.to_text()) {
        warn!("could not save preferences to {}: {e}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let p = Prefs { quality: Quality::High, ask_on_launch: false, vsync: true };
        assert_eq!(Prefs::parse(&p.to_text()), p);
    }

    #[test]
    fn garbage_gives_defaults() {
        assert_eq!(Prefs::parse("quality=ultra\n\u{0}junk\nvsync=maybe"), Prefs::default());
        assert_eq!(Prefs::parse(""), Prefs::default());
    }
}
