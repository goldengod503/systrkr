fn main() -> cosmic::iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn,galaxy_systrkr=info".into()),
        )
        .init();

    cosmic::applet::run::<galaxy_systrkr::app::App>(())
}
