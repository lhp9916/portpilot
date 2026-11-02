use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, copy_bidirectional};
use tokio::net::{TcpListener, TcpStream, UdpSocket, lookup_host};
use tokio::runtime::Runtime;
use tokio::sync::{Semaphore, mpsc};
use tokio::time::{interval, sleep, timeout};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::log_store::LogStore;
use crate::model::{ForwardRule, Protocol, format_host_port};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_SESSIONS: usize = 1024;
const UDP_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RuleStatus {
    #[default]
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed(String),
}

impl RuleStatus {
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }
}

#[derive(Debug, Clone)]
pub struct RuleStatsSnapshot {
    pub active_connections: u64,
    pub total_connections: u64,
    pub bytes_up: u64,
    pub bytes_down: u64,
    pub errors: u64,
    pub average_latency_ms: Option<u64>,
    pub last_error: Option<String>,
    pub started_at: Option<Instant>,
}

pub enum EngineEvent {
    Status(Uuid, RuleStatus),
    Stats(Uuid, RuleStatsSnapshot),
}

pub enum EngineCommand {
    Start(ForwardRule),
    Stop(Uuid),
    StopAll,
    Shutdown,
}

pub struct EngineHandle {
    commands: mpsc::UnboundedSender<EngineCommand>,
    events: Receiver<EngineEvent>,
    runtime: Option<Runtime>,
}

impl EngineHandle {
    pub fn new(logs: Arc<LogStore>) -> Result<Self, EngineError> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("portpilot-forwarder")
            .build()
            .map_err(EngineError::Runtime)?;
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = std::sync::mpsc::channel();
        runtime.spawn(engine_loop(command_rx, event_tx, logs));
        Ok(Self {
            commands: command_tx,
            events: event_rx,
            runtime: Some(runtime),
        })
    }

    pub fn send(&self, command: EngineCommand) {
        let _ = self.commands.send(command);
    }

    pub fn try_recv(&self) -> Option<EngineEvent> {
        self.events.try_recv().ok()
    }
}

impl Drop for EngineHandle {
    fn drop(&mut self) {
        let _ = self.commands.send(EngineCommand::Shutdown);
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_timeout(Duration::from_secs(2));
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("could not start forwarding runtime: {0}")]
    Runtime(#[from] std::io::Error),
}

struct RunningRule {
    cancel: CancellationToken,
    stats: Arc<RuleStats>,
    started_at: Instant,
    name: String,
}

struct RuleStats {
    active_connections: AtomicU64,
    total_connections: AtomicU64,
    bytes_up: AtomicU64,
    bytes_down: AtomicU64,
    errors: AtomicU64,
    latency_sum_ms: AtomicU64,
    latency_count: AtomicU64,
    last_error: std::sync::Mutex<Option<String>>,
    started_at: Instant,
}

impl RuleStats {
    fn new() -> Self {
        Self {
            active_connections: AtomicU64::new(0),
            total_connections: AtomicU64::new(0),
            bytes_up: AtomicU64::new(0),
            bytes_down: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            latency_sum_ms: AtomicU64::new(0),
            latency_count: AtomicU64::new(0),
            last_error: std::sync::Mutex::new(None),
            started_at: Instant::now(),
        }
    }

    fn connection_started(self: &Arc<Self>) -> ActiveConnection {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        self.total_connections.fetch_add(1, Ordering::Relaxed);
        ActiveConnection {
            stats: Arc::clone(self),
        }
    }

    fn add_up(&self, bytes: u64) {
        self.bytes_up.fetch_add(bytes, Ordering::Relaxed);
    }

    fn add_down(&self, bytes: u64) {
        self.bytes_down.fetch_add(bytes, Ordering::Relaxed);
    }

    fn record_latency(&self, duration: Duration) {
        self.latency_sum_ms
            .fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
        self.latency_count.fetch_add(1, Ordering::Relaxed);
    }

    fn record_error(&self, message: impl Into<String>) {
        self.errors.fetch_add(1, Ordering::Relaxed);
        *self
            .last_error
            .lock()
            .unwrap_or_else(|lock| lock.into_inner()) = Some(message.into());
    }

    fn snapshot(&self) -> RuleStatsSnapshot {
        let latency_count = self.latency_count.load(Ordering::Relaxed);
        RuleStatsSnapshot {
            active_connections: self.active_connections.load(Ordering::Relaxed),
            total_connections: self.total_connections.load(Ordering::Relaxed),
            bytes_up: self.bytes_up.load(Ordering::Relaxed),
            bytes_down: self.bytes_down.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            average_latency_ms: (latency_count > 0)
                .then(|| self.latency_sum_ms.load(Ordering::Relaxed) / latency_count),
            last_error: self
                .last_error
                .lock()
                .unwrap_or_else(|lock| lock.into_inner())
                .clone(),
            started_at: Some(self.started_at),
        }
    }
}

struct ActiveConnection {
    stats: Arc<RuleStats>,
}

impl Drop for ActiveConnection {
    fn drop(&mut self) {
        self.stats
            .active_connections
            .fetch_sub(1, Ordering::Relaxed);
    }
}

async fn engine_loop(
    mut commands: mpsc::UnboundedReceiver<EngineCommand>,
    events: Sender<EngineEvent>,
    logs: Arc<LogStore>,
) {
    let mut running: HashMap<Uuid, RunningRule> = HashMap::new();
    let mut stats_tick = interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            command = commands.recv() => {
                match command {
                    Some(EngineCommand::Start(rule)) => {
                        stop_rule(&mut running, rule.id, &events, &logs, "restart");
                        let _ = events.send(EngineEvent::Status(rule.id, RuleStatus::Starting));
                        let result = start_rule(&rule, &events, &logs).await;
                        match result {
                            Ok((cancel, stats, started_at)) => {
                                logs.info(
                                    Some(rule.id),
                                    rule.name.clone(),
                                    format!(
                                        "{} {} -> {}",
                                        rule.protocol.label(),
                                        rule.listener_endpoint(),
                                        rule.target_endpoint()
                                    ),
                                );
                                running.insert(
                                    rule.id,
                                    RunningRule {
                                        cancel,
                                        stats,
                                        started_at,
                                        name: rule.name.clone(),
                                    },
                                );
                                let _ = events.send(EngineEvent::Status(rule.id, RuleStatus::Running));
                            }
                            Err(message) => {
                                logs.error(Some(rule.id), rule.name.clone(), message.clone());
                                let _ = events.send(EngineEvent::Status(
                                    rule.id,
                                    RuleStatus::Failed(message),
                                ));
                            }
                        }
                    }
                    Some(EngineCommand::Stop(id)) => {
                        stop_rule(&mut running, id, &events, &logs, "stop");
                    }
                    Some(EngineCommand::StopAll) => {
                        let ids: Vec<Uuid> = running.keys().copied().collect();
                        for id in ids {
                            stop_rule(&mut running, id, &events, &logs, "stop");
                        }
                    }
                    Some(EngineCommand::Shutdown) | None => {
                        for (id, rule) in running.drain() {
                            rule.cancel.cancel();
                            let _ = events.send(EngineEvent::Status(id, RuleStatus::Stopped));
                        }
                        break;
                    }
                }
            }
            _ = stats_tick.tick() => {
                for (id, rule) in &running {
                    let _ = events.send(EngineEvent::Stats(*id, rule.stats.snapshot()));
                    let _ = rule.started_at;
                }
            }
        }
    }
}

fn stop_rule(
    running: &mut HashMap<Uuid, RunningRule>,
    id: Uuid,
    events: &Sender<EngineEvent>,
    logs: &LogStore,
    reason: &str,
) {
    if let Some(rule) = running.remove(&id) {
        let _ = events.send(EngineEvent::Status(id, RuleStatus::Stopping));
        rule.cancel.cancel();
        logs.info(
            Some(id),
            rule.name,
            if reason == "restart" {
                "restarting listener"
            } else {
                "listener stopped"
            },
        );
        let _ = events.send(EngineEvent::Status(id, RuleStatus::Stopped));
    }
}

async fn start_rule(
    rule: &ForwardRule,
    events: &Sender<EngineEvent>,
    logs: &Arc<LogStore>,
) -> Result<(CancellationToken, Arc<RuleStats>, Instant), String> {
    let address = SocketAddr::new(rule.bind_ip, rule.bind_port);
    let stats = Arc::new(RuleStats::new());
    let started_at = Instant::now();
    let cancel = CancellationToken::new();

    match rule.protocol {
        Protocol::Tcp => {
            let listener = TcpListener::bind(address)
                .await
                .map_err(|error| format!("bind {} failed: {error}", rule.listener_endpoint()))?;
            let task_rule = rule.clone();
            let task_cancel = cancel.clone();
            let task_stats = Arc::clone(&stats);
            let task_logs = Arc::clone(logs);
            let task_events = events.clone();
            tokio::spawn(async move {
                run_tcp(
                    task_rule,
                    listener,
                    task_cancel.clone(),
                    task_stats,
                    task_logs.clone(),
                    task_events.clone(),
                )
                .await;
                let _ = task_events;
            });
        }
        Protocol::Udp => {
            let socket = UdpSocket::bind(address)
                .await
                .map_err(|error| format!("bind {} failed: {error}", rule.listener_endpoint()))?;
            let socket = Arc::new(socket);
            let task_rule = rule.clone();
            let task_cancel = cancel.clone();
            let task_stats = Arc::clone(&stats);
            let task_logs = Arc::clone(logs);
            let task_events = events.clone();
            tokio::spawn(async move {
                run_udp(
                    task_rule,
                    socket,
                    task_cancel.clone(),
                    task_stats,
                    task_logs.clone(),
                    task_events.clone(),
                )
                .await;
                let _ = task_events;
            });
        }
    }

    Ok((cancel, stats, started_at))
}

async fn run_tcp(
    rule: ForwardRule,
    listener: TcpListener,
    cancel: CancellationToken,
    stats: Arc<RuleStats>,
    logs: Arc<LogStore>,
    _events: Sender<EngineEvent>,
) {
    let semaphore = Arc::new(Semaphore::new(MAX_SESSIONS));
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            accepted = listener.accept() => {
                match accepted {
                    Ok((stream, peer)) => {
                        let permit = match Arc::clone(&semaphore).try_acquire_owned() {
                            Ok(permit) => permit,
                            Err(_) => {
                                stats.record_error("connection limit reached");
                                logs.warn(Some(rule.id), rule.name.clone(), "connection limit reached");
                                drop(stream);
                                continue;
                            }
                        };
                        let task_rule = rule.clone();
                        let task_cancel = cancel.clone();
                        let task_stats = Arc::clone(&stats);
                        let task_logs = Arc::clone(&logs);
                        tokio::spawn(async move {
                            handle_tcp_connection(
                                task_rule,
                                stream,
                                peer,
                                task_cancel,
                                task_stats,
                                task_logs,
                            )
                            .await;
                            drop(permit);
                        });
                    }
                    Err(error) => {
                        stats.record_error(format!("accept failed: {error}"));
                        logs.warn(Some(rule.id), rule.name.clone(), format!("accept failed: {error}"));
                        sleep(Duration::from_millis(120)).await;
                    }
                }
            }
        }
    }
}

async fn handle_tcp_connection(
    rule: ForwardRule,
    client: TcpStream,
    peer: SocketAddr,
    cancel: CancellationToken,
    stats: Arc<RuleStats>,
    logs: Arc<LogStore>,
) {
    let _ = client.set_nodelay(true);
    let target_host = rule.normalized_target_host();
    let connect_started = Instant::now();
    let target = tokio::select! {
        _ = cancel.cancelled() => return,
        result = timeout(CONNECT_TIMEOUT, TcpStream::connect((target_host.as_str(), rule.target_port))) => result,
    };

    let target = match target {
        Ok(Ok(stream)) => stream,
        Ok(Err(error)) => {
            stats.record_error(format!("target {} failed: {error}", rule.target_endpoint()));
            logs.warn(
                Some(rule.id),
                rule.name.clone(),
                format!("target {} failed: {error}", rule.target_endpoint()),
            );
            return;
        }
        Err(_) => {
            stats.record_error(format!("target {} timed out", rule.target_endpoint()));
            logs.warn(
                Some(rule.id),
                rule.name.clone(),
                format!("target {} timed out", rule.target_endpoint()),
            );
            return;
        }
    };

    let _ = target.set_nodelay(true);
    stats.record_latency(connect_started.elapsed());
    let _connection = stats.connection_started();
    logs.info(
        Some(rule.id),
        rule.name.clone(),
        format!("connection from {peer}"),
    );

    let mut counted_client = CountingStream::new(client, Arc::clone(&stats), CountDirection::Up);
    let mut counted_target = CountingStream::new(target, Arc::clone(&stats), CountDirection::Down);

    let copied = tokio::select! {
        _ = cancel.cancelled() => return,
        result = copy_bidirectional(&mut counted_client, &mut counted_target) => result,
    };

    match copied {
        Ok((up, down)) => {
            logs.info(
                Some(rule.id),
                rule.name.clone(),
                format!("connection closed: ↑{} ↓{}", up, down),
            );
        }
        Err(error) => {
            let benign = matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::BrokenPipe
            );
            if benign {
                logs.info(
                    Some(rule.id),
                    rule.name.clone(),
                    "connection closed by peer",
                );
            } else {
                stats.record_error(format!("connection error: {error}"));
                logs.warn(
                    Some(rule.id),
                    rule.name.clone(),
                    format!("connection error: {error}"),
                );
            }
        }
    }
}

#[derive(Clone, Copy)]
enum CountDirection {
    Up,
    Down,
}

struct CountingStream<S> {
    inner: S,
    stats: Arc<RuleStats>,
    direction: CountDirection,
}

impl<S> CountingStream<S> {
    fn new(inner: S, stats: Arc<RuleStats>, direction: CountDirection) -> Self {
        Self {
            inner,
            stats,
            direction,
        }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for CountingStream<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buffer.filled().len();
        let result = Pin::new(&mut self.inner).poll_read(cx, buffer);
        if let Poll::Ready(Ok(())) = &result {
            let read = buffer.filled().len().saturating_sub(before) as u64;
            match self.direction {
                CountDirection::Up => self.stats.add_up(read),
                CountDirection::Down => self.stats.add_down(read),
            }
        }
        result
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for CountingStream<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buffer)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

async fn run_udp(
    rule: ForwardRule,
    socket: Arc<UdpSocket>,
    cancel: CancellationToken,
    stats: Arc<RuleStats>,
    logs: Arc<LogStore>,
    _events: Sender<EngineEvent>,
) {
    let (cleanup_tx, mut cleanup_rx) = mpsc::unbounded_channel::<SocketAddr>();
    let mut sessions: HashMap<SocketAddr, mpsc::UnboundedSender<Vec<u8>>> = HashMap::new();
    let mut buffer = vec![0_u8; 65_535];

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            Some(client) = cleanup_rx.recv() => {
                sessions.remove(&client);
            }
            received = socket.recv_from(&mut buffer) => {
                match received {
                    Ok((length, client)) => {
                        let payload = buffer[..length].to_vec();
                        if let Some(sender) = sessions.get(&client) {
                            if sender.send(payload).is_err() {
                                sessions.remove(&client);
                            }
                            continue;
                        }
                        if sessions.len() >= MAX_SESSIONS {
                            stats.record_error("UDP session limit reached");
                            logs.warn(Some(rule.id), rule.name.clone(), "UDP session limit reached");
                            continue;
                        }

                        let (session_tx, session_rx) = mpsc::unbounded_channel();
                        sessions.insert(client, session_tx.clone());
                        let task_rule = rule.clone();
                        let task_cancel = cancel.clone();
                        let task_stats = Arc::clone(&stats);
                        let task_logs = Arc::clone(&logs);
                        let task_socket = Arc::clone(&socket);
                        let task_cleanup = cleanup_tx.clone();
                        tokio::spawn(async move {
                            udp_session(
                                task_rule,
                                client,
                                payload,
                                session_rx,
                                task_socket,
                                task_cancel,
                                task_stats,
                                task_logs,
                                task_cleanup,
                            )
                            .await;
                        });
                    }
                    Err(error) => {
                        stats.record_error(format!("UDP receive failed: {error}"));
                        logs.warn(Some(rule.id), rule.name.clone(), format!("UDP receive failed: {error}"));
                        sleep(Duration::from_millis(120)).await;
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn udp_session(
    rule: ForwardRule,
    client: SocketAddr,
    first_payload: Vec<u8>,
    mut incoming: mpsc::UnboundedReceiver<Vec<u8>>,
    listener: Arc<UdpSocket>,
    cancel: CancellationToken,
    stats: Arc<RuleStats>,
    logs: Arc<LogStore>,
    cleanup: mpsc::UnboundedSender<SocketAddr>,
) {
    let target_host = rule.normalized_target_host();
    let resolved = tokio::select! {
        _ = cancel.cancelled() => {
            let _ = cleanup.send(client);
            return;
        }
        result = timeout(CONNECT_TIMEOUT, lookup_host((target_host.as_str(), rule.target_port))) => result,
    };
    let target = match resolved {
        Ok(Ok(mut addresses)) => match addresses.next() {
            Some(address) => address,
            None => {
                stats.record_error("target resolved to no addresses");
                logs.warn(
                    Some(rule.id),
                    rule.name.clone(),
                    "target resolved to no addresses",
                );
                let _ = cleanup.send(client);
                return;
            }
        },
        Ok(Err(error)) => {
            stats.record_error(format!("target resolve failed: {error}"));
            logs.warn(
                Some(rule.id),
                rule.name.clone(),
                format!("target resolve failed: {error}"),
            );
            let _ = cleanup.send(client);
            return;
        }
        Err(_) => {
            stats.record_error("target resolve timed out");
            logs.warn(Some(rule.id), rule.name.clone(), "target resolve timed out");
            let _ = cleanup.send(client);
            return;
        }
    };

    let bind_address = if target.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    };
    let outbound = match UdpSocket::bind(bind_address).await {
        Ok(socket) => socket,
        Err(error) => {
            stats.record_error(format!("UDP bind failed: {error}"));
            logs.warn(
                Some(rule.id),
                rule.name.clone(),
                format!("UDP bind failed: {error}"),
            );
            let _ = cleanup.send(client);
            return;
        }
    };
    if let Err(error) = outbound.connect(target).await {
        stats.record_error(format!("UDP target failed: {error}"));
        logs.warn(
            Some(rule.id),
            rule.name.clone(),
            format!("UDP target failed: {error}"),
        );
        let _ = cleanup.send(client);
        return;
    }

    let _connection = stats.connection_started();
    if !first_payload.is_empty() {
        match outbound.send(&first_payload).await {
            Ok(sent) => stats.add_up(sent as u64),
            Err(error) => {
                stats.record_error(format!("UDP send failed: {error}"));
                logs.warn(
                    Some(rule.id),
                    rule.name.clone(),
                    format!("UDP send failed: {error}"),
                );
            }
        }
    }

    let mut response = vec![0_u8; 65_535];
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = sleep(UDP_IDLE_TIMEOUT) => break,
            payload = incoming.recv() => {
                match payload {
                    Some(payload) if payload.is_empty() => continue,
                    Some(payload) => {
                        if let Err(error) = outbound.send(&payload).await {
                            stats.record_error(format!("UDP send failed: {error}"));
                            break;
                        }
                        stats.add_up(payload.len() as u64);
                    }
                    None => break,
                }
            }
            received = outbound.recv(&mut response) => {
                match received {
                    Ok(length) => {
                        if let Err(error) = listener.send_to(&response[..length], client).await {
                            stats.record_error(format!("UDP reply failed: {error}"));
                            break;
                        }
                        stats.add_down(length as u64);
                    }
                    Err(error) => {
                        stats.record_error(format!("UDP receive failed: {error}"));
                        break;
                    }
                }
            }
        }
    }
    let _ = cleanup.send(client);
}

pub fn describe_target(host: &str, port: u16) -> String {
    format_host_port(host, port)
}
