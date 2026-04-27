use cosmic::iced::Length;
use cosmic::widget::{button, column as col, divider, icon, row, space, text};
use cosmic::{Application, Element};

use crate::app::{App, Message};
use crate::sampler::{CpuSample, GpuSample};

pub fn view(app: &App) -> Element<'_, Message> {
    let cpu = &app.latest.cpu;
    let gpu = &app.latest.gpu;
    let gpu_name = app.gpu_name().to_string();

    let header = row::with_children(vec![
        text("System").size(13).into(),
        space::horizontal().into(),
        button::icon(icon::from_name("emblem-system-symbolic"))
            .on_press(Message::ToggleSettings)
            .into(),
    ]);

    let mut sections: Vec<Element<'_, Message>> = vec![
        header.into(),
        cpu_section(cpu).into(),
        crate::widgets::proc_list::cpu_list(&app.latest.top_cpu_procs),
        divider::horizontal::default().into(),
        gpu_section(&gpu_name, gpu).into(),
        crate::widgets::proc_list::gpu_list(
            &app.latest.top_gpu_procs,
            app.gpu_proc_backend_available(),
        ),
    ];

    if app.settings_open {
        sections.push(divider::horizontal::default().into());
        sections.push(crate::settings::view(app));
    }

    sections.push(divider::horizontal::default().into());
    sections.push(footer(app.system_monitor_bin).into());

    let body = col::with_children(sections).spacing(8).padding(12);

    app.core().applet.popup_container(body).into()
}

fn cpu_section(s: &CpuSample) -> Element<'_, Message> {
    let model = s
        .model
        .clone()
        .unwrap_or_else(|| "Unknown CPU".to_string());

    col::with_children(vec![
        text(model).size(13).into(),
        kv_row(
            "Usage",
            s.utilization_pct
                .map(|v| format!("{v:.0}%"))
                .unwrap_or_else(|| "—".into()),
        ),
        kv_row(
            "Temperature",
            s.temperature_c
                .map(|v| format!("{v:.0}°C"))
                .unwrap_or_else(|| "—".into()),
        ),
        kv_row("RAM", fmt_used_total(s.ram_used_bytes, s.ram_total_bytes)),
        kv_row("Swap", fmt_used_total(s.swap_used_bytes, s.swap_total_bytes)),
        kv_row(
            "Load avg",
            match (s.load_avg_1m, s.load_avg_5m, s.load_avg_15m) {
                (Some(a), Some(b), Some(c)) => format!("{a:.2} / {b:.2} / {c:.2}"),
                _ => "—".into(),
            },
        ),
    ])
    .spacing(2)
    .into()
}

fn gpu_section<'a>(name: &str, s: &GpuSample) -> Element<'a, Message> {
    col::with_children(vec![
        text(name.to_string()).size(13).into(),
        kv_row(
            "Usage",
            s.utilization_pct
                .map(|v| format!("{v:.0}%"))
                .unwrap_or_else(|| "—".into()),
        ),
        kv_row(
            "Temperature",
            s.temperature_c
                .map(|v| format!("{v:.0}°C"))
                .unwrap_or_else(|| "—".into()),
        ),
        kv_row(
            "VRAM",
            fmt_used_total(s.memory_used_bytes, s.memory_total_bytes),
        ),
    ])
    .spacing(2)
    .into()
}

fn footer<'a>(bin: Option<&'static str>) -> Element<'a, Message> {
    let label = match bin {
        Some(_) => "Open System Monitor",
        None => "System monitor not found",
    };
    let mut btn = button::standard(label).width(Length::Fill);
    if bin.is_some() {
        btn = btn.on_press(Message::OpenSystemMonitor);
    }
    btn.into()
}

fn kv_row<'a>(key: &'static str, value: String) -> Element<'a, Message> {
    row::with_children(vec![
        text(key).size(11).width(Length::Fill).into(),
        text(value).size(11).into(),
    ])
    .into()
}

fn fmt_used_total(used: Option<u64>, total: Option<u64>) -> String {
    match (used, total) {
        (Some(u), Some(t)) => format!("{} / {}", fmt_bytes(u), fmt_bytes(t)),
        _ => "—".into(),
    }
}

fn fmt_bytes(b: u64) -> String {
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    let b = b as f64;
    if b >= GB {
        format!("{:.1} GiB", b / GB)
    } else {
        format!("{:.0} MiB", b / MB)
    }
}
