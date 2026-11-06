//! Frozen catalogs extracted before D15's first DDL change, at d801d4ab.
//! These definitions are independent of the current schema builder. The three
//! published fingerprints also match their independently pinned original SHAs.
use super::super::{RepositoryError, remote::state::recovery_required};
use std::collections::BTreeMap;

pub(super) const GAP_MARKER: &str = "context_retirement_gap_v1";
pub(super) const GAP_TABLE: &str = "retirement_batch_gap_attempts";
pub(super) type Objects = Vec<(&'static str, &'static str, String)>;

pub(super) fn gap_object(name: &str) -> bool {
    name == GAP_TABLE || name.starts_with("retirement_gap_")
}

#[derive(serde::Deserialize)]
struct Frozen<'a> {
    #[serde(borrow)]
    objects: Vec<(&'a str, &'a str, String)>,
    orders: BTreeMap<String, Vec<usize>>,
}

pub(super) fn catalog(version: &str) -> Result<Objects, RepositoryError> {
    let frozen: Frozen<'static> = serde_json::from_str(include_str!("historical_catalogs.json"))
        .map_err(|_| recovery_required())?;
    let objects: Objects = frozen
        .orders
        .get(version)
        .ok_or_else(recovery_required)?
        .iter()
        .map(|&i| {
            let (kind, name, sql) = frozen.objects.get(i).ok_or_else(recovery_required)?;
            Ok((*kind, *name, sql.clone()))
        })
        .collect::<Result<_, RepositoryError>>()?;
    let expected = match version {
        "v1" => "b8ac7ae7201e6182a91d1a8546e63ad518dc9701",
        "v2" => "7f82f75dbd4ea313b8e291c0849a1b7dc763fe4a",
        "index" => "c8da3934cfa9085a681420ef19097d5dc01fd42e",
        "gap" => "754b4df04a2d604c88e596e627daba7da840a8aa",
        _ => return Err(recovery_required()),
    };
    let mut bytes = Vec::new();
    for (kind, name, sql) in &objects {
        for field in [*kind, *name, &super::record::normalized(sql)] {
            bytes.extend_from_slice(field.as_bytes());
            bytes.push(0);
        }
    }
    if git2::Oid::hash_object(git2::ObjectType::Blob, &bytes)
        .map_err(|_| recovery_required())?
        .to_string()
        != expected
    {
        return Err(recovery_required());
    }
    Ok(objects)
}
