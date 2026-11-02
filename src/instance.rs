use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::SystemTime;

use single_instance::SingleInstance;

pub struct InstanceGuard {
    _instance: SingleInstance,
    show_request: PathBuf,
    last_seen: Option<SystemTime>,
}

impl InstanceGuard {
    pub fn acquire(autostart: bool) -> Result<Option<Self>, String> {
        let instance = SingleInstance::new("com.portpilot.app")
            .map_err(|error| format!("single instance: {error}"))?;
        let app_dir = crate::config::app_dir().map_err(|error| error.to_string())?;
        fs::create_dir_all(&app_dir)
            .map_err(|error| format!("create config directory: {error}"))?;
        let show_request = app_dir.join("show.request");

        if !instance.is_single() {
            if !autostart {
                write_show_request(&show_request)
                    .map_err(|error| format!("signal running instance: {error}"))?;
            }
            return Ok(None);
        }

        let last_seen = fs::metadata(&show_request)
            .and_then(|metadata| metadata.modified())
            .ok();
        Ok(Some(Self {
            _instance: instance,
            show_request,
            last_seen,
        }))
    }

    pub fn show_requested(&mut self) -> bool {
        let Some(modified) = fs::metadata(&self.show_request)
            .and_then(|metadata| metadata.modified())
            .ok()
        else {
            return false;
        };
        if self.last_seen.is_none_or(|last_seen| modified > last_seen) {
            self.last_seen = Some(modified);
            true
        } else {
            false
        }
    }
}

fn write_show_request(path: &PathBuf) -> io::Result<()> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    fs::write(path, now.to_string())
}
