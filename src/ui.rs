use crate::app::{PortPilotApp, RuleEditor, RuleFilter, View};
use crate::config;
use crate::engine::RuleStatus;
use crate::i18n::{Lang, tr};
use crate::log_store::{LogEntry, LogLevel};
use crate::model::{ForwardRule, Language, Protocol, RuleIssue, Theme};
use crate::net_util::{format_bytes, format_duration, format_rate};
use crate::theme::Palette;
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Id, Layout, Margin, Rect, RichText, Sense,
    Shape, Stroke, StrokeKind, TextEdit, Ui, pos2, vec2,
};

const RAIL_WIDTH: f32 = 76.0;
const TOPBAR_HEIGHT: f32 = 82.0;
const RULES_WIDTH: f32 = 370.0;

enum CardAction {
    Select,
    Toggle,
    Copy,
    Edit,
    Restart,
    Duplicate,
    Delete,
}

enum NavKind {
    Routes,
    Logs,
    Settings,
}

pub fn draw(app: &mut PortPilotApp, ui: &mut Ui) {
    let p = app.palette;
    let size = ui.available_size();
    ui.painter().rect_filled(ui.max_rect(), 0.0, p.bg);
    ui.set_min_size(size);
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.allocate_ui_with_layout(
            vec2(RAIL_WIDTH, size.y),
            Layout::top_down(Align::Center),
            |ui| rail(app, ui, size.y),
        );
        ui.allocate_ui_with_layout(
            vec2((size.x - RAIL_WIDTH).max(0.0), size.y),
            Layout::top_down(Align::Min),
            |ui| workspace(app, ui, size.y),
        );
    });

    if app.editor.is_some() {
        show_rule_editor(app, ui.ctx());
    }
    if app.pending_delete.is_some() {
        show_delete_dialog(app, ui.ctx());
    }
    if app.pending_import.is_some() {
        show_import_dialog(app, ui.ctx());
    }
    show_toast(app, ui.ctx());
}

fn rail(app: &mut PortPilotApp, ui: &mut Ui, height: f32) {
    let p = app.palette;
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, p.rail);
    ui.painter().line_segment(
        [
            pos2(rect.right() - 1.0, rect.top()),
            pos2(rect.right() - 1.0, rect.bottom()),
        ],
        Stroke::new(1.0, p.border),
    );
    ui.set_min_size(vec2(RAIL_WIDTH, height));
    ui.add_space(18.0);

    let (icon_rect, _) = ui.allocate_exact_size(vec2(42.0, 42.0), Sense::hover());
    ui.painter().image(
        app.icon_texture.id(),
        icon_rect,
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::WHITE,
    );
    ui.add_space(16.0);

    if nav_button(ui, NavKind::Routes, app.view == View::Forwarding, p)
        .on_hover_text(tr(app.lang, "nav.rules"))
        .clicked()
    {
        app.view = View::Forwarding;
    }
    if nav_button(ui, NavKind::Logs, app.view == View::Logs, p)
        .on_hover_text(tr(app.lang, "nav.logs"))
        .clicked()
    {
        app.view = View::Logs;
    }
    if nav_button(ui, NavKind::Settings, app.view == View::Settings, p)
        .on_hover_text(tr(app.lang, "nav.settings"))
        .clicked()
    {
        app.view = View::Settings;
    }

    let remaining = ui.available_height();
    if remaining > 82.0 {
        ui.add_space(remaining - 82.0);
    }
    let (dot_rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
    ui.painter()
        .circle_filled(dot_rect.center(), 8.0, p.green_soft);
    ui.painter().circle_filled(dot_rect.center(), 4.0, p.green);
    ui.add_space(8.0);
    ui.label(
        RichText::new("v0.1")
            .font(FontId::monospace(8.5))
            .color(p.faint),
    );
}

fn nav_button(ui: &mut Ui, kind: NavKind, active: bool, p: Palette) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(52.0, 46.0), Sense::click());
    let fill = if active {
        p.accent_soft
    } else if response.hovered() {
        p.hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter()
        .rect_filled(rect.shrink2(vec2(4.0, 1.0)), CornerRadius::same(10), fill);
    if active {
        ui.painter().rect_filled(
            Rect::from_min_size(pos2(rect.left(), rect.top() + 12.0), vec2(2.0, 22.0)),
            CornerRadius::same(1),
            p.accent,
        );
    }
    paint_nav_icon(
        ui.painter(),
        rect.shrink(14.0),
        kind,
        if active { p.accent } else { p.muted },
    );
    response
}

fn paint_nav_icon(painter: &egui::Painter, rect: Rect, kind: NavKind, color: Color32) {
    let stroke = Stroke::new(1.6, color);
    match kind {
        NavKind::Routes => {
            painter.circle_stroke(pos2(rect.left() + 3.0, rect.top() + 4.0), 2.2, stroke);
            painter.circle_stroke(pos2(rect.right() - 3.0, rect.bottom() - 4.0), 2.2, stroke);
            painter.add(Shape::line(
                vec![
                    pos2(rect.left() + 3.0, rect.top() + 6.0),
                    pos2(rect.center().x + 2.0, rect.top() + 6.0),
                    pos2(rect.center().x + 2.0, rect.bottom() - 6.0),
                    pos2(rect.right() - 5.0, rect.bottom() - 6.0),
                ],
                stroke,
            ));
        }
        NavKind::Logs => {
            for (index, y) in [0.25, 0.5, 0.75].iter().enumerate() {
                let end = if index == 2 { 0.72 } else { 1.0 };
                painter.line_segment(
                    [
                        pos2(rect.left(), rect.top() + rect.height() * *y),
                        pos2(
                            rect.left() + rect.width() * end,
                            rect.top() + rect.height() * *y,
                        ),
                    ],
                    stroke,
                );
            }
            painter.circle_filled(
                pos2(rect.right() - 1.0, rect.bottom() - rect.height() * 0.25),
                1.4,
                color,
            );
        }
        NavKind::Settings => {
            painter.circle_stroke(rect.center(), 4.0, stroke);
            for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
                painter.line_segment(
                    [
                        rect.center() + vec2(dx * 7.0, dy * 7.0),
                        rect.center() + vec2(dx * 10.0, dy * 10.0),
                    ],
                    stroke,
                );
            }
        }
    }
}

fn workspace(app: &mut PortPilotApp, ui: &mut Ui, height: f32) {
    let p = app.palette;
    ui.painter().rect_filled(ui.max_rect(), 0.0, p.shell);
    ui.set_min_size(vec2(ui.available_width(), height));
    topbar(app, ui);

    let body_height = ui.available_height();
    egui::Frame::NONE
        .fill(p.shell)
        .inner_margin(Margin {
            left: 16,
            right: 18,
            top: 14,
            bottom: 16,
        })
        .show(ui, |ui| {
            ui.set_min_size(vec2(ui.available_width(), body_height - 30.0));
            match app.view {
                View::Forwarding => forwarding_body(app, ui),
                View::Logs => logs_view(app, ui),
                View::Settings => settings_view(app, ui),
            }
        });
}

fn topbar(app: &mut PortPilotApp, ui: &mut Ui) {
    let p = app.palette;
    let width = ui.available_width();
    let response = ui.allocate_ui_with_layout(
        vec2(width, TOPBAR_HEIGHT),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_min_size(vec2(width, TOPBAR_HEIGHT));
            ui.painter().rect_filled(ui.max_rect(), 0.0, p.shell);
            ui.add_space(24.0);
            ui.vertical(|ui| {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(tr(app.lang, "top.eyebrow"))
                        .font(FontId::monospace(9.0))
                        .color(p.accent),
                );
                let title = match app.view {
                    View::Forwarding => tr(app.lang, "top.title"),
                    View::Logs => tr(app.lang, "logs.title"),
                    View::Settings => tr(app.lang, "settings.title"),
                };
                ui.label(RichText::new(title).size(23.0).strong().color(p.text));
                let subtitle = match app.view {
                    View::Forwarding => tr(app.lang, "top.subtitle"),
                    View::Logs => tr(app.lang, "logs.subtitle"),
                    View::Settings => tr(app.lang, "settings.subtitle"),
                };
                ui.label(RichText::new(subtitle).size(11.0).color(p.muted));
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(23.0);
                if theme_button(ui, p).clicked() {
                    app.set_theme(if p.dark { Theme::Light } else { Theme::Dark });
                }
                ui.add_space(8.0);
                language_switch(app, ui);
                if app.view == View::Forwarding {
                    ui.add_space(10.0);
                    if accent_button(ui, &format!("+  {}", tr(app.lang, "top.new_rule")), p)
                        .clicked()
                    {
                        app.open_new_rule();
                    }
                    ui.add_space(8.0);
                    if secondary_button(ui, &format!("▶  {}", tr(app.lang, "top.start_all")), p)
                        .clicked()
                    {
                        app.start_all();
                    }
                }
            });
        },
    );
    ui.painter().line_segment(
        [
            pos2(
                response.response.rect.left(),
                response.response.rect.bottom(),
            ),
            pos2(
                response.response.rect.right(),
                response.response.rect.bottom(),
            ),
        ],
        Stroke::new(1.0, p.border),
    );
}

fn language_switch(app: &mut PortPilotApp, ui: &mut Ui) {
    let p = app.palette;
    egui::Frame::NONE
        .fill(p.panel_soft)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(3, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let zh = app.lang == Lang::Zh;
                if mini_language_button(ui, "中", zh, p).clicked() {
                    app.set_language(Language::ZhCn);
                }
                if mini_language_button(ui, "EN", !zh, p).clicked() {
                    app.set_language(Language::EnUs);
                }
            });
        });
}

fn mini_language_button(ui: &mut Ui, text: &str, selected: bool, p: Palette) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(text)
                .font(FontId::monospace(9.0))
                .color(if selected { p.text } else { p.faint }),
        )
        .fill(if selected {
            p.panel_raised
        } else {
            Color32::TRANSPARENT
        })
        .stroke(Stroke::NONE)
        .corner_radius(CornerRadius::same(5))
        .min_size(vec2(30.0, 26.0)),
    )
}

fn forwarding_body(app: &mut PortPilotApp, ui: &mut Ui) {
    let height = ui.available_height();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        ui.allocate_ui_with_layout(
            vec2(RULES_WIDTH, height),
            Layout::top_down(Align::Min),
            |ui| rules_panel(app, ui, height),
        );
        ui.allocate_ui_with_layout(
            vec2(ui.available_width(), height),
            Layout::top_down(Align::Min),
            |ui| detail_panel(app, ui, height),
        );
    });
}

fn rules_panel(app: &mut PortPilotApp, ui: &mut Ui, height: f32) {
    let p = app.palette;
    egui::Frame::NONE
        .fill(p.panel)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(Margin::symmetric(12, 14))
        .show(ui, |ui| {
            ui.set_min_size(vec2(RULES_WIDTH - 26.0, height - 30.0));
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(tr(app.lang, "rules.title"))
                        .size(13.0)
                        .strong()
                        .color(p.text),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} {}",
                            app.config.rules.len(),
                            if app.lang == Lang::Zh {
                                "条规则"
                            } else {
                                "RULES"
                            }
                        ))
                        .font(FontId::monospace(9.5))
                        .color(p.faint),
                    );
                });
            });
            ui.add_space(10.0);

            egui::Frame::NONE
                .fill(p.panel_soft)
                .stroke(Stroke::new(1.0, p.border))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(9, 5))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        search_icon(ui, p.faint);
                        ui.add(
                            TextEdit::singleline(&mut app.search)
                                .hint_text(tr(app.lang, "rules.search"))
                                .desired_width(f32::INFINITY)
                                .frame(egui::Frame::NONE),
                        );
                    });
                });
            ui.add_space(9.0);

            let total = app.config.rules.len();
            let running = app.running_count();
            let stopped = total.saturating_sub(running);
            ui.horizontal(|ui| {
                if filter_chip(
                    ui,
                    app.filter == RuleFilter::All,
                    &format!("{} {}", tr(app.lang, "rules.all"), total),
                    p,
                )
                .clicked()
                {
                    app.filter = RuleFilter::All;
                }
                if filter_chip(
                    ui,
                    app.filter == RuleFilter::Running,
                    &format!("{} {}", tr(app.lang, "rules.running"), running),
                    p,
                )
                .clicked()
                {
                    app.filter = RuleFilter::Running;
                }
                if filter_chip(
                    ui,
                    app.filter == RuleFilter::Stopped,
                    &format!("{} {}", tr(app.lang, "rules.stopped"), stopped),
                    p,
                )
                .clicked()
                {
                    app.filter = RuleFilter::Stopped;
                }
            });
            ui.add_space(8.0);

            if app.config.rules.is_empty() {
                empty_rules_state(app, ui);
                return;
            }

            let mut actions = Vec::new();
            egui::ScrollArea::vertical()
                .id_salt("rules-scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let rules = app.config.rules.clone();
                    let mut visible = 0usize;
                    for rule in rules {
                        let status = app.status(rule.id);
                        if !matches_filter(app, &rule, &status) {
                            continue;
                        }
                        visible += 1;
                        let runtime = app.runtimes.get(&rule.id);
                        let history = runtime
                            .map(|runtime| runtime.history.iter().copied().collect::<Vec<_>>())
                            .unwrap_or_default();
                        let rate = runtime.map(|runtime| runtime.up_rate).unwrap_or(0.0);
                        let connections = runtime
                            .and_then(|runtime| runtime.stats.as_ref())
                            .map(|stats| stats.active_connections)
                            .unwrap_or(0);
                        if let Some(action) =
                            rule_card(ui, app, &rule, &status, rate, connections, &history)
                        {
                            actions.push((rule.id, action));
                        }
                    }
                    if visible == 0 {
                        ui.add_space(24.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new(tr(app.lang, "rules.no_match"))
                                    .size(12.0)
                                    .color(p.text_soft),
                            );
                            ui.label(
                                RichText::new(tr(app.lang, "rules.no_match_body"))
                                    .size(10.0)
                                    .color(p.faint),
                            );
                        });
                    }
                });

            for (id, action) in actions {
                match action {
                    CardAction::Select => app.selected = Some(id),
                    CardAction::Toggle => app.toggle_rule(id),
                    CardAction::Copy => app.copy_listen_address(ui.ctx(), id),
                    CardAction::Edit => app.open_edit_rule(id),
                    CardAction::Restart => app.restart_rule(id),
                    CardAction::Duplicate => app.duplicate_rule(id),
                    CardAction::Delete => app.pending_delete = Some(id),
                }
            }
        });
}

fn matches_filter(app: &PortPilotApp, rule: &ForwardRule, status: &RuleStatus) -> bool {
    let query = app.search.trim().to_ascii_lowercase();
    let search_matches = query.is_empty()
        || rule.name.to_ascii_lowercase().contains(&query)
        || rule.bind_port.to_string().contains(&query)
        || rule.target_host.to_ascii_lowercase().contains(&query)
        || rule.target_port.to_string().contains(&query);
    let status_matches = match app.filter {
        RuleFilter::All => true,
        RuleFilter::Running => matches!(status, RuleStatus::Running | RuleStatus::Starting),
        RuleFilter::Stopped => !matches!(status, RuleStatus::Running | RuleStatus::Starting),
    };
    search_matches && status_matches
}

fn empty_rules_state(app: &mut PortPilotApp, ui: &mut Ui) {
    let p = app.palette;
    ui.add_space(46.0);
    ui.vertical_centered(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(54.0, 54.0), Sense::hover());
        ui.painter()
            .circle_filled(rect.center(), 25.0, p.accent_soft);
        paint_nav_icon(ui.painter(), rect.shrink(17.0), NavKind::Routes, p.accent);
        ui.add_space(12.0);
        ui.label(
            RichText::new(tr(app.lang, "rules.empty_title"))
                .size(13.0)
                .strong()
                .color(p.text),
        );
        ui.label(
            RichText::new(tr(app.lang, "rules.empty_body"))
                .size(10.5)
                .color(p.muted),
        );
        ui.add_space(14.0);
        if accent_button(ui, &format!("+  {}", tr(app.lang, "top.new_rule")), p).clicked() {
            app.open_new_rule();
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn rule_card(
    ui: &mut Ui,
    app: &mut PortPilotApp,
    rule: &ForwardRule,
    status: &RuleStatus,
    rate: f64,
    connections: u64,
    history: &[(f64, f64)],
) -> Option<CardAction> {
    let p = app.palette;
    let selected = app.selected == Some(rule.id);
    let top_left = ui.cursor().min;
    let estimated = Rect::from_min_size(top_left, vec2(ui.available_width(), 84.0));
    let hovered = ui
        .ctx()
        .pointer_hover_pos()
        .is_some_and(|pointer| estimated.contains(pointer));
    let show_actions = selected || hovered;
    let mut action = None;
    let fill = if selected {
        p.panel_raised
    } else if hovered {
        p.hover
    } else {
        Color32::TRANSPARENT
    };
    let stroke_color = if selected {
        p.border_strong
    } else if hovered {
        p.border
    } else {
        Color32::TRANSPARENT
    };

    let response = egui::Frame::NONE
        .fill(fill)
        .stroke(Stroke::new(1.0, stroke_color))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(11, 9))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                protocol_chip(ui, rule.protocol, p);
                ui.add_space(2.0);
                ui.add(
                    egui::Label::new(RichText::new(&rule.name).size(12.5).strong().color(p.text))
                        .truncate(),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if show_actions {
                        let symbol = if status.is_running() { "■" } else { "▶" };
                        let color = if status.is_running() { p.red } else { p.accent };
                        let button =
                            egui::Button::new(RichText::new(symbol).size(10.0).color(color))
                                .fill(p.panel_soft)
                                .stroke(Stroke::new(1.0, p.border))
                                .corner_radius(CornerRadius::same(5))
                                .min_size(vec2(24.0, 22.0));
                        if ui.add(button).clicked() {
                            action = Some(CardAction::Toggle);
                        }
                    } else {
                        ui.add_space(24.0);
                    }
                    status_widget(ui, status, app.lang, p);
                });
            });
            ui.add_space(5.0);
            ui.add(
                egui::Label::new(
                    RichText::new(format!(
                        "{}  →  {}",
                        rule.listener_endpoint(),
                        rule.target_endpoint()
                    ))
                    .monospace()
                    .size(10.5)
                    .color(p.text_soft),
                )
                .truncate(),
            );
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format_rate(rate as u64))
                        .font(FontId::monospace(9.5))
                        .color(p.muted),
                );
                ui.label(
                    RichText::new(if rule.protocol == Protocol::Udp {
                        format!("{} {}", connections, tr(app.lang, "rule.sessions"))
                    } else {
                        format!("{} {}", connections, tr(app.lang, "rule.connections"))
                    })
                    .font(FontId::monospace(9.5))
                    .color(p.faint),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if show_actions {
                        let more =
                            ui.menu_button(RichText::new("⋯").size(13.0).color(p.muted), |ui| {
                                if ui.button(tr(app.lang, "rule.edit")).clicked() {
                                    action = Some(CardAction::Edit);
                                    ui.close();
                                }
                                if ui.button(tr(app.lang, "detail.restart")).clicked() {
                                    action = Some(CardAction::Restart);
                                    ui.close();
                                }
                                if ui.button(tr(app.lang, "detail.duplicate")).clicked() {
                                    action = Some(CardAction::Duplicate);
                                    ui.close();
                                }
                                if ui.button(tr(app.lang, "detail.delete")).clicked() {
                                    action = Some(CardAction::Delete);
                                    ui.close();
                                }
                            });
                        if more.response.clicked() && action.is_none() {
                            action = Some(CardAction::Select);
                        }
                        let copy = copy_button(ui, 22.0, p);
                        if copy.clicked() {
                            action = Some(CardAction::Copy);
                        }
                        copy.on_hover_text(tr(app.lang, "rule.copy_address"));
                    } else {
                        sparkline(
                            ui,
                            history,
                            if status.is_running() {
                                p.accent
                            } else {
                                p.faint
                            },
                            42.0,
                        );
                    }
                });
            });
        });

    if selected {
        let rect = response.response.rect;
        ui.painter().rect_filled(
            Rect::from_min_size(pos2(rect.left() - 1.0, rect.top() + 14.0), vec2(2.0, 28.0)),
            CornerRadius::same(1),
            p.accent,
        );
    }
    let card_response = ui.interact(
        response.response.rect,
        Id::unique(("rule-card", rule.id)),
        Sense::click(),
    );
    if action.is_none() && card_response.clicked() {
        action = Some(CardAction::Select);
    }
    action
}

fn protocol_chip(ui: &mut Ui, protocol: Protocol, p: Palette) {
    let (color, fill) = match protocol {
        Protocol::Tcp => (p.blue, p.blue_soft),
        Protocol::Udp => (p.violet, p.violet_soft),
    };
    egui::Frame::NONE
        .fill(fill)
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(6, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(protocol.label())
                    .font(FontId::monospace(8.0))
                    .strong()
                    .color(color),
            );
        });
}

fn status_widget(ui: &mut Ui, status: &RuleStatus, lang: Lang, p: Palette) {
    let color = p.status_color(status);
    let (rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
    if status.is_running() {
        ui.painter().circle_filled(rect.center(), 6.0, p.green_soft);
    }
    ui.painter().circle_filled(rect.center(), 3.0, color);
    ui.label(
        RichText::new(status_text(status, lang))
            .size(9.5)
            .color(if status.is_running() {
                p.green
            } else {
                p.muted
            }),
    );
}

fn status_text(status: &RuleStatus, lang: Lang) -> &'static str {
    match status {
        RuleStatus::Stopped => tr(lang, "status.stopped"),
        RuleStatus::Starting => tr(lang, "status.starting"),
        RuleStatus::Running => tr(lang, "status.running"),
        RuleStatus::Stopping => tr(lang, "status.stopping"),
        RuleStatus::Failed(_) => tr(lang, "status.failed"),
    }
}

fn filter_chip(ui: &mut Ui, selected: bool, text: &str, p: Palette) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(10.0).color(if selected {
            p.accent
        } else {
            p.muted
        }))
        .fill(if selected {
            p.accent_soft
        } else {
            Color32::TRANSPARENT
        })
        .stroke(Stroke::new(
            1.0,
            if selected {
                p.accent.gamma_multiply(0.24)
            } else {
                Color32::TRANSPARENT
            },
        ))
        .corner_radius(CornerRadius::same(7))
        .min_size(vec2(0.0, 27.0)),
    )
}

fn sparkline(ui: &mut Ui, history: &[(f64, f64)], color: Color32, width: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 16.0), Sense::hover());
    let painter = ui.painter_at(rect);
    let max = history
        .iter()
        .map(|(up, down)| up.max(*down))
        .fold(1.0_f64, f64::max);
    let points: Vec<_> = if history.len() < 2 {
        vec![
            pos2(rect.left(), rect.bottom() - 2.0),
            pos2(rect.right(), rect.bottom() - 2.0),
        ]
    } else {
        history
            .iter()
            .enumerate()
            .map(|(index, (_, down))| {
                let x = rect.left() + rect.width() * index as f32 / (history.len() - 1) as f32;
                let y = rect.bottom() - (down / max).clamp(0.0, 1.0) as f32 * (rect.height() - 2.0);
                pos2(x, y)
            })
            .collect()
    };
    painter.add(Shape::line(points, Stroke::new(1.3, color)));
}

fn detail_panel(app: &mut PortPilotApp, ui: &mut Ui, height: f32) {
    let p = app.palette;
    egui::Frame::NONE
        .fill(p.panel_soft)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(14))
        .show(ui, |ui| {
            ui.set_min_size(vec2(ui.available_width(), height - 2.0));
            let Some(rule) = app.selected_rule().cloned() else {
                empty_detail(ui, app.lang, p);
                return;
            };
            detail_header(app, ui, &rule);
            ui.painter().line_segment(
                [
                    pos2(ui.max_rect().left(), ui.cursor().top()),
                    pos2(ui.max_rect().right(), ui.cursor().top()),
                ],
                Stroke::new(1.0, p.border),
            );
            egui::Frame::NONE
                .inner_margin(Margin {
                    left: 17,
                    right: 17,
                    top: 14,
                    bottom: 16,
                })
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("detail-scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            detail_content(app, ui, &rule);
                        });
                });
        });
}

fn empty_detail(ui: &mut Ui, lang: Lang, p: Palette) {
    ui.vertical_centered(|ui| {
        ui.add_space(90.0);
        let (rect, _) = ui.allocate_exact_size(vec2(64.0, 64.0), Sense::hover());
        ui.painter()
            .circle_filled(rect.center(), 30.0, p.accent_soft);
        paint_nav_icon(ui.painter(), rect.shrink(20.0), NavKind::Routes, p.accent);
        ui.add_space(14.0);
        ui.label(
            RichText::new(tr(lang, "detail.no_rule"))
                .size(14.0)
                .strong()
                .color(p.text),
        );
        ui.label(
            RichText::new(tr(lang, "detail.no_rule_body"))
                .size(10.5)
                .color(p.muted),
        );
    });
}

fn detail_header(app: &mut PortPilotApp, ui: &mut Ui, rule: &ForwardRule) {
    let p = app.palette;
    let status = app.status(rule.id);
    let started = app
        .runtimes
        .get(&rule.id)
        .and_then(|runtime| runtime.stats.as_ref())
        .and_then(|stats| stats.started_at)
        .filter(|_| app.is_active(rule.id));
    egui::Frame::NONE
        .fill(p.panel)
        .inner_margin(Margin {
            left: 20,
            right: 20,
            top: 16,
            bottom: 14,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        protocol_chip(ui, rule.protocol, p);
                        status_widget(ui, &status, app.lang, p);
                        if let Some(started) = started {
                            ui.label(
                                RichText::new(format_duration(started.elapsed().as_secs()))
                                    .font(FontId::monospace(9.0))
                                    .color(p.faint),
                            );
                        }
                    });
                    ui.add_space(4.0);
                    ui.label(RichText::new(&rule.name).size(20.0).strong().color(p.text));
                    ui.label(
                        RichText::new(format!(
                            "{} → {}",
                            rule.listener_endpoint(),
                            rule.target_endpoint()
                        ))
                        .size(10.5)
                        .color(p.muted),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if error_button(
                        ui,
                        &format!(
                            "■  {}",
                            tr(
                                app.lang,
                                if app.is_active(rule.id) {
                                    "detail.stop"
                                } else {
                                    "detail.start"
                                }
                            )
                        ),
                        p,
                    )
                    .clicked()
                    {
                        app.toggle_rule(rule.id);
                    }
                    ui.menu_button(RichText::new("⋯").size(18.0).color(p.muted), |ui| {
                        if ui.button(tr(app.lang, "detail.restart")).clicked() {
                            app.restart_rule(rule.id);
                            ui.close();
                        }
                        if ui.button(tr(app.lang, "rule.edit")).clicked() {
                            app.open_edit_rule(rule.id);
                            ui.close();
                        }
                        if ui.button(tr(app.lang, "detail.duplicate")).clicked() {
                            app.duplicate_rule(rule.id);
                            ui.close();
                        }
                        if ui.button(tr(app.lang, "detail.delete")).clicked() {
                            app.pending_delete = Some(rule.id);
                            ui.close();
                        }
                    });
                    if copy_button(ui, 32.0, p)
                        .on_hover_text(tr(app.lang, "detail.copy"))
                        .clicked()
                    {
                        app.copy_listen_address(ui.ctx(), rule.id);
                    }
                });
            });
        });
}

fn detail_content(app: &mut PortPilotApp, ui: &mut Ui, rule: &ForwardRule) {
    let p = app.palette;
    route_card(ui, rule, app.lang, p);
    ui.add_space(11.0);
    stats_grid(app, ui, rule);
    ui.add_space(2.0);
    traffic_card(app, ui, rule);
    ui.add_space(11.0);
    recent_events(app, ui, rule);
}

fn route_card(ui: &mut Ui, rule: &ForwardRule, lang: Lang, p: Palette) {
    egui::Frame::NONE
        .fill(p.panel)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new(tr(lang, "detail.route"))
                    .font(FontId::monospace(8.5))
                    .color(p.faint),
            );
            ui.add_space(7.0);
            let width = ui.available_width();
            let content_height = 42.0;
            ui.allocate_ui_with_layout(
                vec2(width, content_height),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.allocate_ui_with_layout(
                        vec2(width * 0.43, content_height),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            endpoint(
                                ui,
                                true,
                                tr(lang, "detail.local_listener"),
                                &rule.listener_endpoint(),
                                p.accent,
                                p,
                                false,
                            );
                        },
                    );
                    ui.allocate_ui_with_layout(
                        vec2(width * 0.14, content_height),
                        Layout::centered_and_justified(egui::Direction::LeftToRight),
                        |ui| {
                            ui.label(RichText::new("→").size(18.0).color(p.faint));
                        },
                    );
                    ui.allocate_ui_with_layout(
                        vec2(width * 0.43, content_height),
                        Layout::right_to_left(Align::Center),
                        |ui| {
                            endpoint(
                                ui,
                                false,
                                tr(lang, "detail.target_service"),
                                &rule.target_endpoint(),
                                p.blue,
                                p,
                                true,
                            );
                        },
                    );
                },
            );
        });
}

#[allow(clippy::too_many_arguments)]
fn endpoint(
    ui: &mut Ui,
    local: bool,
    label: &str,
    address: &str,
    color: Color32,
    p: Palette,
    align_right: bool,
) {
    let layout = if align_right {
        Layout::right_to_left(Align::Center)
    } else {
        Layout::left_to_right(Align::Center)
    };
    ui.with_layout(layout, |ui| {
        ui.horizontal(|ui| {
            let (icon_rect, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
            ui.painter()
                .rect_filled(icon_rect, CornerRadius::same(8), color.gamma_multiply(0.12));
            ui.painter().rect_stroke(
                icon_rect,
                CornerRadius::same(8),
                Stroke::new(1.0, color.gamma_multiply(0.28)),
                StrokeKind::Inside,
            );
            paint_endpoint_icon(ui.painter(), icon_rect.shrink(7.0), local, color);
            ui.vertical(|ui| {
                ui.label(RichText::new(label).size(8.5).color(p.faint));
                ui.label(
                    RichText::new(address)
                        .font(FontId::monospace(11.5))
                        .strong()
                        .color(p.text),
                );
            });
        });
    });
}

fn stats_grid(app: &PortPilotApp, ui: &mut Ui, rule: &ForwardRule) {
    let p = app.palette;
    let runtime = app.runtimes.get(&rule.id);
    let stats = runtime.and_then(|runtime| runtime.stats.as_ref());
    let up = runtime.map(|runtime| runtime.up_rate).unwrap_or(0.0);
    let down = runtime.map(|runtime| runtime.down_rate).unwrap_or(0.0);
    let active = stats.map(|stats| stats.active_connections).unwrap_or(0);
    let latency = stats
        .and_then(|stats| stats.average_latency_ms)
        .map(|value| format!("{value} ms"))
        .unwrap_or_else(|| tr(app.lang, "common.none").to_owned());
    let uptime = stats
        .and_then(|stats| stats.started_at)
        .filter(|_| app.is_active(rule.id))
        .map(|started| format_duration(started.elapsed().as_secs()))
        .unwrap_or_else(|| "00:00:00".to_owned());
    let total_bytes = stats
        .map(|stats| format_bytes(stats.bytes_up.saturating_add(stats.bytes_down)))
        .unwrap_or_else(|| format_bytes(0));
    let width = ui.available_width();
    let columns = if width >= 620.0 { 4 } else { 2 };
    let gap = 8.0;
    let cell_width = (width - gap * (columns - 1) as f32) / columns as f32;

    if columns == 4 {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            stat_card(
                ui,
                cell_width,
                tr(app.lang, "detail.upload"),
                &format_rate(up as u64),
                &format!("↑ {}", format_bytes(stats.map(|s| s.bytes_up).unwrap_or(0))),
                p,
            );
            stat_card(
                ui,
                cell_width,
                tr(app.lang, "detail.download"),
                &format_rate(down as u64),
                &format!(
                    "↓ {}",
                    format_bytes(stats.map(|s| s.bytes_down).unwrap_or(0))
                ),
                p,
            );
            stat_card(
                ui,
                cell_width,
                tr(app.lang, "detail.active_connections"),
                &active.to_string(),
                &format!("{} {}", tr(app.lang, "detail.avg_latency"), latency),
                p,
            );
            stat_card(
                ui,
                cell_width,
                tr(app.lang, "detail.lifetime"),
                &total_bytes,
                &format!("{} {}", tr(app.lang, "detail.uptime"), uptime),
                p,
            );
        });
    } else {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.vertical(|ui| {
                stat_card(
                    ui,
                    cell_width,
                    tr(app.lang, "detail.upload"),
                    &format_rate(up as u64),
                    &format!("↑ {}", format_bytes(stats.map(|s| s.bytes_up).unwrap_or(0))),
                    p,
                );
                ui.add_space(8.0);
                stat_card(
                    ui,
                    cell_width,
                    tr(app.lang, "detail.active_connections"),
                    &active.to_string(),
                    &format!("{} {}", tr(app.lang, "detail.avg_latency"), latency),
                    p,
                );
            });
            ui.vertical(|ui| {
                stat_card(
                    ui,
                    cell_width,
                    tr(app.lang, "detail.download"),
                    &format_rate(down as u64),
                    &format!(
                        "↓ {}",
                        format_bytes(stats.map(|s| s.bytes_down).unwrap_or(0))
                    ),
                    p,
                );
                ui.add_space(8.0);
                stat_card(
                    ui,
                    cell_width,
                    tr(app.lang, "detail.lifetime"),
                    &total_bytes,
                    &format!("{} {}", tr(app.lang, "detail.uptime"), uptime),
                    p,
                );
            });
        });
    }
}

fn stat_card(ui: &mut Ui, width: f32, label: &str, value: &str, foot: &str, p: Palette) {
    egui::Frame::NONE
        .fill(p.panel)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(11, 9))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_min_size(vec2(width - 24.0, 54.0));
                ui.label(RichText::new(label).size(9.0).color(p.faint));
                ui.add_space(4.0);
                ui.label(
                    RichText::new(value)
                        .font(FontId::monospace(16.0))
                        .strong()
                        .color(p.text),
                );
                ui.label(RichText::new(foot).size(8.5).color(p.muted));
            });
        });
}

fn traffic_card(app: &PortPilotApp, ui: &mut Ui, rule: &ForwardRule) {
    let p = app.palette;
    let history: Vec<(f64, f64)> = app
        .runtimes
        .get(&rule.id)
        .map(|runtime| runtime.history.iter().copied().collect())
        .unwrap_or_default();
    let current_up = history.last().map(|(up, _)| *up).unwrap_or(0.0);
    let current_down = history.last().map(|(_, down)| *down).unwrap_or(0.0);
    let peak = history
        .iter()
        .map(|(up, down)| up.max(*down))
        .fold(0.0_f64, f64::max);
    egui::Frame::NONE
        .fill(p.panel)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(tr(app.lang, "detail.traffic"))
                            .size(11.5)
                            .strong()
                            .color(p.text),
                    );
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(tr(app.lang, "detail.traffic_sub"))
                                .size(9.0)
                                .color(p.faint),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} ↑{} ↓{} · {} {}",
                                tr(app.lang, "detail.current"),
                                format_rate(current_up as u64),
                                format_rate(current_down as u64),
                                tr(app.lang, "detail.peak"),
                                format_rate(peak as u64)
                            ))
                            .font(FontId::monospace(8.5))
                            .color(p.muted),
                        );
                    });
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "— {}   — {}",
                            tr(app.lang, "detail.upload_short"),
                            tr(app.lang, "detail.download_short")
                        ))
                        .size(8.5)
                        .color(p.muted),
                    );
                });
            });
            ui.add_space(6.0);
            draw_traffic_chart(ui, &history, p);
        });
}

fn draw_traffic_chart(ui: &mut Ui, history: &[(f64, f64)], p: Palette) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, 150.0), Sense::hover());
    let painter = ui.painter_at(rect);
    let plot = Rect::from_min_max(
        pos2(rect.left() + 42.0, rect.top() + 2.0),
        pos2(rect.right() - 4.0, rect.bottom() - 18.0),
    );
    let max = history
        .iter()
        .map(|(up, down)| up.max(*down))
        .fold(1024.0_f64, f64::max);
    for index in 0..4 {
        let t = index as f32 / 3.0;
        let y = plot.top() + plot.height() * t;
        painter.line_segment(
            [pos2(plot.left(), y), pos2(plot.right(), y)],
            Stroke::new(1.0, p.chart_grid),
        );
        let value = max * (1.0 - t as f64);
        painter.text(
            pos2(rect.left(), y),
            Align2::LEFT_CENTER,
            format_rate(value as u64),
            FontId::monospace(7.5),
            p.faint,
        );
    }
    for (index, label) in ["-60s", "-45s", "-30s", "now"].iter().enumerate() {
        let x = plot.left() + plot.width() * index as f32 / 3.0;
        painter.text(
            pos2(x, rect.bottom() - 3.0),
            if index == 0 {
                Align2::LEFT_BOTTOM
            } else if index == 3 {
                Align2::RIGHT_BOTTOM
            } else {
                Align2::CENTER_BOTTOM
            },
            *label,
            FontId::monospace(7.5),
            p.faint,
        );
    }
    let points = |select: fn(&(f64, f64)) -> f64| -> Vec<egui::Pos2> {
        if history.len() < 2 {
            return vec![
                pos2(plot.left(), plot.bottom()),
                pos2(plot.right(), plot.bottom()),
            ];
        }
        history
            .iter()
            .enumerate()
            .map(|(index, point)| {
                let x = plot.left() + plot.width() * index as f32 / (history.len() - 1) as f32;
                let y =
                    plot.bottom() - (select(point) / max).clamp(0.0, 1.0) as f32 * plot.height();
                pos2(x, y)
            })
            .collect()
    };
    let up_points = points(|(up, _)| *up);
    let down_points = points(|(_, down)| *down);
    painter.add(Shape::line(
        up_points.clone(),
        Stroke::new(5.0, p.accent.gamma_multiply(0.08)),
    ));
    painter.add(Shape::line(up_points, Stroke::new(1.6, p.accent)));
    painter.add(Shape::line(down_points, Stroke::new(1.3, p.blue)));
}

fn recent_events(app: &mut PortPilotApp, ui: &mut Ui, rule: &ForwardRule) {
    let p = app.palette;
    let entries: Vec<LogEntry> = app
        .logs
        .recent(80)
        .into_iter()
        .filter(|entry| entry.rule_id == Some(rule.id))
        .collect();
    egui::Frame::NONE
        .fill(p.panel)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(12))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.add_space(13.0);
                ui.label(
                    RichText::new(tr(app.lang, "detail.recent_events"))
                        .size(10.5)
                        .strong()
                        .color(p.text),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(tr(app.lang, "detail.view_all"))
                                    .size(8.5)
                                    .color(p.accent),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE),
                        )
                        .clicked()
                    {
                        app.view = View::Logs;
                    }
                    ui.add_space(5.0);
                });
            });
            ui.add_space(2.0);
            if entries.is_empty() {
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(tr(app.lang, "logs.empty"))
                            .size(9.5)
                            .color(p.faint),
                    );
                });
                ui.add_space(10.0);
            } else {
                ui.add_space(3.0);
                for entry in entries.iter().rev().take(7) {
                    log_row(ui, entry, p, false);
                }
            }
        });
}

fn log_row(ui: &mut Ui, entry: &LogEntry, p: Palette, wide: bool) {
    let level_color = match entry.level {
        LogLevel::Info => p.green,
        LogLevel::Warn => p.amber,
        LogLevel::Error => p.red,
    };
    let height = if wide { 28.0 } else { 21.0 };
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    if wide {
        ui.painter().line_segment(
            [rect.left_top(), rect.right_top()],
            Stroke::new(1.0, p.border.gamma_multiply(0.5)),
        );
    }
    ui.painter().text(
        pos2(rect.left() + 2.0, rect.center().y),
        Align2::LEFT_CENTER,
        entry.timestamp.format("%H:%M:%S").to_string(),
        FontId::monospace(if wide { 9.0 } else { 8.0 }),
        p.faint,
    );
    ui.painter().text(
        pos2(
            rect.left() + if wide { 74.0 } else { 66.0 },
            rect.center().y,
        ),
        Align2::LEFT_CENTER,
        entry.level.label(),
        FontId::monospace(if wide { 8.5 } else { 7.5 }),
        level_color,
    );
    let message_left = if wide { 123.0 } else { 105.0 };
    let mut message = entry.message.clone();
    if wide && !entry.rule_name.is_empty() {
        message = format!("[{}] {message}", entry.rule_name);
    }
    let available = (rect.width() - message_left - 3.0).max(10.0);
    let galley = ui.painter().layout(
        message,
        FontId::monospace(if wide { 9.0 } else { 8.0 }),
        p.muted,
        available,
    );
    ui.painter().galley(
        pos2(
            rect.left() + message_left,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        p.muted,
    );
}

fn logs_view(app: &mut PortPilotApp, ui: &mut Ui) {
    let p = app.palette;
    let height = ui.available_height();
    egui::Frame::NONE
        .fill(p.panel)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(Margin::symmetric(15, 14))
        .show(ui, |ui| {
            ui.set_min_size(vec2(ui.available_width(), height - 30.0));
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(tr(app.lang, "logs.title"))
                        .size(13.0)
                        .strong()
                        .color(p.text),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if secondary_button(ui, tr(app.lang, "logs.export"), p).clicked() {
                        export_logs(app);
                    }
                    if secondary_button(ui, tr(app.lang, "logs.clear"), p).clicked() {
                        app.logs.clear();
                    }
                });
            });
            ui.add_space(9.0);
            ui.horizontal(|ui| {
                egui::Frame::NONE
                    .fill(p.panel_soft)
                    .stroke(Stroke::new(1.0, p.border))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(9, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            search_icon(ui, p.faint);
                            ui.add(
                                TextEdit::singleline(&mut app.logs_view.search)
                                    .hint_text(tr(app.lang, "logs.search"))
                                    .desired_width(260.0)
                                    .frame(egui::Frame::NONE),
                            );
                        });
                    });
                ui.add_space(8.0);
                if filter_chip(
                    ui,
                    app.logs_view.level.is_none(),
                    tr(app.lang, "logs.all_levels"),
                    p,
                )
                .clicked()
                {
                    app.logs_view.level = None;
                }
                for (level, key) in [
                    (LogLevel::Info, "logs.info"),
                    (LogLevel::Warn, "logs.warn"),
                    (LogLevel::Error, "logs.error"),
                ] {
                    if filter_chip(ui, app.logs_view.level == Some(level), tr(app.lang, key), p)
                        .clicked()
                    {
                        app.logs_view.level = Some(level);
                    }
                }
            });
            ui.add_space(8.0);
            let entries: Vec<LogEntry> = app
                .logs
                .snapshot()
                .into_iter()
                .rev()
                .filter(|entry| log_matches(app, entry))
                .collect();
            if entries.is_empty() {
                ui.add_space(80.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(tr(app.lang, "logs.empty"))
                            .size(11.0)
                            .color(p.faint),
                    );
                });
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("logs-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for entry in &entries {
                            log_row(ui, entry, p, true);
                        }
                    });
            }
        });
}

fn log_matches(app: &PortPilotApp, entry: &LogEntry) -> bool {
    let search = app.logs_view.search.trim().to_ascii_lowercase();
    let level_ok = app.logs_view.level.is_none_or(|level| level == entry.level);
    let search_ok = search.is_empty()
        || entry.message.to_ascii_lowercase().contains(&search)
        || entry.rule_name.to_ascii_lowercase().contains(&search);
    level_ok && search_ok
}

fn export_logs(app: &mut PortPilotApp) {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Log", &["log", "txt"])
        .set_file_name("portpilot.log")
        .save_file()
    else {
        return;
    };
    let search = app.logs_view.search.trim().to_ascii_lowercase();
    let level = app.logs_view.level;
    let result = app.logs.export(&path, |entry| {
        level.is_none_or(|level| level == entry.level)
            && (search.is_empty()
                || entry.message.to_ascii_lowercase().contains(&search)
                || entry.rule_name.to_ascii_lowercase().contains(&search))
    });
    match result {
        Ok(_) => {
            app.logs
                .info(None, "", tr(app.lang, "notice.logs_exported"));
            app.show_toast(tr(app.lang, "notice.logs_exported"));
        }
        Err(error) => {
            app.logs.error(
                None,
                "",
                format!("{}: {error}", tr(app.lang, "log.log_export_failed")),
            );
            app.show_toast(format!("{}: {error}", tr(app.lang, "notice.export_failed")));
        }
    }
}

fn settings_view(app: &mut PortPilotApp, ui: &mut Ui) {
    let p = app.palette;
    egui::ScrollArea::vertical()
        .id_salt("settings-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            settings_card(ui, p, |ui| {
                section_title(ui, tr(app.lang, "settings.appearance"), p);
                ui.add_space(9.0);
                ui.label(
                    RichText::new(tr(app.lang, "settings.language"))
                        .size(9.5)
                        .color(p.muted),
                );
                ui.horizontal(|ui| {
                    for (language, key) in [
                        (Language::System, "settings.system"),
                        (Language::ZhCn, "settings.chinese"),
                        (Language::EnUs, "settings.english"),
                    ] {
                        if option_button(ui, app.config.language == language, tr(app.lang, key), p)
                            .clicked()
                        {
                            app.set_language(language);
                        }
                    }
                });
                ui.add_space(10.0);
                ui.label(
                    RichText::new(tr(app.lang, "settings.theme"))
                        .size(9.5)
                        .color(p.muted),
                );
                ui.horizontal(|ui| {
                    for (theme, key) in [
                        (Theme::System, "settings.system"),
                        (Theme::Dark, "settings.dark"),
                        (Theme::Light, "settings.light"),
                    ] {
                        if option_button(ui, app.config.theme == theme, tr(app.lang, key), p)
                            .clicked()
                        {
                            app.set_theme(theme);
                        }
                    }
                });
            });
            ui.add_space(11.0);
            settings_card(ui, p, |ui| {
                section_title(ui, tr(app.lang, "settings.desktop"), p);
                ui.add_space(8.0);
                let mut close_to_tray = app.config.close_to_tray;
                if ui
                    .checkbox(&mut close_to_tray, tr(app.lang, "settings.close_to_tray"))
                    .changed()
                {
                    app.config.close_to_tray = close_to_tray;
                    app.save_config();
                }
                ui.label(
                    RichText::new(tr(app.lang, "settings.close_to_tray_hint"))
                        .size(9.0)
                        .color(p.faint),
                );
                ui.add_space(8.0);
                let available = app.autostart.is_some();
                let mut autostart = app
                    .autostart
                    .as_ref()
                    .is_some_and(|manager| manager.is_enabled());
                if ui
                    .add_enabled(
                        available,
                        egui::Checkbox::new(&mut autostart, tr(app.lang, "settings.autostart")),
                    )
                    .changed()
                {
                    let result = app
                        .autostart
                        .as_mut()
                        .map(|manager| manager.set_enabled(autostart));
                    match result {
                        Some(Ok(())) => {
                            app.logs.info(
                                None,
                                "",
                                tr(
                                    app.lang,
                                    if autostart {
                                        "log.autostart_enabled"
                                    } else {
                                        "log.autostart_disabled"
                                    },
                                ),
                            );
                        }
                        Some(Err(error)) => {
                            app.logs.error(None, "", error.clone());
                            app.show_toast(format!(
                                "{}: {error}",
                                tr(app.lang, "notice.autostart_failed")
                            ));
                        }
                        None => {
                            app.show_toast(tr(app.lang, "settings.autostart_unavailable"));
                        }
                    }
                }
                ui.label(
                    RichText::new(tr(app.lang, "settings.autostart_hint"))
                        .size(9.0)
                        .color(p.faint),
                );
                if let Some(error) = &app.autostart_error {
                    ui.label(RichText::new(error).size(9.0).color(p.red));
                }
            });
            ui.add_space(11.0);
            settings_card(ui, p, |ui| {
                section_title(ui, tr(app.lang, "settings.rules_file"), p);
                ui.add_space(6.0);
                ui.label(
                    RichText::new(tr(app.lang, "settings.import_hint"))
                        .size(9.0)
                        .color(p.faint),
                );
                ui.add_space(7.0);
                ui.horizontal(|ui| {
                    if secondary_button(ui, tr(app.lang, "settings.import"), p).clicked() {
                        import_rules(app);
                    }
                    if secondary_button(ui, tr(app.lang, "settings.export"), p).clicked() {
                        export_rules(app);
                    }
                });
            });
            ui.add_space(11.0);
            settings_card(ui, p, |ui| {
                section_title(ui, tr(app.lang, "settings.about"), p);
                ui.add_space(8.0);
                key_value(
                    ui,
                    tr(app.lang, "settings.version"),
                    env!("CARGO_PKG_VERSION"),
                    p,
                );
                key_value(
                    ui,
                    tr(app.lang, "settings.platform"),
                    &format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
                    p,
                );
                key_value(
                    ui,
                    tr(app.lang, "settings.config_path"),
                    &app.config_path.display().to_string(),
                    p,
                );
            });
            ui.add_space(12.0);
        });
}

fn settings_card(ui: &mut Ui, p: Palette, content: impl FnOnce(&mut Ui)) {
    egui::Frame::NONE
        .fill(p.panel)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            content(ui);
        });
}

fn section_title(ui: &mut Ui, text: &str, p: Palette) {
    ui.label(RichText::new(text).size(11.0).strong().color(p.text));
}

fn key_value(ui: &mut Ui, key: &str, value: &str, p: Palette) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [145.0, 18.0],
            egui::Label::new(RichText::new(key).size(9.5).color(p.muted)),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new(value)
                .font(FontId::monospace(9.0))
                .color(p.text_soft),
        );
    });
}

fn option_button(ui: &mut Ui, selected: bool, text: &str, p: Palette) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(10.0).color(if selected {
            p.accent
        } else {
            p.muted
        }))
        .fill(if selected {
            p.accent_soft
        } else {
            p.panel_soft
        })
        .stroke(Stroke::new(
            1.0,
            if selected {
                p.accent.gamma_multiply(0.35)
            } else {
                p.border
            },
        ))
        .corner_radius(CornerRadius::same(7))
        .min_size(vec2(0.0, 29.0)),
    )
}

fn import_rules(app: &mut PortPilotApp) {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("JSON", &["json"])
        .pick_file()
    else {
        return;
    };
    match config::import_rules(&path) {
        Ok(rules) => app.pending_import = Some(rules),
        Err(error) => {
            app.logs.error(
                None,
                "",
                format!("{}: {error}", tr(app.lang, "log.import_failed")),
            );
            app.show_toast(format!("{}: {error}", tr(app.lang, "notice.import_failed")));
        }
    }
}

fn export_rules(app: &mut PortPilotApp) {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("JSON", &["json"])
        .set_file_name("portpilot-rules.json")
        .save_file()
    else {
        return;
    };
    match config::export_rules(&app.config.rules, &path) {
        Ok(()) => app.show_toast(tr(app.lang, "notice.exported")),
        Err(error) => {
            app.logs.error(
                None,
                "",
                format!("{}: {error}", tr(app.lang, "log.export_failed")),
            );
            app.show_toast(format!("{}: {error}", tr(app.lang, "notice.export_failed")));
        }
    }
}

fn show_rule_editor(app: &mut PortPilotApp, ctx: &egui::Context) {
    let p = app.palette;
    let lang = app.lang;
    let bind_addresses = app.bind_addresses.clone();
    let mut save = false;
    let mut close = false;
    let Some(editor) = app.editor.as_mut() else {
        return;
    };
    let modal = egui::Modal::new(Id::unique("rule-editor-modal"))
        .frame(modal_frame(p))
        .backdrop_color(Color32::from_black_alpha(150));
    let response = modal.show(ctx, |ui| {
        ui.set_width(520.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(tr(
                        lang,
                        if editor.original_id.is_some() {
                            "dialog.edit_rule"
                        } else {
                            "dialog.new_rule"
                        },
                    ))
                    .size(17.0)
                    .strong()
                    .color(p.text),
                );
                ui.label(
                    RichText::new(tr(lang, "dialog.safety"))
                        .size(9.0)
                        .color(p.faint),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icon_text_button(ui, "×", p).clicked() {
                    close = true;
                }
            });
        });
        ui.add_space(12.0);
        form_label(ui, tr(lang, "dialog.rule_name"), p);
        ui.add(
            TextEdit::singleline(&mut editor.name)
                .hint_text(if lang == Lang::Zh {
                    "例如：本地 API"
                } else {
                    "e.g. Local API"
                })
                .desired_width(f32::INFINITY),
        );
        field_error(ui, editor, RuleIssue::EmptyName, lang, p);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                form_label(ui, tr(lang, "dialog.protocol"), p);
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut editor.protocol, Protocol::Tcp, "TCP");
                    ui.selectable_value(&mut editor.protocol, Protocol::Udp, "UDP");
                });
            });
            ui.add_space(16.0);
            ui.vertical(|ui| {
                form_label(ui, tr(lang, "dialog.listen_port"), p);
                ui.add(TextEdit::singleline(&mut editor.bind_port).desired_width(100.0));
            });
            ui.vertical(|ui| {
                form_label(ui, tr(lang, "dialog.target_port"), p);
                ui.add(TextEdit::singleline(&mut editor.target_port).desired_width(100.0));
            });
        });
        field_error(ui, editor, RuleIssue::InvalidListenPort, lang, p);
        field_error(ui, editor, RuleIssue::InvalidTargetPort, lang, p);
        ui.add_space(8.0);
        form_label(ui, tr(lang, "dialog.listen_address"), p);
        let selected = editor.bind_ip.clone();
        egui::ComboBox::from_id_salt("bind-address")
            .selected_text(selected.clone())
            .width(220.0)
            .show_ui(ui, |ui| {
                for address in &bind_addresses {
                    let text = address.to_string();
                    if ui
                        .selectable_label(selected == text, text.clone())
                        .clicked()
                    {
                        editor.bind_ip = text;
                    }
                }
            });
        field_error(ui, editor, RuleIssue::LocalBindRequired, lang, p);
        field_error(ui, editor, RuleIssue::WildcardBind, lang, p);
        field_error(ui, editor, RuleIssue::DuplicateListener, lang, p);
        ui.add_space(8.0);
        form_label(ui, tr(lang, "dialog.target_host"), p);
        ui.add(
            TextEdit::singleline(&mut editor.target_host)
                .hint_text("localhost")
                .desired_width(f32::INFINITY),
        );
        field_error(ui, editor, RuleIssue::EmptyTargetHost, lang, p);
        field_error(ui, editor, RuleIssue::InvalidTargetHost, lang, p);
        ui.add_space(9.0);
        ui.checkbox(&mut editor.enabled, tr(lang, "dialog.enable_now"));
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if accent_button(ui, tr(lang, "dialog.save"), p).clicked() {
                save = true;
            }
            if secondary_button(ui, tr(lang, "dialog.cancel"), p).clicked() {
                close = true;
            }
        });
    });
    if response.backdrop_response.clicked() {
        close = true;
    }
    if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
        close = true;
    }
    if save {
        app.try_save_editor();
    } else if close {
        app.editor = None;
    }
}

fn show_delete_dialog(app: &mut PortPilotApp, ctx: &egui::Context) {
    let p = app.palette;
    let Some(id) = app.pending_delete else {
        return;
    };
    let name = app
        .rule(id)
        .map(|rule| rule.name.clone())
        .unwrap_or_default();
    let modal = egui::Modal::new(Id::unique("delete-rule-modal"))
        .frame(modal_frame(p))
        .backdrop_color(Color32::from_black_alpha(140));
    let response = modal.show(ctx, |ui| {
        ui.set_width(400.0);
        ui.label(
            RichText::new(tr(app.lang, "dialog.delete_title"))
                .size(16.0)
                .strong()
                .color(p.text),
        );
        ui.add_space(5.0);
        ui.label(
            RichText::new(format!("{}: {}", name, tr(app.lang, "dialog.delete_body")))
                .size(10.0)
                .color(p.muted),
        );
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            if error_button(ui, tr(app.lang, "dialog.delete_confirm"), p).clicked() {
                app.delete_rule(id);
                app.pending_delete = None;
            }
            if secondary_button(ui, tr(app.lang, "dialog.cancel"), p).clicked() {
                app.pending_delete = None;
            }
        });
    });
    if response.backdrop_response.clicked() {
        app.pending_delete = None;
    }
}

fn show_import_dialog(app: &mut PortPilotApp, ctx: &egui::Context) {
    let p = app.palette;
    let count = app.pending_import.as_ref().map(Vec::len).unwrap_or(0);
    let modal = egui::Modal::new(Id::unique("import-rules-modal"))
        .frame(modal_frame(p))
        .backdrop_color(Color32::from_black_alpha(140));
    let response = modal.show(ctx, |ui| {
        ui.set_width(420.0);
        ui.label(
            RichText::new(tr(app.lang, "dialog.import_title"))
                .size(16.0)
                .strong()
                .color(p.text),
        );
        ui.add_space(5.0);
        ui.label(
            RichText::new(format!("{} ({count})", tr(app.lang, "dialog.import_body")))
                .size(10.0)
                .color(p.muted),
        );
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            if accent_button(ui, tr(app.lang, "dialog.import_confirm"), p).clicked()
                && let Some(rules) = app.pending_import.take()
            {
                app.import_rules(rules);
            }
            if secondary_button(ui, tr(app.lang, "dialog.cancel"), p).clicked() {
                app.pending_import = None;
            }
        });
    });
    if response.backdrop_response.clicked() {
        app.pending_import = None;
    }
}

fn modal_frame(p: Palette) -> egui::Frame {
    egui::Frame::NONE
        .fill(p.panel)
        .stroke(Stroke::new(1.0, p.border_strong))
        .corner_radius(CornerRadius::same(16))
        .inner_margin(Margin::symmetric(20, 18))
}

fn form_label(ui: &mut Ui, text: &str, p: Palette) {
    ui.label(RichText::new(text).size(9.5).color(p.muted));
}

fn field_error(ui: &mut Ui, editor: &RuleEditor, issue: RuleIssue, lang: Lang, p: Palette) {
    if editor.has_error(issue) {
        ui.label(RichText::new(issue.message(lang)).size(9.0).color(p.red));
    }
}

fn show_toast(app: &PortPilotApp, ctx: &egui::Context) {
    let Some(toast) = &app.toast else {
        return;
    };
    let p = app.palette;
    egui::Area::new(Id::unique("toast-area"))
        .anchor(Align2::RIGHT_BOTTOM, vec2(-20.0, -18.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(p.panel_raised)
                .stroke(Stroke::new(1.0, p.border_strong))
                .corner_radius(CornerRadius::same(9))
                .inner_margin(Margin::symmetric(12, 9))
                .show(ui, |ui| {
                    ui.label(RichText::new(&toast.message).size(10.0).color(p.text));
                });
        });
}

fn theme_button(ui: &mut Ui, p: Palette) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(8), p.hover);
    }
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(8),
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );
    let center = rect.center();
    let color = p.muted;
    if p.dark {
        ui.painter()
            .circle_stroke(center, 3.8, Stroke::new(1.4, color));
        for (dx, dy) in [
            (0.0, -1.0),
            (0.0, 1.0),
            (-1.0, 0.0),
            (1.0, 0.0),
            (-0.7, -0.7),
            (0.7, -0.7),
            (-0.7, 0.7),
            (0.7, 0.7),
        ] {
            ui.painter().line_segment(
                [
                    center + vec2(dx * 6.0, dy * 6.0),
                    center + vec2(dx * 8.0, dy * 8.0),
                ],
                Stroke::new(1.2, color),
            );
        }
    } else {
        ui.painter()
            .circle_filled(center + vec2(1.0, 0.5), 5.0, color);
        ui.painter()
            .circle_filled(center + vec2(3.2, -1.8), 4.8, p.shell);
    }
    response
}

fn search_icon(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
    let center = rect.center() + vec2(-1.0, -1.0);
    ui.painter()
        .circle_stroke(center, 4.0, Stroke::new(1.3, color));
    ui.painter().line_segment(
        [center + vec2(3.0, 3.0), center + vec2(6.0, 6.0)],
        Stroke::new(1.3, color),
    );
}

fn copy_button(ui: &mut Ui, size: f32, p: Palette) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(5), p.hover);
    }
    let icon = Rect::from_center_size(rect.center(), vec2(size * 0.48, size * 0.48));
    let color = if response.hovered() { p.text } else { p.muted };
    ui.painter().rect_stroke(
        Rect::from_min_size(icon.min + vec2(2.0, 0.0), icon.size() - vec2(2.0, 2.0)),
        CornerRadius::same(2),
        Stroke::new(1.2, color),
        StrokeKind::Inside,
    );
    ui.painter().line_segment(
        [
            icon.left_top() + vec2(0.0, 3.0),
            icon.left_bottom() - vec2(0.0, 1.0),
        ],
        Stroke::new(1.2, color),
    );
    ui.painter().line_segment(
        [
            icon.left_top() + vec2(3.0, 0.0),
            icon.right_top() - vec2(1.0, 0.0),
        ],
        Stroke::new(1.2, color),
    );
    response
}

fn paint_endpoint_icon(painter: &egui::Painter, rect: Rect, local: bool, color: Color32) {
    let stroke = Stroke::new(1.3, color);
    if local {
        let screen = Rect::from_min_max(
            pos2(rect.left() + 1.0, rect.top() + 1.0),
            pos2(rect.right() - 1.0, rect.bottom() - 4.0),
        );
        painter.rect_stroke(screen, CornerRadius::same(2), stroke, StrokeKind::Inside);
        painter.line_segment(
            [
                pos2(rect.center().x, screen.bottom()),
                pos2(rect.center().x, rect.bottom() - 1.0),
            ],
            stroke,
        );
        painter.line_segment(
            [
                pos2(rect.center().x - 3.0, rect.bottom() - 1.0),
                pos2(rect.center().x + 3.0, rect.bottom() - 1.0),
            ],
            stroke,
        );
    } else {
        for top in [rect.top(), rect.center().y + 1.0] {
            let server = Rect::from_min_size(
                pos2(rect.left(), top),
                vec2(rect.width(), rect.height() * 0.38),
            );
            painter.rect_stroke(server, CornerRadius::same(2), stroke, StrokeKind::Inside);
            painter.circle_filled(pos2(server.left() + 3.0, server.center().y), 1.0, color);
        }
    }
}

fn accent_button(ui: &mut Ui, text: &str, p: Palette) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(text)
                .size(11.5)
                .strong()
                .color(Color32::from_rgb(4, 32, 28)),
        )
        .fill(p.accent)
        .stroke(Stroke::NONE)
        .corner_radius(CornerRadius::same(8))
        .min_size(vec2(0.0, 32.0)),
    )
}

fn secondary_button(ui: &mut Ui, text: &str, p: Palette) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(10.5).color(p.text_soft))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, p.border_strong))
            .corner_radius(CornerRadius::same(8))
            .min_size(vec2(0.0, 32.0)),
    )
}

fn error_button(ui: &mut Ui, text: &str, p: Palette) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(10.5).strong().color(p.red))
            .fill(p.red_soft)
            .stroke(Stroke::new(1.0, p.red.gamma_multiply(0.28)))
            .corner_radius(CornerRadius::same(8))
            .min_size(vec2(0.0, 32.0)),
    )
}

fn icon_text_button(ui: &mut Ui, text: &str, p: Palette) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(15.0).color(p.muted))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, p.border))
            .corner_radius(CornerRadius::same(8))
            .min_size(vec2(32.0, 32.0)),
    )
}
