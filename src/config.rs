use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};

pub const CONFIG_ID: &str = "com.goldengod503.GalaxySysTrkr";
pub const CONFIG_VERSION: u64 = 2;

#[derive(Clone, Debug, PartialEq, CosmicConfigEntry)]
#[version = 2]
pub struct SystrkrConfig {
    pub refresh_ms: u64,
    pub history_seconds: u64,
    pub warn_threshold: u8,
    pub crit_threshold: u8,
    pub show_cpu: bool,
    pub show_gpu: bool,
    pub show_ram: bool,
    pub show_net: bool,
    pub show_disk: bool,
    pub gpu_index: usize,
    pub show_ollama: bool,
    pub ollama_host: String,
}

impl Default for SystrkrConfig {
    fn default() -> Self {
        Self {
            refresh_ms: 500,
            history_seconds: 30,
            warn_threshold: 60,
            crit_threshold: 85,
            show_cpu: true,
            show_gpu: true,
            show_ram: false,
            show_net: false,
            show_disk: false,
            gpu_index: 0,
            show_ollama: false,
            ollama_host: "http://localhost:11434".into(),
        }
    }
}

impl SystrkrConfig {
    pub fn history_capacity(&self) -> usize {
        let cap = (self.history_seconds * 1000).saturating_div(self.refresh_ms.max(1)) as usize;
        cap.max(2)
    }

    pub fn refresh_duration(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.refresh_ms.max(50))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_capacity_30s_at_500ms_is_60() {
        let cfg = SystrkrConfig::default();

        assert_eq!(cfg.history_capacity(), 60);
    }

    #[test]
    fn history_capacity_30s_at_5s_is_6() {
        let cfg = SystrkrConfig {
            refresh_ms: 5000,
            history_seconds: 30,
            ..SystrkrConfig::default()
        };

        assert_eq!(cfg.history_capacity(), 6);
    }
}
