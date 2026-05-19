use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ai_agent=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

pub fn init_logging_with_level(level: tracing::Level) {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("ai_agent={},reqwest=info", level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
