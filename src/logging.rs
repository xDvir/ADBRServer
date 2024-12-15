use once_cell::sync::OnceCell;
use tempfile::Builder;
use tracing_subscriber::{fmt, layer::SubscriberExt, Registry};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

static TEMP_DIR: OnceCell<tempfile::TempDir> = OnceCell::new();
const ADBR_SERVER_PREFIX: &str = "adbr-server-";
const RANDOM_BYTES_LENGTH: usize = 5;
const LOG_FILE_NAME: &str = "adbr";

pub fn init() -> std::io::Result<()> {
    TEMP_DIR.get_or_try_init(|| {
        let tmp = std::env::temp_dir();
        Builder::new()
            .prefix(ADBR_SERVER_PREFIX)
            .rand_bytes(RANDOM_BYTES_LENGTH)
            .tempdir_in(tmp)
    })?;

    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        get_temp_path(),
        LOG_FILE_NAME,
    );

    let subscriber = Registry::default()
        .with(fmt::layer().with_writer(file_appender));

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    Ok(())
}

pub fn get_temp_path() -> &'static std::path::Path {
    TEMP_DIR.get()
        .expect("Temp directory not initialized")
        .path()
}