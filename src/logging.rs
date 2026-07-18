use std::{
    fs::{File, OpenOptions, create_dir_all},
    io::Write,
    path::Path,
    sync::{Mutex, OnceLock},
};

use anyhow::Result;

static LOG_FILE: OnceLock<Mutex<File>> = OnceLock::new();

pub fn init(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let _ = LOG_FILE.set(Mutex::new(file));
    Ok(())
}

pub fn log_line(message: impl AsRef<str>) {
    let message = message.as_ref();
    eprintln!("{message}");
    if let Some(file) = LOG_FILE.get()
        && let Ok(mut file) = file.lock()
    {
        let _ = writeln!(file, "{message}");
        let _ = file.flush();
    }
}
