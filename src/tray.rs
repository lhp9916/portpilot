use std::sync::mpsc::{Receiver, channel};

use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

use crate::i18n::{Lang, tr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    Show,
    StartAll,
    StopAll,
    Quit,
}

pub struct TrayManager {
    _tray: TrayIcon,
    events: Receiver<MenuEvent>,
}

impl TrayManager {
    pub fn new(ctx: &egui::Context, lang: Lang) -> Result<Self, String> {
        let show = MenuItem::with_id(
            "show",
            if lang == Lang::Zh {
                "显示 PortPilot"
            } else {
                "Show PortPilot"
            },
            true,
            None,
        );
        let start_all = MenuItem::with_id("start_all", tr(lang, "top.start_all"), true, None);
        let stop_all = MenuItem::with_id(
            "stop_all",
            if lang == Lang::Zh {
                "全部停止"
            } else {
                "Stop all"
            },
            true,
            None,
        );
        let separator = PredefinedMenuItem::separator();
        let quit = MenuItem::with_id(
            "quit",
            if lang == Lang::Zh {
                "退出 PortPilot"
            } else {
                "Quit PortPilot"
            },
            true,
            None,
        );
        let menu = Menu::new();
        menu.append_items(&[&show, &start_all, &stop_all, &separator, &quit])
            .map_err(|error| format!("build tray menu: {error}"))?;

        let (sender, receiver) = channel();
        let repaint_ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event| {
            let _ = sender.send(event);
            repaint_ctx.request_repaint();
        }));

        let icon = crate::assets::tray_icon()?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("PortPilot")
            .with_icon(icon)
            .build()
            .map_err(|error| format!("build tray icon: {error}"))?;

        Ok(Self {
            _tray: tray,
            events: receiver,
        })
    }

    pub fn poll(&self) -> Vec<TrayAction> {
        let mut actions = Vec::new();
        while let Ok(event) = self.events.try_recv() {
            let action = match event.id().as_ref() {
                "show" => Some(TrayAction::Show),
                "start_all" => Some(TrayAction::StartAll),
                "stop_all" => Some(TrayAction::StopAll),
                "quit" => Some(TrayAction::Quit),
                _ => None,
            };
            if let Some(action) = action {
                actions.push(action);
            }
        }
        actions
    }
}
