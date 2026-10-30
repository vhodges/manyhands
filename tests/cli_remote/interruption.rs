//! Ctrl-C through a pseudo-terminal, against a held transfer (decision 14).
//! Task 7's spike records what a press during a blocked network call
//! answers; Task 13 adds the other interruption cases.
use super::*;

pub const CASES: &[ssh_harness::Case] = &[
    #[cfg(unix)]
    (
        "c4_interruption_held_transfer",
        unix::c4_interruption_held_transfer,
    ),
];

#[cfg(unix)]
mod unix {
    use super::*;
    use remote_world::{CONTEXT, ITEM, PASSWORD, World};
    use repository::{keys::*, transport::*};
    use ssh_remote::{FixtureError, fixed};
    use support::pty::PtyCli;

    /// Bytes the fixture forwards before it holds the fetch's transfer.
    const HELD_AFTER: usize = 131_072;

    /// Pins the host as `host approve` would.
    fn approve(w: &World) -> Result<(), FixtureError> {
        fixed(w.service.verify_ssh_transport(
            VerifySshTransportRequest {
                root: w.root.clone(),
                direction: SshDirection::Fetch,
                approval: Some(HostApproval {
                    authority: SshAuthority {
                        host: "127.0.0.1".into(),
                        port: w.server.address().port(),
                    },
                    expected: None,
                    presented: w.server.host_identity(),
                }),
            },
            &mut SessionCredentials::new(remote_world::Provider),
        ))?;
        Ok(())
    }

    /// Selects an unprotected copy of the world's key, so that a
    /// non-interactive call needs no unlock. The server accepts it: it is
    /// the same key.
    fn select_an_unprotected_key(w: &World) -> Result<(), FixtureError> {
        let key = fixed(
            fixed(ssh_key::PrivateKey::read_openssh_file(
                w.server.client_key_path(),
            ))?
            .decrypt(PASSWORD),
        )?;
        let text = fixed(key.to_openssh(ssh_key::LineEnding::LF))?;
        OUTPUT_PROBES.with(|probes| {
            probes.borrow_mut().extend(
                text.lines()
                    .filter(|line| line.len() > 40)
                    .map(str::to_owned),
            );
        });
        let path = w.directory.path().join("unprotected-key");
        fixed(std::fs::write(&path, text.as_bytes()))?;
        use std::os::unix::fs::PermissionsExt;
        fixed(std::fs::set_permissions(
            &path,
            std::fs::Permissions::from_mode(0o600),
        ))?;
        let RegisterSharedKeyOutcome::Registered(key) =
            fixed(w.service.register_shared_key(RegisterSharedKeyRequest {
                label: "unprotected".into(),
                ownership: SharedKeyOwnership::Imported,
                private_key_path: path,
                public_key_path: None,
            }))?
        else {
            return Err(FixtureError);
        };
        fixed(w.service.select_shared_key(key.id))?;
        Ok(())
    }

    /// A peer moves the remote primary on with a commit large enough that
    /// a fetch of it is still transferring when the fixture holds it.
    fn incoming(w: &World) -> Result<git2::Oid, FixtureError> {
        let mut state = 41u64;
        let payload = (0..1_600_000)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state as u8
            })
            .collect::<Vec<_>>();
        let base = fixed(w.peer.refname_to_id("refs/heads/main"))?;
        let oid = remote_world::commit(&w.peer, base, "payload", &payload)?;
        fixed(w.peer.reference("refs/heads/main", oid, true, "incoming"))?;
        remote_world::push_peer(&w.peer, &w.server, "refs/heads/main")?;
        Ok(oid)
    }

    fn index_of<T: Copy + PartialEq>(all: &[T], value: T) -> u128 {
        all.iter().position(|known| *known == value).unwrap() as u128
    }

    /// Decision 14's spike: `item sync` is pressed with Ctrl-C while its
    /// fetch is held. Decision 14 accepts `cancelled`, exit 130, at the next
    /// safe point, or `transport_unavailable`, exit 5, if a blocking call
    /// returned `EINTR`, either with its effects intact. The spike recorded
    /// `cancelled` (Task 7's ledger entry), which this case pins. It also
    /// records the exit, the code and the effects as an observation.
    pub fn c4_interruption_held_transfer() -> Result<(), FixtureError> {
        let mut w = World::new()?;
        private_fixture(&w.server);
        let env = CliEnv::new(w.data());
        approve(&w)?;
        select_an_unprotected_key(&w)?;
        w.context()?;
        let repo = w.repo()?;
        let context = fixed(repo.refname_to_id(CONTEXT))?;
        let primary = fixed(repo.refname_to_id("refs/heads/main"))?;
        let incoming = incoming(&w)?;
        let pushes = w.server.receive_updates().len();

        let hold = w.server.hold_transfer_after(HELD_AFTER)?;
        let request = repository::RequestId::new().to_string();
        let root = w.root.to_str().unwrap().to_owned();
        let mut pty = PtyCli::spawn(
            &[
                "item",
                "sync",
                "--repo",
                &root,
                "--id",
                ITEM,
                "--non-interactive",
                "--request-id",
                &request,
            ]
            .iter()
            .map(OsStr::new)
            .collect::<Vec<_>>(),
            &env,
            &[],
        );
        hold.wait_until_held()?;
        assert!(pty.first_stderr_line().starts_with("progress: "));
        pty.send(&[3]);
        // The press is delivered while the transfer is still held.
        std::thread::sleep(std::time::Duration::from_millis(500));
        hold.release()?;
        let (out, _) = pty.finish();
        scan_output(&out);
        assert!(out.stdout.is_empty());
        let stderr = String::from_utf8(out.stderr.clone()).unwrap();
        assert!(
            !stderr.contains("progress: retrying"),
            "no retry follows a press"
        );

        // The stored result and what the remote and the clone hold.
        let stored = checked(
            &env,
            &["request", "show", "--request-id", &request],
            &[],
            "ok",
            0,
            Some("request.schema.json"),
        );
        let result = &stored["data"]["result"];
        let code = result["code"].as_str().unwrap().to_owned();
        assert!(stderr.contains(&format!("code: {code}\n")), "{stderr}");
        let bare = w.bare()?;
        let remote_primary = fixed(bare.refname_to_id("refs/heads/main"))?;
        let remote_context = bare.refname_to_id(CONTEXT).is_ok();
        let repo = w.repo()?;
        let local_context = fixed(repo.refname_to_id(CONTEXT))?;
        let local_primary = fixed(repo.refname_to_id("refs/heads/main"))?;
        let effect = |key: &str| result["effects"][key].as_str().unwrap().to_owned();
        ssh_harness::observation(&[
            out.status as u128,
            index_of(
                &manyhands::results::ResultCode::ALL.map(|code| code.as_str()),
                code.as_str(),
            ),
            index_of(
                &manyhands::results::PublicationEffect::ALL.map(|effect| effect.as_str()),
                effect("publication").as_str(),
            ),
            index_of(
                &manyhands::results::IntegrationEffect::ALL.map(|effect| effect.as_str()),
                effect("integration").as_str(),
            ),
            u128::from(remote_context),
            u128::from(local_context == context),
            u128::from(local_primary == primary),
            u128::from(remote_primary == incoming),
            (w.server.receive_updates().len() - pushes) as u128,
        ]);

        // The press cancelled the request at the next safe point: the
        // operation is final and its result is stored.
        assert_eq!((out.status, code.as_str()), (130, "cancelled"));
        assert_eq!(result["outcome"], "cancelled");
        assert!(stderr.contains("progress: stopping\n"), "{stderr}");
        assert_eq!(stored["data"]["state"], "finished");
        // Its effects are intact: nothing was pushed, merged or moved
        // locally, and the effects say exactly that.
        assert_eq!(w.server.receive_updates().len(), pushes, "nothing pushed");
        assert!(!remote_context, "the context was not published");
        assert_eq!(remote_primary, incoming);
        assert_eq!(local_context, context, "nothing merged into the context");
        assert_eq!(local_primary, primary);
        for (key, value) in [
            ("write", "not_requested"),
            ("checkpoint", "not_requested"),
            ("discovery", "not_requested"),
            ("publication", "pending"),
            ("integration", "not_requested"),
            ("cleanup", "not_requested"),
        ] {
            assert_eq!(effect(key), value, "{key}");
        }
        assert!(result["effects"]["commit_oid"].is_null());
        assert!(
            stderr.contains("effects: publication=pending\n"),
            "{stderr}"
        );
        Ok(())
    }
}
