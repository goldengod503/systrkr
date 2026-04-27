use cosmic::app::{Core, Task};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{time, widget::canvas::Cache, Subscription};
use cosmic::Element;

use crate::config::{SystrkrConfig, CONFIG_ID, CONFIG_VERSION};
use crate::history::RingBuf;
use crate::sampler::{Sample, Sampler};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Metric {
    Cpu,
    Gpu,
    Ram,
    Net,
    Disk,
}

#[derive(Clone, Debug)]
pub enum Message {
    Tick,
    TogglePopup,
    PopupClosed,
    OpenSystemMonitor,
    ConfigUpdated(SystrkrConfig),
    ToggleSettings,
    SetRefreshMs(u64),
    SetHistorySeconds(u64),
    SetWarnThreshold(u8),
    SetCritThreshold(u8),
    SetShowMetric(Metric, bool),
    SetGpuIndex(usize),
}

pub struct App {
    core: Core,
    sampler: Sampler,
    pub(crate) cpu_history: RingBuf<f32>,
    pub(crate) gpu_history: RingBuf<f32>,
    pub(crate) ram_history: RingBuf<f32>,
    pub(crate) ram_cache: Cache,
    pub(crate) net_history: RingBuf<f32>,
    pub(crate) net_cache: Cache,
    pub(crate) disk_history: RingBuf<f32>,
    pub(crate) disk_cache: Cache,
    pub(crate) latest: Sample,
    pub(crate) cpu_cache: Cache,
    pub(crate) gpu_cache: Cache,
    pub(crate) popup_id: Option<cosmic::iced::window::Id>,
    pub(crate) system_monitor_bin: Option<&'static str>,
    pub(crate) config: SystrkrConfig,
    pub(crate) settings_open: bool,
}

impl cosmic::Application for App {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "com.system76.SysTrkr";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: ()) -> (Self, Task<Message>) {
        let config = cosmic_config::Config::new(CONFIG_ID, CONFIG_VERSION)
            .ok()
            .and_then(|c| SystrkrConfig::get_entry(&c).ok())
            .unwrap_or_default();
        let cap = config.history_capacity();
        let sampler = Sampler::new(&config);
        let app = Self {
            core,
            sampler,
            cpu_history: RingBuf::new(cap),
            gpu_history: RingBuf::new(cap),
            ram_history: RingBuf::new(cap),
            ram_cache: Cache::default(),
            net_history: RingBuf::new(cap),
            net_cache: Cache::default(),
            disk_history: RingBuf::new(cap),
            disk_cache: Cache::default(),
            latest: Sample::default(),
            cpu_cache: Cache::default(),
            gpu_cache: Cache::default(),
            popup_id: None,
            system_monitor_bin: detect_system_monitor(),
            config,
            settings_open: false,
        };
        (app, Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                let sample = self.sampler.tick();
                if let Some(v) = sample.cpu.utilization_pct {
                    self.cpu_history.push(v);
                }
                if let Some(v) = sample.gpu.utilization_pct {
                    self.gpu_history.push(v);
                }
                if let (Some(used), Some(total)) =
                    (sample.cpu.ram_used_bytes, sample.cpu.ram_total_bytes)
                {
                    if total > 0 {
                        self.ram_history.push((used as f32 / total as f32) * 100.0);
                    }
                }
                let net_combined = sample.net.rx_bps.saturating_add(sample.net.tx_bps);
                self.net_history.push(net_combined as f32);
                let disk_combined = sample.disk.read_bps.saturating_add(sample.disk.write_bps);
                self.disk_history.push(disk_combined as f32);
                self.latest = sample;
                self.cpu_cache.clear();
                self.gpu_cache.clear();
                self.ram_cache.clear();
                self.net_cache.clear();
                self.disk_cache.clear();
                Task::none()
            }
            Message::TogglePopup => {
                use cosmic::iced::window::Id;
                use cosmic::surface::action::{app_popup, destroy_popup};

                if let Some(id) = self.popup_id.take() {
                    return cosmic::task::message(cosmic::Action::Cosmic(
                        cosmic::app::Action::Surface(destroy_popup(id)),
                    ));
                }

                return cosmic::task::message(cosmic::Action::Cosmic(
                    cosmic::app::Action::Surface(app_popup::<App>(
                        |state: &mut App| {
                            let new_id = Id::unique();
                            state.popup_id = Some(new_id);
                            state.core.applet.get_popup_settings(
                                state.core.main_window_id().unwrap(),
                                new_id,
                                Some((300, 360)),
                                None,
                                None,
                            )
                        },
                        Some(Box::new(|state: &App| {
                            crate::popup::view(state).map(cosmic::Action::App)
                        })),
                    )),
                ));
            }
            Message::PopupClosed => {
                self.popup_id = None;
                Task::none()
            }
            Message::OpenSystemMonitor => {
                spawn_system_monitor(self.system_monitor_bin);
                Task::none()
            }
            Message::ConfigUpdated(new_cfg) => {
                let cap = new_cfg.history_capacity();
                if cap != self.cpu_history.capacity() {
                    self.cpu_history.resize(cap);
                    self.gpu_history.resize(cap);
                    self.ram_history.resize(cap);
                    self.net_history.resize(cap);
                    self.disk_history.resize(cap);
                }
                self.config = new_cfg;
                self.cpu_cache.clear();
                self.gpu_cache.clear();
                self.ram_cache.clear();
                self.net_cache.clear();
                self.disk_cache.clear();
                Task::none()
            }
            Message::ToggleSettings => {
                self.settings_open = !self.settings_open;
                Task::none()
            }
            Message::SetRefreshMs(ms) => {
                self.config.refresh_ms = ms;
                self.persist();
                Task::none()
            }
            Message::SetHistorySeconds(sec) => {
                self.config.history_seconds = sec;
                self.persist();
                Task::none()
            }
            Message::SetWarnThreshold(t) => {
                self.config.warn_threshold = t;
                self.persist();
                Task::none()
            }
            Message::SetCritThreshold(t) => {
                self.config.crit_threshold = t;
                self.persist();
                Task::none()
            }
            Message::SetShowMetric(m, on) => {
                match m {
                    Metric::Cpu => self.config.show_cpu = on,
                    Metric::Gpu => self.config.show_gpu = on,
                    Metric::Ram => self.config.show_ram = on,
                    Metric::Net => self.config.show_net = on,
                    Metric::Disk => self.config.show_disk = on,
                }
                self.persist();
                Task::none()
            }
            Message::SetGpuIndex(i) => {
                self.config.gpu_index = i;
                self.persist();
                self.sampler = Sampler::new(&self.config);
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        use cosmic::iced::{Alignment, Length};
        use cosmic::widget::{autosize, column as col, container, mouse_area, row, text, Id};
        use cosmic::widget::canvas::Canvas;

        use crate::widgets::Sparkline;

        let theme = cosmic::theme::active();
        let cpu_color = threshold_color(&theme, &self.config, self.latest.cpu.utilization_pct);
        let gpu_color = threshold_color(&theme, &self.config, self.latest.gpu.utilization_pct);

        let cpu_samples: Vec<f32> = self.cpu_history.iter().collect();
        let gpu_samples: Vec<f32> = self.gpu_history.iter().collect();

        let cpu_pct = self
            .latest
            .cpu
            .utilization_pct
            .map(|v| format!("CPU {v:.0}%"))
            .unwrap_or_else(|| "CPU —".into());
        let gpu_pct = self
            .latest
            .gpu
            .utilization_pct
            .map(|v| format!("GPU {v:.0}%"))
            .unwrap_or_else(|| "GPU —".into());

        let cap = self.config.history_capacity();

        let cpu_column = col::with_children(vec![
            text(cpu_pct).size(10).into(),
            Canvas::new(
                Sparkline::new(cpu_samples, cap, &self.cpu_cache).tint(cpu_color),
            )
            .width(Length::Fixed(48.0))
            .height(Length::Fixed(20.0))
            .into(),
        ])
        .align_x(Alignment::Center)
        .spacing(2);

        let gpu_column = col::with_children(vec![
            text(gpu_pct).size(10).into(),
            Canvas::new(
                Sparkline::new(gpu_samples, cap, &self.gpu_cache).tint(gpu_color),
            )
            .width(Length::Fixed(48.0))
            .height(Length::Fixed(20.0))
            .into(),
        ])
        .align_x(Alignment::Center)
        .spacing(2);

        let mut columns: Vec<Element<'_, Message>> = Vec::new();

        if self.config.show_cpu {
            columns.push(cpu_column.into());
        }
        if self.config.show_gpu {
            columns.push(gpu_column.into());
        }

        if self.config.show_ram {
            let ram_pct = match (
                self.latest.cpu.ram_used_bytes,
                self.latest.cpu.ram_total_bytes,
            ) {
                (Some(used), Some(total)) if total > 0 => {
                    Some((used as f32 / total as f32) * 100.0)
                }
                _ => None,
            };
            let ram_color = threshold_color(&theme, &self.config, ram_pct);
            let ram_label = ram_pct
                .map(|v| format!("RAM {v:.0}%"))
                .unwrap_or_else(|| "RAM —".into());
            let ram_samples: Vec<f32> = self.ram_history.iter().collect();
            let ram_column = col::with_children(vec![
                text(ram_label).size(10).into(),
                Canvas::new(
                    Sparkline::new(ram_samples, cap, &self.ram_cache).tint(ram_color),
                )
                .width(Length::Fixed(48.0))
                .height(Length::Fixed(20.0))
                .into(),
            ])
            .align_x(Alignment::Center)
            .spacing(2);
            columns.push(ram_column.into());
        }

        if self.config.show_net {
            let net_total = self.latest.net.rx_bps.saturating_add(self.latest.net.tx_bps);
            let net_color = threshold_color(&theme, &self.config, None);
            let net_label = format!("NET {}", fmt_bps(net_total));
            let net_samples: Vec<f32> = self.net_history.iter().collect();
            let net_column = col::with_children(vec![
                text(net_label).size(10).into(),
                Canvas::new(
                    Sparkline::new(net_samples, cap, &self.net_cache)
                        .tint(net_color)
                        .scale(crate::widgets::sparkline::Scale::AutoMax),
                )
                .width(Length::Fixed(48.0))
                .height(Length::Fixed(20.0))
                .into(),
            ])
            .align_x(Alignment::Center)
            .spacing(2);
            columns.push(net_column.into());
        }

        if self.config.show_disk {
            let disk_total = self
                .latest
                .disk
                .read_bps
                .saturating_add(self.latest.disk.write_bps);
            let disk_color = threshold_color(&theme, &self.config, None);
            let disk_label = format!("DSK {}", fmt_bps(disk_total));
            let disk_samples: Vec<f32> = self.disk_history.iter().collect();
            let disk_column = col::with_children(vec![
                text(disk_label).size(10).into(),
                Canvas::new(
                    Sparkline::new(disk_samples, cap, &self.disk_cache)
                        .tint(disk_color)
                        .scale(crate::widgets::sparkline::Scale::AutoMax),
                )
                .width(Length::Fixed(48.0))
                .height(Length::Fixed(20.0))
                .into(),
            ])
            .align_x(Alignment::Center)
            .spacing(2);
            columns.push(disk_column.into());
        }

        let content = row::with_children(columns)
            .spacing(8)
            .align_y(Alignment::Center);

        let button = mouse_area(container(content).padding(4))
            .on_press(Message::TogglePopup);

        autosize::autosize(button, Id::new("systrkr-applet")).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        use cosmic::cosmic_config::config_subscription;

        let tick = time::every(self.config.refresh_duration()).map(|_| Message::Tick);
        let cfg = config_subscription::<_, SystrkrConfig>(0u8, CONFIG_ID.into(), CONFIG_VERSION)
            .map(|update| Message::ConfigUpdated(update.config));
        Subscription::batch([tick, cfg])
    }

    fn view_window(&self, _id: cosmic::iced::window::Id) -> Element<'_, Message> {
        crate::popup::view(self)
    }

    fn on_close_requested(&self, _id: cosmic::iced::window::Id) -> Option<Message> {
        Some(Message::PopupClosed)
    }
}

impl App {
    pub fn gpu_name(&self) -> &str {
        self.sampler.gpu_name()
    }

    pub fn gpu_proc_backend_available(&self) -> bool {
        self.sampler.gpu_proc_backend_available()
    }

    fn persist(&self) {
        if let Ok(handle) = cosmic_config::Config::new(CONFIG_ID, CONFIG_VERSION) {
            let _ = self.config.write_entry(&handle);
        }
    }
}

fn detect_system_monitor() -> Option<&'static str> {
    let candidates = ["cosmic-monitor", "gnome-system-monitor"];
    for bin in candidates {
        if which(bin) {
            return Some(bin);
        }
    }
    None
}

fn which(bin: &str) -> bool {
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    for dir in path.split(':') {
        let p = std::path::PathBuf::from(dir).join(bin);
        if p.is_file() {
            return true;
        }
    }
    false
}

fn spawn_system_monitor(bin: Option<&'static str>) {
    use std::process::Command;
    let Some(bin) = bin else {
        tracing::warn!("OpenSystemMonitor pressed but no system monitor binary detected");
        return;
    };
    if let Err(e) = Command::new(bin)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        tracing::warn!(error = %e, bin, "failed to spawn system monitor");
    }
}

fn fmt_bps(bps: u64) -> String {
    const M: f64 = 1_000_000.0;
    const K: f64 = 1_000.0;
    let f = bps as f64;
    if f >= M {
        format!("{:.1}M", f / M)
    } else if f >= K {
        format!("{:.0}K", f / K)
    } else {
        format!("{}", bps)
    }
}

fn threshold_color(
    theme: &cosmic::Theme,
    cfg: &SystrkrConfig,
    value: Option<f32>,
) -> cosmic::iced::Color {
    use cosmic::iced::Color;
    let cosmic = theme.cosmic();
    let palette = match value {
        Some(v) if v >= cfg.crit_threshold as f32 => cosmic.destructive_color(),
        Some(v) if v >= cfg.warn_threshold as f32 => cosmic.warning_color(),
        _ => cosmic.accent_color(),
    };
    Color {
        r: palette.red,
        g: palette.green,
        b: palette.blue,
        a: 1.0,
    }
}
