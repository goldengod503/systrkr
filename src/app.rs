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
                // Filled in by Task 13.
                Task::none()
            }
            Message::PopupClosed => {
                self.popup_id = None;
                Task::none()
            }
            Message::OpenSystemMonitor => {
                spawn_system_monitor();
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
}

fn spawn_system_monitor() {
    use std::process::Command;
    for bin in ["cosmic-monitor", "gnome-system-monitor"] {
        if Command::new(bin)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .is_ok()
        {
            return;
        }
    }
    tracing::warn!("no system monitor binary found on PATH");
}
