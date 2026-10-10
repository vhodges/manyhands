//! Restricted SSH sessions and owned, shell-free Git helper processes.
use super::{Fault, FixtureBoundary, FixtureError, GitHelper, ReceiveUpdate, Shared, fixed};
use russh::{
    Channel, ChannelId,
    server::{self, Auth, Handler, Msg, Session},
};
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    task::JoinSet,
};

pub(super) fn discover_helpers(
    directory: Option<&Path>,
) -> Result<(GitHelper, GitHelper), FixtureError> {
    // Resolve once so subsequent helper launches do not repeat PATH lookup.
    let suffix = std::env::consts::EXE_SUFFIX;
    let path = std::env::var_os("PATH").ok_or(FixtureError)?;
    let git = std::env::split_paths(&path)
        .map(|directory| directory.join(format!("git{suffix}")))
        .find(|program| program.is_file())
        .ok_or(FixtureError)?;
    let git = fixed(std::path::absolute(git))?;
    let mut command = Command::new(&git);
    if let Some(directory) = directory {
        command.env("GIT_EXEC_PATH", directory);
    }
    let mut child = fixed(
        command
            .arg("--exec-path")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn(),
    )?;
    let mut output = child.stdout.take().ok_or(FixtureError)?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = output.by_ref().take(16_385).read_to_end(&mut bytes);
        (result.is_ok(), bytes)
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err(FixtureError);
            }
        }
    };
    let (read_ok, bytes) = fixed(reader.join())?;
    if !status.success() || !read_ok || bytes.len() > 16_384 {
        return Err(FixtureError);
    }
    let value = fixed(String::from_utf8(bytes))?;
    let directory = PathBuf::from(value.trim_end_matches(['\r', '\n']));
    let helper = |name| {
        let program = directory.join(format!("git-{name}{suffix}"));
        if program.is_file() {
            program.into()
        } else {
            // Git for Windows may omit dashed aliases for built-in commands.
            // Only these fixed subcommands are used; no SSH input becomes argv.
            GitHelper {
                program: git.clone(),
                argument: Some(name),
            }
        }
    };
    Ok((helper("upload-pack"), helper("receive-pack")))
}

async fn boundary(shared: &Shared, point: FixtureBoundary) -> Result<(), russh::Error> {
    if point == FixtureBoundary::Advertisement {
        let hold = shared.advertisement_hold.lock().unwrap().take();
        if let Some(hold) = hold {
            hold.reached
                .send(())
                .map_err(|_| russh::Error::Disconnect)?;
            let mut shutdown = shared.shutdown.clone();
            // Release is explicit. Controller unwind, fixture shutdown, or a
            // stalled controller all disconnect instead of publishing the batch.
            tokio::select! {
                released = hold.release => released.map_err(|_| russh::Error::Disconnect)?,
                _ = shutdown.changed() => return Err(russh::Error::Disconnect),
                _ = tokio::time::sleep(Duration::from_secs(20)) => return Err(russh::Error::Disconnect),
            }
        }
    }
    let fault = *shared.fault.lock().unwrap();
    match fault {
        Some(Fault::Disconnect(at)) if at == point => Err(russh::Error::Disconnect),
        Some(Fault::Stall(at, delay)) if at == point => {
            let mut shutdown = shared.shutdown.clone();
            tokio::select! { _ = tokio::time::sleep(delay) => Ok(()), _ = shutdown.changed() => Err(russh::Error::Disconnect) }
        }
        _ => Ok(()),
    }
}

pub(super) async fn serve(
    listener: tokio::net::TcpListener,
    shared: Arc<Shared>,
    ready: tokio::sync::oneshot::Sender<()>,
) {
    let mut shutdown = shared.shutdown.clone();
    let mut sessions = JoinSet::new();
    let _ = ready.send(());
    loop {
        tokio::select! {
            _ = shutdown.changed() => break,
            result = listener.accept() => {
                let Ok((socket, _)) = result else { break };
                let shared = shared.clone();
                sessions.spawn(async move {
                    if boundary(&shared, FixtureBoundary::Handshake).await.is_err() { return; }
                    let config = server::Config {
                        keys: vec![shared.host.lock().unwrap().clone()],
                        methods: (&[russh::MethodKind::PublicKey][..]).into(),
                        auth_rejection_time: Duration::ZERO,
                        auth_rejection_time_initial: Some(Duration::ZERO),
                        inactivity_timeout: None,
                        ..Default::default()
                    };
                    let handler = Restricted { shared: shared.clone(), channels: HashMap::new() };
                    let mut stop = shared.shutdown.clone();
                    let start = tokio::select! {
                        result = server::run_stream(Arc::new(config), socket, handler) => result,
                        _ = stop.changed() => return,
                    };
                    if let Ok(mut running) = start {
                        tokio::select! {
                            _ = &mut running => {},
                            _ = stop.changed() => {
                                let _ = running.handle().disconnect(russh::Disconnect::ByApplication, "fixture stopped".into(), "".into()).await;
                                let _ = tokio::time::timeout(Duration::from_secs(2), running).await;
                            }
                        }
                    }
                });
            },
            _ = sessions.join_next(), if !sessions.is_empty() => {},
        }
    }
    // Session handles close and relay tasks kill/wait for their helpers before
    // the fixture removes its repository. Runtime shutdown is the final backstop.
    while sessions.join_next().await.is_some() {}
    let tasks = std::mem::take(&mut *shared.helper_tasks.lock().unwrap());
    for task in tasks {
        let _ = task.await;
    }
}

struct Restricted {
    shared: Arc<Shared>,
    channels: HashMap<ChannelId, Channel<Msg>>,
}
impl Handler for Restricted {
    type Error = russh::Error;
    async fn auth_none(&mut self, user: &str) -> Result<Auth, Self::Error> {
        Ok(
            if user == "fixture" && self.shared.anonymous.load(Ordering::SeqCst) {
                Auth::Accept
            } else {
                Auth::reject()
            },
        )
    }
    async fn auth_publickey(
        &mut self,
        user: &str,
        key: &russh::keys::PublicKey,
    ) -> Result<Auth, Self::Error> {
        boundary(&self.shared, FixtureBoundary::Authentication).await?;
        self.shared
            .authentication_attempts
            .fetch_add(1, Ordering::SeqCst);
        if user == "fixture"
            && !self.shared.reject.load(Ordering::SeqCst)
            && key == &*self.shared.allowed.lock().unwrap()
        {
            self.shared.accepted.lock().unwrap().push(key.to_bytes()?);
            Ok(Auth::Accept)
        } else {
            self.shared
                .rejected_authentications
                .fetch_add(1, Ordering::SeqCst);
            Ok(Auth::reject())
        }
    }
    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        reply: server::ChannelOpenHandle,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        self.channels.insert(channel.id(), channel);
        reply.accept().await;
        Ok(())
    }
    async fn exec_request(
        &mut self,
        id: ChannelId,
        command: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.shared.commands.lock().unwrap().push(command.to_vec());
        let path = self.shared.command_path.lock().unwrap().clone();
        let receiving = command == format!("git-receive-pack '{path}'").as_bytes();
        let program = if receiving {
            self.shared.receive.clone()
        } else if command == format!("git-upload-pack '{path}'").as_bytes() {
            self.shared.upload.clone()
        } else {
            session.channel_failure(id)?;
            session.close(id)?;
            return Ok(());
        };
        let Some(channel) = self.channels.remove(&id) else {
            session.channel_failure(id)?;
            return Ok(());
        };
        boundary(&self.shared, FixtureBoundary::ExecAcknowledgement).await?;
        let child = tokio::process::Command::new(&program.program)
            .args(program.argument)
            .arg(&self.shared.repository)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn();
        let Ok(mut child) = child else {
            session.channel_failure(id)?;
            session.close(id)?;
            return Ok(());
        };
        if let Err(error) = session.channel_success(id) {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(error);
        }
        self.shared.helpers.fetch_add(1, Ordering::SeqCst);
        self.shared.active_helpers.fetch_add(1, Ordering::SeqCst);
        self.shared.helper_audits.lock().unwrap().0 += 1;
        let shared = self.shared.clone();
        let handle = session.handle();
        let task = tokio::spawn(async move {
            relay(channel, child, handle, shared, receiving).await;
        });
        self.shared.helper_tasks.lock().unwrap().push(task);
        Ok(())
    }
    async fn shell_request(
        &mut self,
        id: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_failure(id)
    }
    async fn env_request(
        &mut self,
        id: ChannelId,
        _: &str,
        _: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_failure(id)
    }
    async fn subsystem_request(
        &mut self,
        id: ChannelId,
        _: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_failure(id)
    }
    async fn pty_request(
        &mut self,
        id: ChannelId,
        _: &str,
        _: u32,
        _: u32,
        _: u32,
        _: u32,
        _: &[(russh::Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_failure(id)
    }
}

async fn relay(
    channel: Channel<Msg>,
    mut child: tokio::process::Child,
    handle: server::Handle,
    shared: Arc<Shared>,
    receiving: bool,
) {
    let id = channel.id();
    let mut input = child.stdin.take();
    let mut output = child.stdout.take().unwrap();
    let mut errors = child.stderr.take().unwrap();
    let mut stream = channel.into_stream();
    let mut shutdown = shared.shutdown.clone();
    let mut incoming = [0; 16_384];
    let mut outgoing = [0; 16_384];
    let mut diagnostics = [0; 4096];
    let mut output_open = true;
    let mut errors_open = true;
    let mut status = None;
    let mut transfer_started = false;
    let mut transfer_checked = false;
    let mut advertisement_checked = false;
    let mut hostile_pending = Vec::new();
    let mut hostile_sideband_sent = false;
    let mut commands = ReceiveCommands::default();
    let mut race_prefix = Vec::new();
    loop {
        if status.is_some() && !output_open {
            break;
        }
        tokio::select! {
            _ = shutdown.changed() => break,
            result = child.wait(), if status.is_none() => { status = Some(result); input.take(); },
            result = stream.read(&mut incoming), if input.is_some() => {
                match result {
                    Ok(0) | Err(_) => { input.take(); },
                    Ok(count) => {
                        transfer_started = true;
                        if receiving && commands.feed(&incoming[..count]).is_err() { break; }
                        let race_pending = receiving && shared.receive_race.lock().unwrap().is_some();
                        let payload = if race_pending {
                            race_prefix.extend_from_slice(&incoming[..count]);
                            if race_prefix.len() > 81_904 { break; }
                            if !commands.finished { continue; }
                            if commands.updates.is_empty() { race_prefix.clear(); continue; }
                            let race = shared.receive_race.lock().unwrap().take().unwrap();
                            if !apply_receive_race(&shared, &commands, race) { break; }
                            std::mem::take(&mut race_prefix)
                        } else { incoming[..count].to_vec() };
                        let written = tokio::select! {
                            result = input.as_mut().unwrap().write_all(&payload) => result,
                            _ = shutdown.changed() => break,
                        };
                        if written.is_err() { input.take(); }
                    }
                }
            },
            result = output.read(&mut outgoing), if output_open => {
                match result {
                    Ok(0) | Err(_) => output_open = false,
                    Ok(count) => {
                        if !advertisement_checked {
                            advertisement_checked = true;
                            if boundary(&shared, FixtureBoundary::Advertisement).await.is_err() { break; }
                        }
                        if transfer_started && !transfer_checked {
                            transfer_checked = true;
                            if boundary(&shared, FixtureBoundary::Transfer).await.is_err() { break; }
                        }
                        if receiving && !commands.updates.is_empty() && matches!(*shared.fault.lock().unwrap(), Some(Fault::Disconnect(FixtureBoundary::AfterReceivePack))) {
                            // Wait for the actual helper to commit its ref update, then lose
                            // its status response. This intentionally cannot imply rollback.
                            status = match tokio::time::timeout(Duration::from_secs(5), child.wait()).await {
                                Ok(result) => Some(result),
                                Err(_) => break,
                            };
                            if receive_effect_proven(&shared, &commands, &status) {
                                shared.receive_status_withheld.store(true, Ordering::SeqCst);
                                break;
                            }
                        }
                        let fault = *shared.fault.lock().unwrap();
                        if transfer_started && let Some(Fault::Pace(delay)) = fault {
                            tokio::select! { _ = tokio::time::sleep(delay) => {}, _ = shutdown.changed() => break }
                        }
                        let hostile = shared.hostile.lock().unwrap().clone();
                        let payload = if receiving && transfer_started && let Some(marker) = hostile {
                            hostile_pending.extend_from_slice(&outgoing[..count]);
                            match hostile_packets(&mut hostile_pending, marker.as_bytes(), &mut hostile_sideband_sent) { Ok(bytes) => bytes, Err(_) => break }
                        } else { outgoing[..count].to_vec() };
                        if payload.is_empty() { continue; }
                        let sent = tokio::select! {
                            result = handle.data(id, payload) => result,
                            _ = shutdown.changed() => break,
                        };
                        if sent.is_err() { break; }
                    }
                }
            },
            result = errors.read(&mut diagnostics), if errors_open => {
                // Bounded buffer, continuously drained, deliberately never printed.
                if !matches!(result, Ok(n) if n > 0) { errors_open = false; }
            }
        }
    }
    let audit_released = if receiving && receive_effect_proven(&shared, &commands, &status) {
        let hold = shared.receive_audit_hold.lock().unwrap().take();
        if let Some(hold) = hold {
            let reached = hold.events.send(super::ReceiveAuditEvent::PublicationHeld);
            reached.is_ok()
                && tokio::select! {
                    released = hold.release => released.is_ok(),
                    _ = shutdown.changed() => false,
                    _ = tokio::time::sleep(Duration::from_secs(20)) => false,
                }
        } else {
            true
        }
    } else {
        true
    };
    let code = match status {
        Some(Ok(_)) if !audit_released => 1,
        Some(Ok(status)) => status.code().unwrap_or(1) as u32,
        _ => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            1
        }
    };
    if receiving {
        if commands.updates.is_empty() {
            shared.receive_advertisements.fetch_add(1, Ordering::SeqCst);
        } else {
            let repo = git2::Repository::open_bare(&shared.repository).ok();
            let mut updates = shared.receive_updates.lock().unwrap();
            updates.extend(commands.updates.into_iter().map(|mut update| {
                update.accepted = code == 0
                    && update.old_oid != update.new_oid
                    && repo
                        .as_ref()
                        .and_then(|repo| repo.refname_to_id(&update.reference).ok())
                        == Some(update.new_oid);
                update
            }));
            shared.receive_updates_changed.notify_all();
        }
    }
    let _ = handle.exit_status_request(id, code).await;
    let _ = handle.eof(id).await;
    let _ = handle.close(id).await;
    shared.active_helpers.fetch_sub(1, Ordering::SeqCst);
    shared.completed_helpers.fetch_add(1, Ordering::SeqCst);
    shared.helper_audits.lock().unwrap().1 += 1;
    shared.helper_audits_changed.notify_all();
}

// Receive-pack report-status is pkt-line framed, possibly nested in sideband 1.
// Buffer incomplete packets so arbitrary process read boundaries cannot lose probes.
fn hostile_packets(
    pending: &mut Vec<u8>,
    marker: &[u8],
    sideband_sent: &mut bool,
) -> Result<Vec<u8>, FixtureError> {
    let mut output = Vec::new();
    while pending.len() >= 4 {
        let size = fixed(usize::from_str_radix(
            fixed(std::str::from_utf8(&pending[..4]))?,
            16,
        ))?;
        let size = if size == 0 {
            4
        } else if size < 4 {
            return Err(FixtureError);
        } else {
            size
        };
        if pending.len() < size {
            break;
        }
        let mut packet: Vec<_> = pending.drain(..size).collect();
        if packet.get(4) == Some(&1) && !*sideband_sent {
            output.extend_from_slice(format!("{:04x}", marker.len() + 6).as_bytes());
            output.push(2);
            output.extend_from_slice(marker);
            output.push(b'\n');
            *sideband_sent = true;
        }
        let needle = b"non-fast-forward";
        for index in 0..=packet.len().saturating_sub(needle.len()) {
            if packet.get(index..index + needle.len()) == Some(needle.as_slice()) {
                packet[index..index + needle.len()].copy_from_slice(marker);
            }
        }
        output.extend(packet);
    }
    Ok(output)
}

// Only the bounded command prefix is parsed. Pack bytes remain opaque and are
// forwarded in their original order without buffering or modification.
#[derive(Default)]
struct ReceiveCommands {
    pending: Vec<u8>,
    finished: bool,
    updates: Vec<ReceiveUpdate>,
}
impl ReceiveCommands {
    fn feed(&mut self, bytes: &[u8]) -> Result<(), FixtureError> {
        if self.finished {
            return Ok(());
        }
        for byte in bytes {
            self.pending.push(*byte);
            if self.pending.len() < 4 {
                continue;
            }
            let size = fixed(usize::from_str_radix(
                fixed(std::str::from_utf8(&self.pending[..4]))?,
                16,
            ))?;
            if size == 0 {
                self.finished = true;
                self.pending.clear();
                return Ok(());
            }
            if !(4..=65_520).contains(&size) {
                return Err(FixtureError);
            }
            if self.pending.len() < size {
                continue;
            }
            if self.updates.len() >= 32 {
                return Err(FixtureError);
            }
            let line = fixed(std::str::from_utf8(&self.pending[4..]))?;
            let line = line
                .split('\0')
                .next()
                .ok_or(FixtureError)?
                .trim_end_matches('\n');
            let mut fields = line.split(' ');
            let old_oid = fixed(git2::Oid::from_str(fields.next().ok_or(FixtureError)?))?;
            let new_oid = fixed(git2::Oid::from_str(fields.next().ok_or(FixtureError)?))?;
            let reference = fields.next().ok_or(FixtureError)?;
            if fields.next().is_some()
                || !reference.starts_with("refs/heads/")
                || !git2::Reference::is_valid_name(reference)
            {
                return Err(FixtureError);
            }
            self.updates.push(ReceiveUpdate {
                reference: reference.into(),
                old_oid,
                new_oid,
                accepted: false,
            });
            self.pending.clear();
        }
        Ok(())
    }
}
fn receive_effect_proven(
    shared: &Shared,
    commands: &ReceiveCommands,
    status: &Option<std::io::Result<std::process::ExitStatus>>,
) -> bool {
    status
        .as_ref()
        .is_some_and(|status| status.as_ref().is_ok_and(|status| status.success()))
        && git2::Repository::open_bare(&shared.repository).is_ok_and(|repo| {
            !commands.updates.is_empty()
                && commands.updates.iter().all(|update| {
                    update.old_oid != update.new_oid
                        && repo.refname_to_id(&update.reference).ok() == Some(update.new_oid)
                })
        })
}

pub(super) fn receiver_command_fragmentation() -> Result<(), FixtureError> {
    let old = git2::Oid::zero();
    let new = git2::Oid::from_str("1111111111111111111111111111111111111111").unwrap();
    let body = format!("{old} {new} refs/heads/main\0report-status side-band-64k\n");
    let packet = format!("{:04x}{body}0000PACKopaque-private-pack", body.len() + 4);
    for chunk in [1, 2, 3, 7, 16_384] {
        let mut parser = ReceiveCommands::default();
        for bytes in packet.as_bytes().chunks(chunk) {
            parser.feed(bytes)?;
        }
        assert!(parser.finished && parser.pending.is_empty());
        assert_eq!(
            parser.updates,
            vec![ReceiveUpdate {
                reference: "refs/heads/main".into(),
                old_oid: old,
                new_oid: new,
                accepted: false
            }]
        );
    }
    let mut advertisement_only = ReceiveCommands::default();
    advertisement_only.feed(b"0000")?;
    assert!(advertisement_only.updates.is_empty());
    let mut malformed = ReceiveCommands::default();
    assert!(malformed.feed(b"ffff").is_err());
    Ok(())
}

fn apply_receive_race(
    shared: &Shared,
    commands: &ReceiveCommands,
    (reference, expected, competing): (String, git2::Oid, git2::Oid),
) -> bool {
    if commands.updates.len() != 1
        || commands.updates[0].reference != reference
        || commands.updates[0].old_oid != expected
    {
        return false;
    }
    git2::Repository::open_bare(&shared.repository).is_ok_and(|repo| {
        repo.reference_matching(&reference, competing, true, expected, "owned receive race")
            .is_ok()
    })
}
