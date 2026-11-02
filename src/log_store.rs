use std::collections::VecDeque;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::sync::Mutex;

use chrono::{DateTime, Local};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Local>,
    pub level: LogLevel,
    pub rule_id: Option<Uuid>,
    pub rule_name: String,
    pub message: String,
}

pub struct LogStore {
    entries: Mutex<VecDeque<LogEntry>>,
    capacity: usize,
}

impl LogStore {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Mutex::new(VecDeque::with_capacity(capacity.min(256))),
            capacity,
        }
    }

    pub fn push(
        &self,
        level: LogLevel,
        rule_id: Option<Uuid>,
        rule_name: impl Into<String>,
        message: impl Into<String>,
    ) {
        let mut entries = self.entries.lock().unwrap_or_else(|lock| lock.into_inner());
        while entries.len() >= self.capacity {
            entries.pop_front();
        }
        entries.push_back(LogEntry {
            timestamp: Local::now(),
            level,
            rule_id,
            rule_name: rule_name.into(),
            message: message.into(),
        });
    }

    pub fn info(
        &self,
        rule_id: Option<Uuid>,
        rule_name: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.push(LogLevel::Info, rule_id, rule_name, message);
    }

    pub fn warn(
        &self,
        rule_id: Option<Uuid>,
        rule_name: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.push(LogLevel::Warn, rule_id, rule_name, message);
    }

    pub fn error(
        &self,
        rule_id: Option<Uuid>,
        rule_name: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.push(LogLevel::Error, rule_id, rule_name, message);
    }

    pub fn snapshot(&self) -> Vec<LogEntry> {
        self.entries
            .lock()
            .unwrap_or_else(|lock| lock.into_inner())
            .iter()
            .cloned()
            .collect()
    }

    pub fn recent(&self, limit: usize) -> Vec<LogEntry> {
        let entries = self.entries.lock().unwrap_or_else(|lock| lock.into_inner());
        let start = entries.len().saturating_sub(limit);
        entries.iter().skip(start).cloned().collect()
    }

    pub fn clear(&self) {
        self.entries
            .lock()
            .unwrap_or_else(|lock| lock.into_inner())
            .clear();
    }

    pub fn export<F>(&self, path: &Path, filter: F) -> io::Result<usize>
    where
        F: Fn(&LogEntry) -> bool,
    {
        let entries = self.snapshot();
        let mut file = fs::File::create(path)?;
        let mut written = 0usize;
        for entry in entries.into_iter().filter(filter) {
            writeln!(
                file,
                "{} {:<5} [{}] {}",
                entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
                entry.level.label(),
                if entry.rule_name.is_empty() {
                    "app"
                } else {
                    entry.rule_name.as_str()
                },
                entry.message
            )?;
            written += 1;
        }
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_capacity() {
        let store = LogStore::new(2);
        store.info(None, "", "one");
        store.info(None, "", "two");
        store.info(None, "", "three");
        let entries = store.snapshot();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message, "two");
        assert_eq!(entries[1].message, "three");
    }
}
