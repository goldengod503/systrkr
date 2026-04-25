use std::time::Duration;

use cosmic::app::{Core, Task};
use cosmic::iced::{time, widget::canvas::Cache, Subscription};
use cosmic::Element;

use crate::history::RingBuffer;
use crate::sampler::{Sample, Sampler};

pub const HISTORY_LEN: usize = 60;
pub const TICK_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone, Debug)]
pub enum Message {
    Tick,
    TogglePopup,
    PopupClosed,
    OpenSystemMonitor,
}

pub struct App {
    core: Core,
    sampler: Sampler,
    pub(crate) cpu_history: RingBuffer<f32, HISTORY_LEN>,
    pub(crate) gpu_history: RingBuffer<f32, HISTORY_LEN>,
    pub(crate) latest: Sample,
    pub(crate) cpu_cache: Cache,
    pub(crate) gpu_cache: Cache,
    pub(crate) popup_id: Option<cosmic::iced::window::Id>,
    pub(crate) system_monitor_bin: Option<&'static str>,
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
        let app = Self {
            core,
            sampler: Sampler::new(),
            cpu_history: RingBuffer::new(),
            gpu_history: RingBuffer::new(),
            latest: Sample::default(),
            cpu_cache: Cache::default(),
            gpu_cache: Cache::default(),
            popup_id: None,
            system_monitor_bin: detect_system_monitor(),
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
                self.latest = sample;
                self.cpu_cache.clear();
                self.gpu_cache.clear();
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
        }
    }

    fn view(&self) -> Element<'_, Message> {
        use cosmic::iced::{Alignment, Length};
        use cosmic::widget::{column as col, container, mouse_area, row, text};
        use cosmic::widget::canvas::Canvas;

        use crate::widgets::Sparkline;

        let cpu_samples: Vec<f32> = self.cpu_history.iter().collect();
        let gpu_samples: Vec<f32> = self.gpu_history.iter().collect();

        let cpu_column = col::with_children(vec![
            text("CPU").size(8).into(),
            Canvas::new(Sparkline::new(cpu_samples, HISTORY_LEN, &self.cpu_cache))
                .width(Length::Fixed(40.0))
                .height(Length::Fixed(20.0))
                .into(),
        ])
        .align_x(Alignment::Center)
        .spacing(2);

        let gpu_column = col::with_children(vec![
            text("GPU").size(8).into(),
            Canvas::new(Sparkline::new(gpu_samples, HISTORY_LEN, &self.gpu_cache))
                .width(Length::Fixed(40.0))
                .height(Length::Fixed(20.0))
                .into(),
        ])
        .align_x(Alignment::Center)
        .spacing(2);

        let content = row::with_children(vec![cpu_column.into(), gpu_column.into()])
            .spacing(6)
            .align_y(Alignment::Center);

        mouse_area(container(content).padding(4))
            .on_press(Message::TogglePopup)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        time::every(TICK_INTERVAL).map(|_| Message::Tick)
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
