//! Restricted SSH sessions and owned, shell-free Git helper processes.
use super::{Fault, FixtureBoundary, FixtureError, Shared, fixed};
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
) -> Result<(PathBuf, PathBuf), FixtureError> {
    let mut command = Command::new("git");
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
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    let upload = directory.join(format!("git-upload-pack{suffix}"));
    let receive = directory.join(format!("git-receive-pack{suffix}"));
    if !upload.is_file() || !receive.is_file() {
        return Err(FixtureError);
    }
    Ok((upload, receive))
}

async fn boundary(shared: &Shared, point: FixtureBoundary) -> Result<(), russh::Error> {
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
        if user == "fixture"
            && !self.shared.reject.load(Ordering::SeqCst)
            && key == &*self.shared.allowed.lock().unwrap()
        {
            self.shared.accepted.lock().unwrap().push(key.to_bytes()?);
            Ok(Auth::Accept)
        } else {
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
        let program = match command {
            b"git-upload-pack '/fixture.git'" => self.shared.upload.clone(),
            b"git-receive-pack '/fixture.git'" => self.shared.receive.clone(),
            _ => {
                session.channel_failure(id)?;
                session.close(id)?;
                return Ok(());
            }
        };
        let Some(channel) = self.channels.remove(&id) else {
            session.channel_failure(id)?;
            return Ok(());
        };
        boundary(&self.shared, FixtureBoundary::ExecAcknowledgement).await?;
        let child = tokio::process::Command::new(program)
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
        let shared = self.shared.clone();
        let handle = session.handle();
        let task = tokio::spawn(async move {
            relay(channel, child, handle, shared).await;
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
                        let written = tokio::select! {
                            result = input.as_mut().unwrap().write_all(&incoming[..count]) => result,
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
                        let fault = *shared.fault.lock().unwrap();
                        if transfer_started && let Some(Fault::Pace(delay)) = fault {
                            tokio::select! { _ = tokio::time::sleep(delay) => {}, _ = shutdown.changed() => break }
                        }
                        let sent = tokio::select! {
                            result = handle.data(id, outgoing[..count].to_vec()) => result,
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
    let code = match status {
        Some(Ok(status)) => status.code().unwrap_or(1) as u32,
        _ => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            1
        }
    };
    let _ = handle.exit_status_request(id, code).await;
    let _ = handle.eof(id).await;
    let _ = handle.close(id).await;
    shared.active_helpers.fetch_sub(1, Ordering::SeqCst);
    shared.completed_helpers.fetch_add(1, Ordering::SeqCst);
}
