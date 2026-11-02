use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use portpilot::engine::{EngineCommand, EngineEvent, EngineHandle, RuleStatsSnapshot, RuleStatus};
use portpilot::log_store::LogStore;
use portpilot::model::{ForwardRule, Protocol};
use uuid::Uuid;

fn free_port(protocol: Protocol) -> u16 {
    match protocol {
        Protocol::Tcp => TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port(),
        Protocol::Udp => UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port(),
    }
}

fn wait_for_status(
    engine: &EngineHandle,
    id: Uuid,
    expected: impl Fn(&RuleStatus) -> bool,
) -> RuleStatus {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(EngineEvent::Status(rule_id, status)) = engine.try_recv()
            && rule_id == id
            && expected(&status)
        {
            return status;
        }
        if Instant::now() >= deadline {
            panic!("timed out waiting for expected rule status");
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn wait_for_stats(
    engine: &EngineHandle,
    id: Uuid,
    expected: impl Fn(&RuleStatsSnapshot) -> bool,
) -> RuleStatsSnapshot {
    let deadline = Instant::now() + Duration::from_secs(6);
    loop {
        if let Some(EngineEvent::Stats(rule_id, stats)) = engine.try_recv()
            && rule_id == id
            && expected(&stats)
        {
            return stats;
        }
        if Instant::now() >= deadline {
            panic!("timed out waiting for expected traffic stats");
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn tcp_rule(bind_port: u16, target_port: u16) -> ForwardRule {
    ForwardRule {
        id: Uuid::new_v4(),
        name: "TCP test".to_owned(),
        protocol: Protocol::Tcp,
        bind_ip: "127.0.0.1".parse().unwrap(),
        bind_port,
        target_host: "127.0.0.1".to_owned(),
        target_port,
        enabled: true,
    }
}

fn udp_rule(bind_port: u16, target_port: u16) -> ForwardRule {
    ForwardRule {
        protocol: Protocol::Udp,
        ..tcp_rule(bind_port, target_port)
    }
}

#[test]
fn forwards_tcp_bidirectionally_and_counts_bytes() {
    let target_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let target_port = target_listener.local_addr().unwrap().port();
    let echo = thread::spawn(move || {
        let (mut stream, _) = target_listener.accept().unwrap();
        let mut buffer = [0_u8; 4];
        stream.read_exact(&mut buffer).unwrap();
        stream.write_all(&buffer).unwrap();
        stream.flush().unwrap();
    });

    let logs = Arc::new(LogStore::new(128));
    let engine = EngineHandle::new(Arc::clone(&logs)).unwrap();
    let listen_port = free_port(Protocol::Tcp);
    let rule = tcp_rule(listen_port, target_port);
    let id = rule.id;
    engine.send(EngineCommand::Start(rule));
    wait_for_status(&engine, id, |status| matches!(status, RuleStatus::Running));

    let deadline = Instant::now() + Duration::from_secs(3);
    let mut client = loop {
        if let Ok(stream) = TcpStream::connect(("127.0.0.1", listen_port)) {
            break stream;
        }
        if Instant::now() >= deadline {
            panic!("unable to connect to forwarding listener");
        }
        thread::sleep(Duration::from_millis(50));
    };

    client
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    client.write_all(b"ping").unwrap();
    let mut response = [0_u8; 4];
    client.read_exact(&mut response).unwrap();
    assert_eq!(&response, b"ping");

    wait_for_stats(&engine, id, |stats| {
        stats.bytes_up >= 4 && stats.bytes_down >= 4
    });
    engine.send(EngineCommand::Stop(id));
    echo.join().unwrap();
}

#[test]
fn forwards_udp_packets() {
    let target = UdpSocket::bind("127.0.0.1:0").unwrap();
    let target_port = target.local_addr().unwrap().port();
    let echo = thread::spawn(move || {
        let mut buffer = [0_u8; 64];
        let (length, peer) = target.recv_from(&mut buffer).unwrap();
        target.send_to(&buffer[..length], peer).unwrap();
    });

    let logs = Arc::new(LogStore::new(128));
    let engine = EngineHandle::new(Arc::clone(&logs)).unwrap();
    let listen_port = free_port(Protocol::Udp);
    let rule = udp_rule(listen_port, target_port);
    let id = rule.id;
    engine.send(EngineCommand::Start(rule));
    wait_for_status(&engine, id, |status| matches!(status, RuleStatus::Running));

    let client = UdpSocket::bind("127.0.0.1:0").unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    client
        .send_to(b"hello", ("127.0.0.1", listen_port))
        .unwrap();
    let mut response = [0_u8; 64];
    let (length, _) = client.recv_from(&mut response).unwrap();
    assert_eq!(&response[..length], b"hello");

    wait_for_stats(&engine, id, |stats| {
        stats.bytes_up >= 5 && stats.bytes_down >= 5
    });
    engine.send(EngineCommand::Stop(id));
    echo.join().unwrap();
}

#[test]
fn reports_listener_conflict() {
    let occupied = TcpListener::bind("127.0.0.1:0").unwrap();
    let occupied_port = occupied.local_addr().unwrap().port();
    let logs = Arc::new(LogStore::new(128));
    let engine = EngineHandle::new(logs).unwrap();
    let rule = tcp_rule(occupied_port, 3000);
    let id = rule.id;
    engine.send(EngineCommand::Start(rule));
    let status = wait_for_status(&engine, id, |status| {
        matches!(status, RuleStatus::Failed(_))
    });
    assert!(matches!(status, RuleStatus::Failed(_)));
    drop(occupied);
}
