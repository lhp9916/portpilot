use crate::model::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Zh,
    En,
}

pub fn resolve_language(language: Language) -> Lang {
    match language {
        Language::ZhCn => Lang::Zh,
        Language::EnUs => Lang::En,
        Language::System => {
            let is_chinese = sys_locale::get_locale()
                .map(|locale| locale.to_ascii_lowercase().starts_with("zh"))
                .unwrap_or(false);
            if is_chinese { Lang::Zh } else { Lang::En }
        }
    }
}

pub fn tr(lang: Lang, key: &str) -> &'static str {
    match key {
        "app.name" => ("PortPilot", "PortPilot"),
        "app.brand_subtitle" => ("本地端口转发", "Local port forwarding"),
        "nav.rules" => ("端口转发", "Forwarding"),
        "nav.logs" => ("日志", "Logs"),
        "nav.settings" => ("设置", "Settings"),
        "nav.engine_online" => ("转发引擎正常", "Forwarding engine online"),
        "top.eyebrow" => ("LOCAL PORT CONTROL", "LOCAL PORT CONTROL"),
        "top.title" => ("端口转发", "Port forwarding"),
        "top.subtitle" => ("管理本机监听端口与目标服务之间的连接", "Manage local listeners and target services"),
        "top.start_all" => ("全部启动", "Start all"),
        "top.stop_all" => ("全部停止", "Stop all"),
        "top.new_rule" => ("新建转发", "New rule"),
        "rules.title" => ("转发规则", "Forwarding rules"),
        "rules.search" => ("搜索名称、端口或目标…", "Search name, port, or target…"),
        "rules.all" => ("全部", "All"),
        "rules.running" => ("运行中", "Running"),
        "rules.stopped" => ("已停止", "Stopped"),
        "rules.empty_title" => ("还没有转发规则", "No forwarding rules yet"),
        "rules.empty_body" => ("创建一条规则，把本机端口连接到目标服务。", "Create a rule to connect a local port to a target service."),
        "rules.no_match" => ("没有匹配的规则", "No matching rules"),
        "rules.no_match_body" => ("调整搜索关键词或筛选条件。", "Adjust the search or filter."),
        "rule.connections" => ("连接", "conn."),
        "rule.sessions" => ("会话", "sessions"),
        "rule.copy_address" => ("复制监听地址", "Copy listen address"),
        "rule.more" => ("更多操作", "More actions"),
        "rule.edit" => ("编辑规则", "Edit rule"),
        "rule.start" => ("启动规则", "Start rule"),
        "rule.stop" => ("停止规则", "Stop rule"),
        "rule.enabled" => ("启用", "Enabled"),
        "status.running" => ("运行中", "Running"),
        "status.stopped" => ("已停止", "Stopped"),
        "status.starting" => ("启动中", "Starting"),
        "status.stopping" => ("停止中", "Stopping"),
        "status.failed" => ("失败", "Failed"),
        "detail.route" => ("CONNECTION ROUTE", "CONNECTION ROUTE"),
        "detail.local_listener" => ("本机监听", "Local listener"),
        "detail.target_service" => ("目标服务", "Target service"),
        "detail.upload" => ("上行速率", "Upload rate"),
        "detail.download" => ("下行速率", "Download rate"),
        "detail.active_connections" => ("活跃连接", "Active connections"),
        "detail.lifetime" => ("累计流量", "Lifetime traffic"),
        "detail.total" => ("累计", "total"),
        "detail.peak" => ("峰值", "peak"),
        "detail.avg_latency" => ("平均延迟", "avg latency"),
        "detail.uptime" => ("已运行", "uptime"),
        "detail.traffic" => ("实时流量", "Live traffic"),
        "detail.traffic_sub" => ("最近 60 秒 · 每秒采样", "Last 60 seconds · 1s sample"),
        "detail.current" => ("当前", "Current"),
        "detail.upload_short" => ("上行", "Upload"),
        "detail.download_short" => ("下行", "Download"),
        "detail.recent_events" => ("最近事件", "Recent events"),
        "detail.view_all" => ("查看全部日志 →", "View all logs →"),
        "detail.restart" => ("重启", "Restart"),
        "detail.duplicate" => ("复制规则", "Duplicate rule"),
        "detail.delete" => ("删除规则", "Delete rule"),
        "detail.stop" => ("停止", "Stop"),
        "detail.start" => ("启动", "Start"),
        "detail.copy" => ("复制", "Copy"),
        "detail.more" => ("更多", "More"),
        "detail.no_rule" => ("选择一条规则查看详情", "Select a rule to inspect details"),
        "detail.no_rule_body" => ("左侧列表中的规则会显示连接路径、流量和事件。", "Rules in the list show route, traffic, and events."),
        "logs.title" => ("运行日志", "Runtime logs"),
        "logs.subtitle" => ("仅保存在内存中，最多 5000 条", "In-memory only, up to 5,000 entries"),
        "logs.level" => ("级别", "Level"),
        "logs.search" => ("搜索日志…", "Search logs…"),
        "logs.export" => ("导出日志", "Export logs"),
        "logs.clear" => ("清空", "Clear"),
        "logs.empty" => ("暂无日志", "No logs yet"),
        "logs.all_levels" => ("全部级别", "All levels"),
        "logs.info" => ("信息", "Info"),
        "logs.warn" => ("警告", "Warn"),
        "logs.error" => ("错误", "Error"),
        "settings.title" => ("设置", "Settings"),
        "settings.subtitle" => ("外观、语言、系统集成与规则文件", "Appearance, language, system integration, and rule files"),
        "settings.appearance" => ("外观", "Appearance"),
        "settings.language" => ("界面语言", "Interface language"),
        "settings.system" => ("跟随系统", "System"),
        "settings.chinese" => ("简体中文", "Simplified Chinese"),
        "settings.english" => ("English", "English"),
        "settings.theme" => ("主题", "Theme"),
        "settings.dark" => ("深色", "Dark"),
        "settings.light" => ("浅色", "Light"),
        "settings.desktop" => ("桌面集成", "Desktop integration"),
        "settings.close_to_tray" => ("关闭窗口时最小化到托盘", "Close to tray"),
        "settings.close_to_tray_hint" => ("托盘不可用时将正常退出", "If the tray is unavailable, closing exits normally"),
        "settings.autostart" => ("开机自动启动", "Launch at login"),
        "settings.autostart_hint" => ("自启时静默运行到托盘", "Start silently in the tray"),
        "settings.autostart_unavailable" => ("当前系统无法配置开机自启", "Launch at login is unavailable on this system"),
        "settings.rules_file" => ("规则文件", "Rule files"),
        "settings.import" => ("导入规则", "Import rules"),
        "settings.export" => ("导出规则", "Export rules"),
        "settings.import_hint" => ("导入会替换当前全部规则", "Import replaces all current rules"),
        "settings.config_path" => ("配置目录", "Config directory"),
        "settings.about" => ("关于", "About"),
        "settings.version" => ("版本", "Version"),
        "settings.platform" => ("平台", "Platform"),
        "dialog.new_rule" => ("新建转发规则", "New forwarding rule"),
        "dialog.edit_rule" => ("编辑转发规则", "Edit forwarding rule"),
        "dialog.rule_name" => ("规则名称", "Rule name"),
        "dialog.protocol" => ("协议", "Protocol"),
        "dialog.listen_address" => ("监听地址", "Listen address"),
        "dialog.listen_port" => ("监听端口", "Listen port"),
        "dialog.target_host" => ("目标主机", "Target host"),
        "dialog.target_port" => ("目标端口", "Target port"),
        "dialog.enable_now" => ("创建后立即启用", "Enable after saving"),
        "dialog.safety" => ("监听地址仅允许回环或本机具体网卡地址，禁止使用通配地址。", "Listeners are limited to loopback or a specific local interface; wildcard addresses are not allowed."),
        "dialog.cancel" => ("取消", "Cancel"),
        "dialog.save" => ("保存", "Save"),
        "dialog.create" => ("创建规则", "Create rule"),
        "dialog.delete_title" => ("删除转发规则？", "Delete forwarding rule?"),
        "dialog.delete_body" => ("规则停止后会从配置中删除，此操作不可撤销。", "The rule will stop and be removed from the configuration."),
        "dialog.delete_confirm" => ("删除", "Delete"),
        "dialog.import_title" => ("替换全部规则？", "Replace all rules?"),
        "dialog.import_body" => ("导入会停止当前全部规则并替换为文件中的规则。", "Import stops all current rules and replaces them with the file contents."),
        "dialog.import_confirm" => ("导入并替换", "Import and replace"),
        "error.empty_name" => ("请输入规则名称", "Enter a rule name"),
        "error.invalid_listen_port" => ("监听端口必须在 1-65535 之间", "Listen port must be between 1 and 65535"),
        "error.invalid_target_port" => ("目标端口必须在 1-65535 之间", "Target port must be between 1 and 65535"),
        "error.empty_target_host" => ("请输入目标主机", "Enter a target host"),
        "error.invalid_target_host" => ("目标主机不能包含协议、路径或空格", "Target host cannot contain a scheme, path, or spaces"),
        "error.wildcard_bind" => ("禁止监听通配地址，请选择具体网卡地址", "Wildcard listeners are not allowed; choose a specific interface"),
        "error.local_bind_required" => ("请选择本机当前存在的网卡地址", "Choose an address that exists on this machine"),
        "error.duplicate_listener" => ("该监听地址和端口已被其他规则占用", "This listener is already used by another rule"),
        "error.port_text" => ("请输入有效端口", "Enter a valid port"),
        "error.generic" => ("操作失败", "Operation failed"),
        "notice.copied" => ("已复制监听地址", "Listen address copied"),
        "notice.exported" => ("规则已导出", "Rules exported"),
        "notice.imported" => ("规则已导入", "Rules imported"),
        "notice.logs_exported" => ("日志已导出", "Logs exported"),
        "notice.saved" => ("设置已保存", "Settings saved"),
        "notice.autostart_failed" => ("无法更新开机自启设置", "Could not update launch-at-login"),
        "notice.tray_unavailable" => ("系统托盘不可用，关闭窗口将退出应用", "System tray unavailable; closing will exit"),
        "notice.config_recovered" => ("配置文件损坏，已备份并恢复默认配置", "Configuration was corrupt; backed up and restored defaults"),
        "notice.export_failed" => ("导出失败", "Export failed"),
        "notice.import_failed" => ("导入失败", "Import failed"),
        "notice.target_resolve_failed" => ("无法解析目标主机", "Could not resolve target host"),
        "log.rule_started" => ("规则已启动", "Rule started"),
        "log.rule_stopped" => ("规则已停止", "Rule stopped"),
        "log.listener_failed" => ("监听失败", "Listener failed"),
        "log.connection_opened" => ("新连接", "Connection opened"),
        "log.connection_closed" => ("连接已关闭", "Connection closed"),
        "log.target_failed" => ("无法连接目标", "Target connection failed"),
        "log.udp_session_failed" => ("UDP 会话创建失败", "UDP session failed"),
        "log.udp_session_limit" => ("UDP 会话数达到上限", "UDP session limit reached"),
        "log.accept_failed" => ("接收连接失败", "Accept failed"),
        "log.config_saved" => ("配置已保存", "Configuration saved"),
        "log.config_save_failed" => ("配置保存失败", "Could not save configuration"),
        "log.autostart_enabled" => ("已启用开机自启", "Launch at login enabled"),
        "log.autostart_disabled" => ("已关闭开机自启", "Launch at login disabled"),
        "log.tray_failed" => ("系统托盘初始化失败", "Tray initialization failed"),
        "log.import_failed" => ("导入规则失败", "Rule import failed"),
        "log.export_failed" => ("导出规则失败", "Rule export failed"),
        "log.log_export_failed" => ("导出日志失败", "Log export failed"),
        "unit.b_per_s" => ("B/s", "B/s"),
        "unit.kb_per_s" => ("KB/s", "KB/s"),
        "unit.mb_per_s" => ("MB/s", "MB/s"),
        "unit.gb_per_s" => ("GB/s", "GB/s"),
        "unit.b" => ("B", "B"),
        "unit.kb" => ("KB", "KB"),
        "unit.mb" => ("MB", "MB"),
        "unit.gb" => ("GB", "GB"),
        "common.never" => ("从未", "Never"),
        "common.none" => ("—", "—"),
        "common.close" => ("关闭", "Close"),
        "common.confirm" => ("确认", "Confirm"),
        _ => ("…", "…"),
    }
    .pipe(lang)
}

trait SelectLanguage {
    fn pipe(self, lang: Lang) -> &'static str;
}

impl SelectLanguage for (&'static str, &'static str) {
    fn pipe(self, lang: Lang) -> &'static str {
        match lang {
            Lang::Zh => self.0,
            Lang::En => self.1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_known_and_unknown_keys() {
        assert_eq!(tr(Lang::Zh, "rules.title"), "转发规则");
        assert_eq!(tr(Lang::En, "rules.title"), "Forwarding rules");
        assert_eq!(tr(Lang::En, "missing.key"), "…");
    }
}
