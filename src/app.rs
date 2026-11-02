use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui;
use uuid::Uuid;

use crate::autostart::AutoStartManager;
use crate::config::{self, AppConfig, LoadOutcome};
use crate::engine::{EngineCommand, EngineEvent, EngineHandle, RuleStatsSnapshot, RuleStatus};
use crate::i18n::{Lang, resolve_language, tr};
use crate::instance::InstanceGuard;
use crate::log_store::{LogLevel, LogStore};
use crate::model::{ForwardRule, Language, Protocol, RuleIssue, Theme, validate_rule};
use crate::net_util::{is_local_bind_address, local_bind_addresses};
use crate::theme::Palette;
use crate::tray::{TrayAction, TrayManager};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Forwarding,
    Logs,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleFilter {
    All,
    Running,
    Stopped,
}

#[derive(Default)]
pub struct RuleRuntime {
    pub status: RuleStatus,
    pub stats: Option<RuleStatsSnapshot>,
    pub up_rate: f64,
    pub down_rate: f64,
    pub history: VecDeque<(f64, f64)>,
    last_sample: Option<(Instant, u64, u64)>,
}

impl RuleRuntime {
    fn stopped() -> Self {
        Self {
            status: RuleStatus::Stopped,
            ..Self::default()
        }
    }

    fn apply_stats(&mut self, stats: RuleStatsSnapshot) {
        let now = Instant::now();
        if let Some((at, bytes_up, bytes_down)) = self.last_sample {
            let seconds = now.duration_since(at).as_secs_f64();
            if seconds > 0.1 && stats.bytes_up >= bytes_up && stats.bytes_down >= bytes_down {
                self.up_rate = (stats.bytes_up - bytes_up) as f64 / seconds;
                self.down_rate = (stats.bytes_down - bytes_down) as f64 / seconds;
            }
        }
        self.last_sample = Some((now, stats.bytes_up, stats.bytes_down));
        self.stats = Some(stats);
        if self.history.len() >= 60 {
            self.history.pop_front();
        }
        self.history.push_back((self.up_rate, self.down_rate));
    }

    fn reset_activity(&mut self) {
        self.up_rate = 0.0;
        self.down_rate = 0.0;
        self.last_sample = None;
        if let Some(stats) = &mut self.stats {
            stats.active_connections = 0;
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuleEditor {
    pub original_id: Option<Uuid>,
    pub name: String,
    pub protocol: Protocol,
    pub bind_ip: String,
    pub bind_port: String,
    pub target_host: String,
    pub target_port: String,
    pub enabled: bool,
    pub errors: Vec<RuleIssue>,
}

impl RuleEditor {
    pub fn new(rule: Option<&ForwardRule>, bind_ip: IpAddr, port: u16) -> Self {
        if let Some(rule) = rule {
            Self {
                original_id: Some(rule.id),
                name: rule.name.clone(),
                protocol: rule.protocol,
                bind_ip: rule.bind_ip.to_string(),
                bind_port: rule.bind_port.to_string(),
                target_host: rule.target_host.clone(),
                target_port: rule.target_port.to_string(),
                enabled: rule.enabled,
                errors: Vec::new(),
            }
        } else {
            Self {
                original_id: None,
                name: String::new(),
                protocol: Protocol::Tcp,
                bind_ip: bind_ip.to_string(),
                bind_port: port.to_string(),
                target_host: "127.0.0.1".to_owned(),
                target_port: "3000".to_owned(),
                enabled: true,
                errors: Vec::new(),
            }
        }
    }

    pub fn has_error(&self, issue: RuleIssue) -> bool {
        self.errors.contains(&issue)
    }
}

pub struct Toast {
    pub message: String,
    pub created: Instant,
}

#[derive(Default)]
pub struct LogViewState {
    pub search: String,
    pub level: Option<LogLevel>,
}

pub struct PortPilotApp {
    pub config: AppConfig,
    pub config_path: PathBuf,
    pub config_warnings: Vec<String>,
    pub logs: Arc<LogStore>,
    pub engine: EngineHandle,
    pub instance: Option<InstanceGuard>,
    pub tray: Option<TrayManager>,
    pub tray_error: Option<String>,
    pub autostart: Option<AutoStartManager>,
    pub autostart_error: Option<String>,
    pub bind_addresses: Vec<IpAddr>,
    pub lang: Lang,
    pub palette: Palette,
    pub view: View,
    pub selected: Option<Uuid>,
    pub search: String,
    pub filter: RuleFilter,
    pub runtimes: HashMap<Uuid, RuleRuntime>,
    pub editor: Option<RuleEditor>,
    pub pending_delete: Option<Uuid>,
    pub pending_import: Option<Vec<ForwardRule>>,
    pub logs_view: LogViewState,
    pub toast: Option<Toast>,
    pub allow_exit: bool,
    pub shutdown_done: bool,
    pub window_hidden: bool,
    pub force_show: bool,
    pub last_dark: bool,
    pub icon_texture: egui::TextureHandle,
}

impl PortPilotApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        start_hidden: bool,
        instance: Option<InstanceGuard>,
    ) -> Result<Self, std::io::Error> {
        crate::assets::install_fonts(&cc.egui_ctx);

        let LoadOutcome {
            config,
            warnings,
            path,
        } = config::load().map_err(std::io::Error::other)?;
        let lang = resolve_language(config.language);
        let logs = Arc::new(LogStore::new(5_000));
        for warning in &warnings {
            logs.warn(None, "", warning.clone());
        }

        let engine = EngineHandle::new(Arc::clone(&logs)).map_err(std::io::Error::other)?;
        let bind_addresses =
            local_bind_addresses().unwrap_or_else(|_| vec!["127.0.0.1".parse().unwrap()]);

        let (autostart, autostart_error) = match AutoStartManager::new() {
            Ok(manager) => (Some(manager), None),
            Err(error) => {
                logs.warn(
                    None,
                    "",
                    format!("{}: {error}", tr(lang, "settings.autostart")),
                );
                (None, Some(error))
            }
        };

        let (tray, tray_error) = match TrayManager::new(&cc.egui_ctx, lang) {
            Ok(tray) => (Some(tray), None),
            Err(error) => {
                logs.warn(
                    None,
                    "",
                    format!("{}: {error}", tr(lang, "notice.tray_unavailable")),
                );
                (None, Some(error))
            }
        };

        let icon = crate::assets::egui_icon();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [icon.width as usize, icon.height as usize],
            &icon.rgba,
        );
        let icon_texture =
            cc.egui_ctx
                .load_texture("portpilot-icon", color_image, egui::TextureOptions::LINEAR);

        let palette = Palette::for_theme(config.theme, cc.egui_ctx.system_theme());
        crate::theme::apply(&cc.egui_ctx, palette);

        let mut runtimes = HashMap::new();
        for rule in &config.rules {
            runtimes.insert(rule.id, RuleRuntime::stopped());
        }

        let selected = config.rules.first().map(|rule| rule.id);
        let force_show = start_hidden && tray_error.is_some();
        let initial_view = debug_initial_view();
        let mut app = Self {
            config,
            config_path: path,
            config_warnings: warnings,
            logs,
            engine,
            instance,
            tray,
            tray_error,
            autostart,
            autostart_error,
            bind_addresses,
            lang,
            palette,
            view: initial_view,
            selected,
            search: String::new(),
            filter: RuleFilter::All,
            runtimes,
            editor: None,
            pending_delete: None,
            pending_import: None,
            logs_view: LogViewState::default(),
            toast: None,
            allow_exit: false,
            shutdown_done: false,
            window_hidden: start_hidden,
            force_show,
            last_dark: palette.dark,
            icon_texture,
        };

        if !app.config_warnings.is_empty() {
            app.show_toast(tr(lang, "notice.config_recovered"));
        }
        if let Some(error) = &app.autostart_error {
            app.logs.warn(None, "", error.clone());
        }
        if let Some(error) = &app.tray_error {
            app.logs.warn(None, "", error.clone());
        }

        let enabled_ids: Vec<Uuid> = app
            .config
            .rules
            .iter()
            .filter(|rule| rule.enabled)
            .map(|rule| rule.id)
            .collect();
        for id in enabled_ids {
            app.start_rule(id, false);
        }
        #[cfg(debug_assertions)]
        if std::env::var_os("PORTPILOT_OPEN_EDITOR").is_some() {
            app.open_new_rule();
        }

        Ok(app)
    }

    pub fn status(&self, id: Uuid) -> RuleStatus {
        self.runtimes
            .get(&id)
            .map(|runtime| runtime.status.clone())
            .unwrap_or(RuleStatus::Stopped)
    }

    pub fn is_active(&self, id: Uuid) -> bool {
        matches!(self.status(id), RuleStatus::Running | RuleStatus::Starting)
    }

    pub fn rule(&self, id: Uuid) -> Option<&ForwardRule> {
        self.config.rules.iter().find(|rule| rule.id == id)
    }

    pub fn selected_rule(&self) -> Option<&ForwardRule> {
        self.selected.and_then(|id| self.rule(id))
    }

    pub fn running_count(&self) -> usize {
        self.config
            .rules
            .iter()
            .filter(|rule| self.is_active(rule.id))
            .count()
    }

    pub fn open_new_rule(&mut self) {
        let bind_ip = self
            .bind_addresses
            .first()
            .copied()
            .unwrap_or_else(|| "127.0.0.1".parse().unwrap());
        let port = self.next_available_port(8080);
        self.editor = Some(RuleEditor::new(None, bind_ip, port));
    }

    pub fn open_edit_rule(&mut self, id: Uuid) {
        let Some(rule) = self.rule(id).cloned() else {
            return;
        };
        self.editor = Some(RuleEditor::new(Some(&rule), rule.bind_ip, rule.bind_port));
    }

    fn next_available_port(&self, start: u16) -> u16 {
        let mut port = start;
        while self.config.rules.iter().any(|rule| rule.bind_port == port) {
            port = port.saturating_add(1);
            if port == 0 {
                break;
            }
        }
        port.max(1)
    }

    pub fn try_save_editor(&mut self) -> bool {
        let Some(editor) = self.editor.as_mut() else {
            return false;
        };

        let mut issues = Vec::new();
        let bind_ip = match editor.bind_ip.trim().parse::<IpAddr>() {
            Ok(ip) => Some(ip),
            Err(_) => {
                issues.push(RuleIssue::LocalBindRequired);
                None
            }
        };
        let bind_port = match editor.bind_port.trim().parse::<u16>() {
            Ok(port) if port > 0 => port,
            _ => {
                issues.push(RuleIssue::InvalidListenPort);
                0
            }
        };
        let target_port = match editor.target_port.trim().parse::<u16>() {
            Ok(port) if port > 0 => port,
            _ => {
                issues.push(RuleIssue::InvalidTargetPort);
                0
            }
        };

        let Some(bind_ip) = bind_ip else {
            editor.errors = issues;
            return false;
        };

        let candidate = ForwardRule {
            id: editor.original_id.unwrap_or_else(Uuid::new_v4),
            name: editor.name.trim().to_owned(),
            protocol: editor.protocol,
            bind_ip,
            bind_port,
            target_host: editor.target_host.trim().to_owned(),
            target_port,
            enabled: editor.enabled,
        };

        issues.extend(validate_rule(&candidate, &self.config.rules));
        if !is_local_bind_address(bind_ip) {
            issues.push(RuleIssue::LocalBindRequired);
        }
        issues.sort_by_key(|issue| format!("{issue:?}"));
        issues.dedup();

        if !issues.is_empty() {
            editor.errors = issues;
            return false;
        }

        let original_id = editor.original_id;
        let was_running = original_id.map(|id| self.is_active(id)).unwrap_or(false);
        if was_running {
            self.engine.send(EngineCommand::Stop(candidate.id));
        }

        if let Some(id) = original_id {
            if let Some(existing) = self.config.rules.iter_mut().find(|rule| rule.id == id) {
                *existing = candidate.clone();
            }
        } else {
            self.config.rules.push(candidate.clone());
            self.runtimes.insert(candidate.id, RuleRuntime::stopped());
        }

        self.selected = Some(candidate.id);
        if candidate.enabled {
            self.start_rule(candidate.id, false);
        }
        self.save_config();
        self.editor = None;
        self.show_toast(tr(self.lang, "notice.saved"));
        true
    }

    pub fn start_rule(&mut self, id: Uuid, persist: bool) {
        let Some(rule) = self.rule(id).cloned() else {
            return;
        };
        if self.is_active(id) {
            return;
        }
        if let Some(existing) = self.config.rules.iter_mut().find(|rule| rule.id == id) {
            existing.enabled = true;
        }
        let runtime = self.runtimes.entry(id).or_default();
        runtime.status = RuleStatus::Starting;
        runtime.reset_activity();
        self.engine.send(EngineCommand::Start(rule));
        if persist {
            self.save_config();
        }
    }

    pub fn stop_rule(&mut self, id: Uuid, persist: bool) {
        if let Some(existing) = self.config.rules.iter_mut().find(|rule| rule.id == id) {
            existing.enabled = false;
        }
        let runtime = self.runtimes.entry(id).or_default();
        runtime.status = RuleStatus::Stopping;
        runtime.reset_activity();
        self.engine.send(EngineCommand::Stop(id));
        if persist {
            self.save_config();
        }
    }

    pub fn toggle_rule(&mut self, id: Uuid) {
        if self.is_active(id) {
            self.stop_rule(id, true);
        } else {
            self.start_rule(id, true);
        }
    }

    pub fn restart_rule(&mut self, id: Uuid) {
        self.stop_rule(id, false);
        self.start_rule(id, true);
    }

    pub fn start_all(&mut self) {
        let ids: Vec<Uuid> = self.config.rules.iter().map(|rule| rule.id).collect();
        for id in ids {
            if !self.is_active(id) {
                self.start_rule(id, false);
            }
        }
        self.save_config();
    }

    pub fn stop_all(&mut self) {
        for rule in &mut self.config.rules {
            rule.enabled = false;
            let runtime = self.runtimes.entry(rule.id).or_default();
            runtime.status = RuleStatus::Stopping;
            runtime.reset_activity();
        }
        self.engine.send(EngineCommand::StopAll);
        self.save_config();
    }

    pub fn duplicate_rule(&mut self, id: Uuid) {
        let Some(rule) = self.rule(id).cloned() else {
            return;
        };
        let mut copy = rule;
        copy.id = Uuid::new_v4();
        copy.name = format!(
            "{} {}",
            copy.name,
            if self.lang == Lang::Zh {
                "副本"
            } else {
                "copy"
            }
        );
        copy.enabled = false;
        let new_id = copy.id;
        self.config.rules.push(copy);
        self.runtimes.insert(new_id, RuleRuntime::stopped());
        self.selected = Some(new_id);
        self.save_config();
    }

    pub fn delete_rule(&mut self, id: Uuid) {
        if self.is_active(id) {
            self.engine.send(EngineCommand::Stop(id));
        }
        self.config.rules.retain(|rule| rule.id != id);
        self.runtimes.remove(&id);
        if self.selected == Some(id) {
            self.selected = self.config.rules.first().map(|rule| rule.id);
        }
        self.save_config();
    }

    pub fn copy_listen_address(&mut self, ctx: &egui::Context, id: Uuid) {
        if let Some(rule) = self.rule(id) {
            ctx.copy_text(rule.listener_endpoint());
            self.show_toast(tr(self.lang, "notice.copied"));
        }
    }

    pub fn import_rules(&mut self, rules: Vec<ForwardRule>) {
        self.engine.send(EngineCommand::StopAll);
        self.config.rules = rules
            .into_iter()
            .map(|mut rule| {
                rule.id = Uuid::new_v4();
                rule.enabled = false;
                rule
            })
            .collect();
        self.runtimes.clear();
        for rule in &self.config.rules {
            self.runtimes.insert(rule.id, RuleRuntime::stopped());
        }
        self.selected = self.config.rules.first().map(|rule| rule.id);
        self.save_config();
        self.show_toast(tr(self.lang, "notice.imported"));
    }

    pub fn save_config(&mut self) {
        match config::save(&self.config) {
            Ok(path) => self.config_path = path,
            Err(error) => self.logs.error(
                None,
                "",
                format!("{}: {error}", tr(self.lang, "log.config_save_failed")),
            ),
        }
    }

    pub fn set_language(&mut self, language: Language) {
        self.config.language = language;
        self.lang = resolve_language(language);
        self.save_config();
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.config.theme = theme;
        self.save_config();
    }

    pub fn apply_theme_if_needed(&mut self, ctx: &egui::Context) {
        let palette = Palette::for_theme(self.config.theme, ctx.system_theme());
        if palette.dark != self.last_dark {
            crate::theme::apply(ctx, palette);
            self.last_dark = palette.dark;
        }
        self.palette = palette;
    }

    pub fn show_toast(&mut self, message: impl Into<String>) {
        self.toast = Some(Toast {
            message: message.into(),
            created: Instant::now(),
        });
    }

    pub fn show_window(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.window_hidden = false;
    }

    pub fn hide_window(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        self.window_hidden = true;
    }

    pub fn quit(&mut self, ctx: &egui::Context) {
        self.allow_exit = true;
        self.finish();
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    pub fn finish(&mut self) {
        if self.shutdown_done {
            return;
        }
        self.shutdown_done = true;
        self.engine.send(EngineCommand::StopAll);
        self.save_config();
    }

    fn poll_engine(&mut self) {
        while let Some(event) = self.engine.try_recv() {
            match event {
                EngineEvent::Status(id, status) => {
                    let runtime = self.runtimes.entry(id).or_default();
                    if matches!(status, RuleStatus::Stopped | RuleStatus::Failed(_)) {
                        runtime.reset_activity();
                    }
                    runtime.status = status;
                }
                EngineEvent::Stats(id, stats) => {
                    self.runtimes.entry(id).or_default().apply_stats(stats);
                }
            }
        }
    }

    fn poll_tray(&mut self, ctx: &egui::Context) {
        let Some(tray) = &self.tray else {
            return;
        };
        for action in tray.poll() {
            match action {
                TrayAction::Show => self.show_window(ctx),
                TrayAction::StartAll => self.start_all(),
                TrayAction::StopAll => self.stop_all(),
                TrayAction::Quit => self.quit(ctx),
            }
        }
    }

    fn poll_instance(&mut self, ctx: &egui::Context) {
        if self
            .instance
            .as_mut()
            .is_some_and(InstanceGuard::show_requested)
        {
            self.show_window(ctx);
        }
    }

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.viewport().close_requested()) {
            return;
        }
        if self.allow_exit {
            self.finish();
            return;
        }
        if self.config.close_to_tray && self.tray.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.hide_window(ctx);
        } else {
            self.finish();
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if self.editor.is_some() || self.pending_delete.is_some() || self.pending_import.is_some() {
            return;
        }
        ctx.input(|input| {
            if input.modifiers.command && input.key_pressed(egui::Key::N) {
                self.open_new_rule();
            } else if input.modifiers.command && input.key_pressed(egui::Key::Num1) {
                self.view = View::Forwarding;
            } else if input.modifiers.command && input.key_pressed(egui::Key::Num2) {
                self.view = View::Logs;
            } else if input.modifiers.command && input.key_pressed(egui::Key::Num3) {
                self.view = View::Settings;
            } else if input.modifiers.command
                && input.modifiers.shift
                && input.key_pressed(egui::Key::T)
            {
                self.set_theme(if self.palette.dark {
                    Theme::Light
                } else {
                    Theme::Dark
                });
            }
        });
    }

    fn update_toast(&mut self, ctx: &egui::Context) {
        if self
            .toast
            .as_ref()
            .is_some_and(|toast| toast.created.elapsed() > Duration::from_secs(3))
        {
            self.toast = None;
        }
        if self.toast.is_some() {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }
}

#[cfg(debug_assertions)]
fn debug_initial_view() -> View {
    match std::env::var("PORTPILOT_START_VIEW").as_deref() {
        Ok("logs") => View::Logs,
        Ok("settings") => View::Settings,
        _ => View::Forwarding,
    }
}

#[cfg(not(debug_assertions))]
fn debug_initial_view() -> View {
    View::Forwarding
}

impl eframe::App for PortPilotApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_engine();
        self.poll_tray(ctx);
        self.poll_instance(ctx);
        self.apply_theme_if_needed(ctx);
        self.handle_close_request(ctx);
        self.handle_shortcuts(ctx);

        if self.force_show {
            self.force_show = false;
            self.show_window(ctx);
        }

        self.update_toast(ctx);
        ctx.request_repaint_after(Duration::from_millis(500));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        crate::ui::draw(self, ui);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.finish();
    }
}

pub fn initial_window_options(autostart: bool) -> eframe::NativeOptions {
    let icon = crate::assets::egui_icon();
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PortPilot · 端口转发")
            .with_inner_size([1180.0, 720.0])
            .with_min_inner_size([1000.0, 660.0])
            .with_position([60.0, 24.0])
            .with_icon(icon)
            .with_visible(!autostart),
        ..Default::default()
    }
}
