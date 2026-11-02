#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use portpilot::app::{PortPilotApp, initial_window_options};
use portpilot::instance::InstanceGuard;

fn main() -> eframe::Result<()> {
    let start_hidden = std::env::args().any(|argument| argument == "--autostart");
    let instance = match InstanceGuard::acquire(start_hidden) {
        Ok(Some(instance)) => Some(instance),
        Ok(None) => return Ok(()),
        Err(error) => {
            let _ = rfd::MessageDialog::new()
                .set_title("PortPilot")
                .set_description(format!(
                    "Unable to initialize single-instance guard:\n{error}"
                ))
                .set_level(rfd::MessageLevel::Error)
                .show();
            None
        }
    };

    let result = eframe::run_native(
        "PortPilot",
        initial_window_options(start_hidden),
        Box::new(move |creation_context| {
            let app = PortPilotApp::new(creation_context, start_hidden, instance)?;
            Ok(Box::new(app) as Box<dyn eframe::App>)
        }),
    );

    if let Err(error) = &result {
        let _ = rfd::MessageDialog::new()
            .set_title("PortPilot")
            .set_description(format!("Unable to start PortPilot:\n{error}"))
            .set_level(rfd::MessageLevel::Error)
            .show();
    }
    result
}
