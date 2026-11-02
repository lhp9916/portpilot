use std::fmt;
use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    #[default]
    Tcp,
    Udp,
}

impl Protocol {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tcp => "TCP",
            Self::Udp => "UDP",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    System,
    ZhCn,
    EnUs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    System,
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForwardRule {
    pub id: Uuid,
    pub name: String,
    pub protocol: Protocol,
    pub bind_ip: IpAddr,
    pub bind_port: u16,
    pub target_host: String,
    pub target_port: u16,
    #[serde(default)]
    pub enabled: bool,
}

impl ForwardRule {
    pub fn new(bind_ip: IpAddr, default_port: u16) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            protocol: Protocol::Tcp,
            bind_ip,
            bind_port: default_port,
            target_host: "127.0.0.1".to_owned(),
            target_port: 3000,
            enabled: false,
        }
    }

    pub fn listener_endpoint(&self) -> String {
        format_host_port(&self.bind_ip.to_string(), self.bind_port)
    }

    pub fn target_endpoint(&self) -> String {
        format_host_port(self.target_host.trim(), self.target_port)
    }

    pub fn normalized_target_host(&self) -> String {
        self.target_host
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_owned()
    }
}

pub fn format_host_port(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleIssue {
    EmptyName,
    InvalidListenPort,
    InvalidTargetPort,
    EmptyTargetHost,
    InvalidTargetHost,
    WildcardBind,
    LocalBindRequired,
    DuplicateListener,
}

impl RuleIssue {
    pub fn message(self, language: crate::i18n::Lang) -> &'static str {
        use crate::i18n::tr;

        match self {
            Self::EmptyName => tr(language, "error.empty_name"),
            Self::InvalidListenPort => tr(language, "error.invalid_listen_port"),
            Self::InvalidTargetPort => tr(language, "error.invalid_target_port"),
            Self::EmptyTargetHost => tr(language, "error.empty_target_host"),
            Self::InvalidTargetHost => tr(language, "error.invalid_target_host"),
            Self::WildcardBind => tr(language, "error.wildcard_bind"),
            Self::LocalBindRequired => tr(language, "error.local_bind_required"),
            Self::DuplicateListener => tr(language, "error.duplicate_listener"),
        }
    }
}

impl fmt::Display for RuleIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

pub fn validate_rule(rule: &ForwardRule, all_rules: &[ForwardRule]) -> Vec<RuleIssue> {
    let mut issues = Vec::new();

    if rule.name.trim().is_empty() {
        issues.push(RuleIssue::EmptyName);
    }
    if rule.bind_port == 0 {
        issues.push(RuleIssue::InvalidListenPort);
    }
    if rule.target_port == 0 {
        issues.push(RuleIssue::InvalidTargetPort);
    }
    if rule.target_host.trim().is_empty() {
        issues.push(RuleIssue::EmptyTargetHost);
    } else if !is_valid_target_host(&rule.target_host) {
        issues.push(RuleIssue::InvalidTargetHost);
    }
    if rule.bind_ip.is_unspecified() {
        issues.push(RuleIssue::WildcardBind);
    }
    if all_rules.iter().any(|other| {
        other.id != rule.id
            && other.protocol == rule.protocol
            && other.bind_ip == rule.bind_ip
            && other.bind_port == rule.bind_port
    }) {
        issues.push(RuleIssue::DuplicateListener);
    }

    issues
}

pub fn duplicate_listener_exists(
    protocol: Protocol,
    bind_ip: IpAddr,
    bind_port: u16,
    except_id: Uuid,
    rules: &[ForwardRule],
) -> bool {
    rules.iter().any(|rule| {
        rule.id != except_id
            && rule.protocol == protocol
            && rule.bind_ip == bind_ip
            && rule.bind_port == bind_port
    })
}

fn is_valid_target_host(host: &str) -> bool {
    let host = host.trim();
    !host.is_empty()
        && !host.contains("://")
        && !host.contains(char::is_whitespace)
        && !host.contains('/')
        && !host.contains('\\')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(port: u16) -> ForwardRule {
        ForwardRule {
            id: Uuid::new_v4(),
            name: "Web".to_owned(),
            protocol: Protocol::Tcp,
            bind_ip: "127.0.0.1".parse().unwrap(),
            bind_port: port,
            target_host: "localhost".to_owned(),
            target_port: 3000,
            enabled: false,
        }
    }

    #[test]
    fn validates_ports_and_names() {
        let mut candidate = rule(0);
        candidate.name.clear();
        let issues = validate_rule(&candidate, &[]);
        assert!(issues.contains(&RuleIssue::EmptyName));
        assert!(issues.contains(&RuleIssue::InvalidListenPort));
    }

    #[test]
    fn detects_duplicate_tcp_listener() {
        let first = rule(8080);
        let mut second = rule(8080);
        second.id = Uuid::new_v4();
        let issues = validate_rule(&second, &[first]);
        assert!(issues.contains(&RuleIssue::DuplicateListener));
    }

    #[test]
    fn formats_ipv6_endpoint() {
        assert_eq!(format_host_port("::1", 8080), "[::1]:8080");
        assert_eq!(format_host_port("localhost", 80), "localhost:80");
    }
}
