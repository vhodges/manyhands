//! Windows namespace/lifecycle half of the shared resolution proof protocol.
//! Fixed private roles are disposable only before their immutable provenance is
//! journaled. No Drop cleanup, foreign-lock adoption, or backend-log repair.
use super::*;
use native_resolution::{Directory, Image};

fn external<T>(value: std::io::Result<T>) -> Result<T, SynchronizationError> {
    value.map_err(|_| SynchronizationError::ExternalChange)
}

fn recovery<T>(value: std::io::Result<T>) -> Result<T, SynchronizationError> {
    value.map_err(|_| SynchronizationError::RecoveryRequired)
}

fn required(directory: &Directory, name: &str) -> Result<Image, SynchronizationError> {
    external(directory.image(name))?.ok_or(SynchronizationError::ExternalChange)
}

fn observed(image: &Image) -> IndexFileImage {
    IndexFileImage {
        bytes: image.bytes.clone(),
        device: image.stamp.identity[0],
        inode: image.stamp.identity[1],
    }
}

fn exact(image: &Image, expected: &IndexFileImage) -> bool {
    index_image_is_exact(&observed(image), expected)
}

fn output_matches(image: &Image, output: (u64, u64, [u8; 32])) -> bool {
    image.stamp.identity == [output.0, output.1]
        && *blake3::hash(&image.bytes).as_bytes() == output.2
}

fn private_index(directory: &Directory, name: &str) -> Result<git2::Index, SynchronizationError> {
    let image = required(directory, name)?;
    external(directory.matches(name, &image))?;
    let index = git2::Index::open(directory.path().join(name))
        .map_err(|_| SynchronizationError::ExternalChange)?;
    external(directory.matches(name, &image))?;
    Ok(index)
}

/// Only unjournaled, deterministic private preparation may be rebuilt. Every
/// leaf still needs a fresh no-reparse regular-file proof before retirement.
fn discard_preparation(directory: &Directory, name: &str) -> Result<(), SynchronizationError> {
    if let Some(image) = external(directory.image(name))? {
        external(directory.retire(name, image))?;
    }
    Ok(())
}

pub(super) fn ref_root_identity(path: &Path) -> Result<[u64; 2], SynchronizationError> {
    recovery(Directory::open(path).and_then(|directory| directory.identity()))
}

pub(super) fn ref_log_stamp(file: &std::fs::File) -> Result<RefLogStamp, SynchronizationError> {
    let stamp = recovery(native_resolution::stamp(file))?;
    Ok((
        stamp.identity[0],
        stamp.identity[1],
        stamp.size,
        stamp.attributes,
        stamp.created,
        stamp.modified,
        stamp.changed,
        0,
    ))
}

pub(super) fn open_ref_log_file(
    root: &Path,
    role: &Path,
) -> Result<Option<std::fs::File>, SynchronizationError> {
    let (parent, name) = match native_resolution::owned_parent(root, role, false) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(SynchronizationError::RecoveryRequired),
    };
    recovery(parent.file(&name))
}

/// Retain all existing ancestors across streaming log hashing and the short
/// apply lease. If the log parent is absent, pin the nearest existing ancestor;
/// revalidation through open_ref_log_file will still prove leaf absence.
pub(super) fn observation_namespace(
    root: &Path,
    role: &Path,
) -> Result<Directory, SynchronizationError> {
    let mut directory = recovery(Directory::open(root))?;
    for component in role
        .parent()
        .ok_or(SynchronizationError::RecoveryRequired)?
        .components()
    {
        let std::path::Component::Normal(name) = component else {
            return Err(SynchronizationError::RecoveryRequired);
        };
        let name = name
            .to_str()
            .ok_or(SynchronizationError::RecoveryRequired)?;
        match directory.child(name, false) {
            Ok(child) => directory = child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(_) => return Err(SynchronizationError::RecoveryRequired),
        }
    }
    Ok(directory)
}

pub(super) fn sync_git_role(root: &Path, role: &Path) -> Result<(), SynchronizationError> {
    let (parent, name) = match native_resolution::owned_parent(root, role, false) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(SynchronizationError::RecoveryRequired),
    };
    recovery(parent.flush(&name))
}

pub(super) fn refuse_ambiguous_resolution_backend_locks(
    repository: &git2::Repository,
) -> Result<(), SynchronizationError> {
    let head = repository
        .find_reference("HEAD")
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let branch = head
        .symbolic_target()
        .ok_or(SynchronizationError::RecoveryRequired)?;
    if !branch.starts_with("refs/heads/") || !git2::Reference::is_valid_name(branch) {
        return Err(SynchronizationError::RecoveryRequired);
    }
    for (root, roles) in [
        (
            repository.path(),
            vec![PathBuf::from("HEAD.lock"), PathBuf::from("logs/HEAD.lock")],
        ),
        (
            repository.commondir(),
            vec![
                PathBuf::from("packed-refs.lock"),
                PathBuf::from(format!("{branch}.lock")),
                PathBuf::from(format!("logs/{branch}.lock")),
            ],
        ),
    ] {
        for role in roles {
            // An unsafe leaf/ancestor also refuses. Never read a public reflog
            // through libgit2 (its read API may create absent logs).
            if open_ref_log_file(root, &role)?.is_some() {
                return Err(SynchronizationError::RecoveryRequired);
            }
        }
    }
    Ok(())
}

pub(super) struct ResolutionIndexLock {
    parent: Directory,
    staging: Directory,
    baseline: IndexFileImage,
    sentinel: IndexFileImage,
    pub(super) artifact: state::ResolutionIndexArtifact,
    #[cfg(test)]
    workdir: PathBuf,
}

impl ResolutionIndexLock {
    pub(super) fn acquire(
        repository: &git2::Repository,
        service: &RepositoryService,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
    ) -> Result<Self, SynchronizationError> {
        let parent = external(Directory::open(repository.path()))?;
        let retained = state::with_transaction(service, root, |tx, id| {
            let record = state::read_operation(tx, id, owner.operation_id())?
                .ok_or_else(state::recovery_required)?;
            state::resolution_index_artifact(tx, &record, attempt)
        })?;
        if retained.is_none() && external(parent.image("index.lock"))?.is_some() {
            return Err(SynchronizationError::ExternalChange);
        }
        let staging = external(parent.child(
            &format!(".manyhands-resolution-{attempt}"),
            retained.is_none(),
        ))?;
        if external(parent.identity())?[0] != external(staging.identity())?[0] {
            return Err(SynchronizationError::ExternalChange);
        }
        let artifact = if let Some(artifact) = retained {
            artifact
        } else {
            // A crashed unjournaled baseline may still link the live index. Do
            // not retain another handle to that inode across its verified delete.
            discard_preparation(&staging, "baseline")?;
            let baseline = required(&parent, "index")?;
            approved_index_extensions(&baseline.bytes)?;
            // The baseline anchor retains the ORIGINAL index identity as well as
            // its bytes; the canonical index is never written through this link.
            external(parent.publish_anchor("index", &staging, "baseline", &baseline))?;
            private_index(&staging, "baseline")?;
            discard_preparation(&staging, "sentinel")?;
            let sentinel = external(
                staging.create_image(
                    "sentinel",
                    format!(
                        "manyhands-resolution-sentinel-v2\n{}\n{attempt}\n",
                        owner.operation_id()
                    )
                    .as_bytes(),
                ),
            )?;
            let mut metadata = [None; 3];
            for (ordinal, name) in RESOLUTION_MERGE_MEMBERS.iter().enumerate() {
                metadata[ordinal] = external(parent.image(name))?
                    .map(|image| *blake3::hash(&image.bytes).as_bytes());
            }
            let artifact = state::ResolutionIndexArtifact {
                device: sentinel.stamp.identity[0],
                inode: sentinel.stamp.identity[1],
                sentinel_digest: *blake3::hash(&sentinel.bytes).as_bytes(),
                baseline_digest: *blake3::hash(&baseline.bytes).as_bytes(),
                baseline_identity: (baseline.stamp.identity[0], baseline.stamp.identity[1]),
                metadata,
                output: None,
                ref_phase: "not_started".into(),
                phase: "intent".into(),
            };
            // Immutable ownership is durable BEFORE absent-only public linking.
            service.prepare_synchronization_index_artifact(root, owner, attempt, &artifact)?;
            artifact
        };
        let baseline = required(&staging, "baseline")?;
        let sentinel = required(&staging, "sentinel")?;
        if *blake3::hash(&baseline.bytes).as_bytes() != artifact.baseline_digest
            || baseline.stamp.identity
                != [artifact.baseline_identity.0, artifact.baseline_identity.1]
            || sentinel.stamp.identity != [artifact.device, artifact.inode]
            || *blake3::hash(&sentinel.bytes).as_bytes() != artifact.sentinel_digest
        {
            return Err(SynchronizationError::ExternalChange);
        }
        let mut held = Self {
            baseline: observed(&baseline),
            sentinel: observed(&sentinel),
            parent,
            staging,
            artifact,
            #[cfg(test)]
            workdir: repository
                .workdir()
                .ok_or(SynchronizationError::ExternalChange)?
                .to_owned(),
        };
        if held.artifact.output.is_none() {
            held.original_index_matches()?;
        }
        if held.artifact.phase == "intent" {
            if external(held.parent.image("index.lock"))?.is_none() {
                let image = required(&held.staging, "sentinel")?;
                external(held.staging.publish_anchor(
                    "sentinel",
                    &held.parent,
                    "index.lock",
                    &image,
                ))?;
            }
            held.verify_sentinel()?;
            #[cfg(test)]
            run_resolution_index_lock_hook(&held.workdir);
            held.verify_sentinel()?;
            held.original_index_matches()?;
            service.advance_synchronization_index_artifact(root, owner, attempt, "published")?;
            held.artifact.phase = "published".into();
        } else if held.artifact.phase == "published"
            || external(held.parent.image("index.lock"))?.is_some()
        {
            if held.artifact.phase == "released" {
                return Err(SynchronizationError::ExternalChange);
            }
            held.verify_sentinel()?;
        }
        Ok(held)
    }

    fn verify_namespace(&self) -> Result<(), SynchronizationError> {
        external(self.parent.revalidate())?;
        external(self.staging.revalidate())
    }

    fn original_index_matches(&self) -> Result<(), SynchronizationError> {
        self.verify_namespace()?;
        if !exact(&required(&self.parent, "index")?, &self.baseline) {
            return Err(SynchronizationError::ExternalChange);
        }
        Ok(())
    }

    fn verify_sentinel(&self) -> Result<(), SynchronizationError> {
        self.verify_namespace()?;
        if !exact(&required(&self.staging, "sentinel")?, &self.sentinel)
            || !exact(&required(&self.parent, "index.lock")?, &self.sentinel)
        {
            return Err(SynchronizationError::ExternalChange);
        }
        Ok(())
    }

    pub(super) fn authoritative_index(&self) -> Result<git2::Index, SynchronizationError> {
        self.verify_sentinel()?;
        if self.artifact.output.is_some() {
            return Err(SynchronizationError::RecoveryRequired);
        }
        discard_preparation(&self.staging, "index.lock")?;
        discard_preparation(&self.staging, "index")?;
        external(self.staging.create_image("index", &self.baseline.bytes))?;
        private_index(&self.staging, "index")
    }

    pub(super) fn persisted_images_match(&self) -> Result<(), SynchronizationError> {
        self.verify_sentinel()?;
        self.installed_image_matches()
    }

    pub(super) fn installed_image_matches(&self) -> Result<(), SynchronizationError> {
        self.verify_namespace()?;
        let output = self
            .artifact
            .output
            .ok_or(SynchronizationError::RecoveryRequired)?;
        let installed = required(&self.parent, "index")?;
        let anchored = required(&self.staging, "index")?;
        if !output_matches(&installed, output)
            || !output_matches(&anchored, output)
            || installed.bytes != anchored.bytes
        {
            return Err(SynchronizationError::ExternalChange);
        }
        Ok(())
    }

    pub(super) fn persist(
        &mut self,
        source: &mut git2::Index,
        repository: &git2::Repository,
        service: &RepositoryService,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
    ) -> Result<(), SynchronizationError> {
        self.verify_sentinel()?;
        self.original_index_matches()?;
        if source.path() != Some(self.staging.path().join("index").as_path()) {
            return Err(SynchronizationError::ExternalChange);
        }
        self.verify_namespace()?;
        source
            .write()
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        self.verify_namespace()?;
        external(self.staging.flush("index"))?;
        let prepared = required(&self.staging, "index")?;
        #[cfg(test)]
        run_resolution_index_scratch_hook(
            repository
                .workdir()
                .ok_or(SynchronizationError::ExternalChange)?,
        );
        external(self.staging.matches("index", &prepared))?;
        let parsed = private_index(&self.staging, "index")?;
        if !index_entries_match(&parsed, source) || parsed.has_conflicts() {
            return Err(SynchronizationError::ExternalChange);
        }
        external(self.staging.matches("index", &prepared))?;
        let output = (
            prepared.stamp.identity[0],
            prepared.stamp.identity[1],
            *blake3::hash(&prepared.bytes).as_bytes(),
        );
        service.prepare_synchronization_index_output(root, owner, attempt, output)?;
        self.artifact.output = Some(output);
        #[cfg(test)]
        run_resolution_index_persist_hook(
            repository
                .workdir()
                .ok_or(SynchronizationError::ExternalChange)?,
        );
        self.install_prepared()?;
        #[cfg(test)]
        run_resolution_index_install_hook(
            repository
                .workdir()
                .ok_or(SynchronizationError::ExternalChange)?,
        );
        let _ = repository;
        self.persisted_images_match()
    }

    pub(super) fn install_prepared(&self) -> Result<(), SynchronizationError> {
        self.verify_sentinel()?;
        let output = self
            .artifact
            .output
            .ok_or(SynchronizationError::RecoveryRequired)?;
        let current = required(&self.parent, "index")?;
        let prepared = required(&self.staging, "index")?;
        if !output_matches(&prepared, output) {
            return Err(SynchronizationError::ExternalChange);
        }
        if output_matches(&current, output) {
            return self.installed_image_matches();
        }
        if !exact(&current, &self.baseline) {
            return Err(SynchronizationError::ExternalChange);
        }
        match external(self.staging.image("install"))? {
            Some(image) if !output_matches(&image, output) || image.bytes != prepared.bytes => {
                return Err(SynchronizationError::ExternalChange);
            }
            Some(_) => {}
            None => {
                external(
                    self.staging
                        .publish_anchor("index", &self.staging, "install", &prepared),
                )?
            }
        }
        // Link creation refreshes ChangeTime: fresh retained handle proof must
        // still agree with the immutable journal identity and bytes.
        let install = required(&self.staging, "install")?;
        if !output_matches(&install, output) || install.bytes != prepared.bytes {
            return Err(SynchronizationError::ExternalChange);
        }
        let original = required(&self.parent, "index")?;
        if !exact(&original, &self.baseline) {
            return Err(SynchronizationError::ExternalChange);
        }
        external(self.staging.install_anchor(
            "install",
            &self.parent,
            "index",
            &install,
            &original,
        ))?;
        self.persisted_images_match()
    }

    pub(super) fn metadata_matches(&self, allow_absent: bool) -> Result<(), SynchronizationError> {
        self.verify_namespace()?;
        for (ordinal, name) in RESOLUTION_MERGE_MEMBERS.iter().enumerate() {
            match external(self.parent.image(name))? {
                Some(image)
                    if self.artifact.metadata[ordinal]
                        != Some(*blake3::hash(&image.bytes).as_bytes()) =>
                {
                    return Err(SynchronizationError::ExternalChange);
                }
                None if !allow_absent && self.artifact.metadata[ordinal].is_some() => {
                    return Err(SynchronizationError::ExternalChange);
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(super) fn retire_metadata(&self) -> Result<(), SynchronizationError> {
        self.persisted_images_match()?;
        self.metadata_matches(true)?;
        for name in RESOLUTION_MERGE_MEMBERS {
            self.metadata_matches(true)?;
            if let Some(image) = external(self.parent.image(name))? {
                external(self.parent.retire(name, image))?;
            }
        }
        Ok(())
    }

    pub(super) fn retire(
        &mut self,
        service: &RepositoryService,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
    ) -> Result<(), SynchronizationError> {
        if self.artifact.phase == "published" {
            self.persisted_images_match()?;
            service.advance_synchronization_index_artifact(
                root,
                owner,
                attempt,
                "release_intent",
            )?;
            self.artifact.phase = "release_intent".into();
        }
        #[cfg(test)]
        run_resolution_index_retire_hook(&self.workdir);
        self.verify_namespace()?;
        if let Some(image) = external(self.parent.image("index.lock"))? {
            if self.artifact.phase == "released" {
                return Err(SynchronizationError::ExternalChange);
            }
            self.verify_sentinel()?;
            // This image handle is consumed and closed inside retire; no other
            // retained leaf handle can leave the target merely delete-pending.
            external(self.parent.retire("index.lock", image))?;
        }
        if external(self.parent.image("index.lock"))?.is_some() {
            return Err(SynchronizationError::ExternalChange);
        }
        service.advance_synchronization_index_artifact(root, owner, attempt, "released")?;
        self.artifact.phase = "released".into();
        Ok(())
    }

    pub(super) fn ref_manifest(
        &self,
        service: &RepositoryService,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
        role: &str,
    ) -> Result<Option<(RefLogManifest, state::ResolutionRefLogArtifact)>, SynchronizationError>
    {
        self.verify_namespace()?;
        let artifact = state::with_transaction(service, root, |tx, id| {
            let record = state::read_operation(tx, id, owner.operation_id())?
                .ok_or_else(state::recovery_required)?;
            state::resolution_ref_log_artifact(tx, &record, attempt, role)
        })?;
        let Some(artifact) = artifact else {
            return Ok(None);
        };
        let name = format!("ref-log-{role}");
        let image =
            required(&self.staging, &name).map_err(|_| SynchronizationError::RecoveryRequired)?;
        let anchor = required(&self.staging, &format!("{name}-anchor"))
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        if image.bytes.len() > 16384
            || image.stamp.identity != [artifact.device, artifact.inode]
            || image.stamp.identity != anchor.stamp.identity
            || image.bytes != anchor.bytes
            || *blake3::hash(&image.bytes).as_bytes() != artifact.digest
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        let manifest = serde_yaml::from_slice(&image.bytes)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        Ok(Some((manifest, artifact)))
    }

    pub(super) fn persist_ref_manifest(
        &self,
        service: &RepositoryService,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
        role: &str,
        manifest: &RefLogManifest,
    ) -> Result<(), SynchronizationError> {
        if let Some((existing, _)) = self.ref_manifest(service, root, owner, attempt, role)? {
            return if existing == *manifest {
                Ok(())
            } else {
                Err(SynchronizationError::RecoveryRequired)
            };
        }
        let name = format!("ref-log-{role}");
        let anchor = format!("{name}-anchor");
        discard_preparation(&self.staging, &name)?;
        discard_preparation(&self.staging, &anchor)?;
        let bytes = serde_yaml::to_string(manifest)
            .map_err(|_| SynchronizationError::RecoveryRequired)?
            .into_bytes();
        if bytes.len() > 16384 {
            return Err(SynchronizationError::RecoveryRequired);
        }
        let image = external(self.staging.create_image(&name, &bytes))?;
        external(
            self.staging
                .publish_anchor(&name, &self.staging, &anchor, &image),
        )?;
        service.prepare_synchronization_ref_log_artifact(
            root,
            owner,
            attempt,
            role,
            &state::ResolutionRefLogArtifact {
                device: image.stamp.identity[0],
                inode: image.stamp.identity[1],
                digest: *blake3::hash(&bytes).as_bytes(),
            },
        )?;
        self.ref_manifest(service, root, owner, attempt, role)?
            .ok_or(SynchronizationError::RecoveryRequired)?;
        Ok(())
    }
}
