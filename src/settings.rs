use cosmic::iced::Length;
use cosmic::widget::{column as col, dropdown, row, slider, text, text_input, toggler};
use cosmic::Element;

use crate::app::{App, Message, Metric};
use crate::sampler::gpu;

pub fn view(app: &App) -> Element<'_, Message> {
    let cfg = &app.config;

    let refresh_options = [500u64, 1000, 2000, 5000];
    let refresh_idx = refresh_options
        .iter()
        .position(|v| *v == cfg.refresh_ms)
        .unwrap_or(0);
    let refresh_labels: Vec<String> = refresh_options
        .iter()
        .map(|ms| format!("{:.1} s", *ms as f32 / 1000.0))
        .collect();
    let refresh_picker = dropdown(refresh_labels, Some(refresh_idx), move |i| {
        Message::SetRefreshMs(refresh_options[i])
    });

    let history_options = [15u64, 30, 60, 120];
    let history_idx = history_options
        .iter()
        .position(|v| *v == cfg.history_seconds)
        .unwrap_or(1);
    let history_labels: Vec<String> = history_options
        .iter()
        .map(|s| format!("{} s", s))
        .collect();
    let history_picker = dropdown(history_labels, Some(history_idx), move |i| {
        Message::SetHistorySeconds(history_options[i])
    });

    let gpu_infos = gpu::enumerate();
    let gpu_labels: Vec<String> = if gpu_infos.is_empty() {
        vec!["No GPU detected".into()]
    } else {
        gpu_infos.iter().map(|g| g.name.clone()).collect()
    };
    let gpu_idx = if gpu_infos.is_empty() {
        Some(0)
    } else {
        Some(cfg.gpu_index.min(gpu_infos.len().saturating_sub(1)))
    };
    let gpu_picker = dropdown(gpu_labels, gpu_idx, Message::SetGpuIndex);

    let warn_slider = slider(0.0..=100.0, cfg.warn_threshold as f32, |v| {
        Message::SetWarnThreshold(v as u8)
    });
    let crit_slider = slider(0.0..=100.0, cfg.crit_threshold as f32, |v| {
        Message::SetCritThreshold(v as u8)
    });

    let mut children: Vec<Element<'_, Message>> = vec![
        text("Settings").size(13).into(),
        labeled("Refresh", refresh_picker.into()),
        labeled("History", history_picker.into()),
        labeled("GPU", gpu_picker.into()),
        labeled(format!("Warn ≥ {}%", cfg.warn_threshold), warn_slider.into()),
        labeled(format!("Crit ≥ {}%", cfg.crit_threshold), crit_slider.into()),
        metric_row("Show CPU", Metric::Cpu, cfg.show_cpu),
        metric_row("Show GPU", Metric::Gpu, cfg.show_gpu),
        metric_row("Show RAM", Metric::Ram, cfg.show_ram),
        metric_row("Show Network", Metric::Net, cfg.show_net),
        metric_row("Show Disk", Metric::Disk, cfg.show_disk),
        metric_row("Show Ollama", Metric::Ollama, cfg.show_ollama),
    ];

    if cfg.show_ollama {
        let host_input = text_input("http://localhost:11434", &cfg.ollama_host)
            .on_input(Message::SetOllamaHost)
            .width(Length::Fill);
        children.push(labeled("Host", host_input.into()));
    }

    col::with_children(children).spacing(6).into()
}

fn labeled<'a>(label: impl Into<String>, control: Element<'a, Message>) -> Element<'a, Message> {
    row::with_children(vec![
        text(label.into()).size(11).width(Length::Fixed(80.0)).into(),
        control,
    ])
    .into()
}

fn metric_row<'a>(label: &'static str, m: Metric, on: bool) -> Element<'a, Message> {
    row::with_children(vec![
        text(label).size(11).width(Length::Fill).into(),
        toggler(on)
            .on_toggle(move |v| Message::SetShowMetric(m, v))
            .into(),
    ])
    .into()
}
