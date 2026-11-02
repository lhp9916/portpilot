use auto_launch::{AutoLaunch, AutoLaunchBuilder, WindowsEnableMode};

pub struct AutoStartManager {
    launcher: AutoLaunch,
    enabled: bool,
}

impl AutoStartManager {
    pub fn new() -> Result<Self, String> {
        let executable =
            std::env::current_exe().map_err(|error| format!("current executable: {error}"))?;
        let path = executable.to_string_lossy();
        let launcher = AutoLaunchBuilder::new()
            .set_app_name("PortPilot")
            .set_app_path(path.as_ref())
            .set_args(&["--autostart"])
            .set_windows_enable_mode(WindowsEnableMode::CurrentUser)
            .build()
            .map_err(|error| format!("auto launch: {error}"))?;
        let enabled = launcher.is_enabled().unwrap_or(false);
        Ok(Self { launcher, enabled })
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) -> Result<(), String> {
        if enabled {
            self.launcher
                .enable()
                .map_err(|error| format!("enable launch at login: {error}"))?;
        } else {
            self.launcher
                .disable()
                .map_err(|error| format!("disable launch at login: {error}"))?;
        }
        self.enabled = self.launcher.is_enabled().unwrap_or(enabled);
        Ok(())
    }
}
