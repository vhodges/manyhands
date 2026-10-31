//! `comment add`, one of the two collaboration verbs that take input
//! (`conflict resolve`, in `conflict`, is the other). It is C4's driver
//! around `Mutation::CommentAdd`: the same bounded retry, the same Ctrl-C
//! window (`mutate.rs`) and the same human output (`human_collaboration`)
//! as the verbs of `collaboration`.
//!
//! The verb adds only what the request needs and the library cannot read
//! for itself: the item's kind (R-2) and the four input fields. The comment
//! ID is the caller's, so a request sent again under the same ID names the
//! same comment (design, Comment Submission).

use clap::ArgMatches;
use manyhands::{
    canonical::ItemId,
    repository::{CommentAddInput, Mutation, RepositoryService},
};

use crate::{
    admin, authoring,
    catalog::CommandSpec,
    collaboration, fill,
    input::{InputError, InputObject, Tristate},
    mutate::Session,
    render::Output,
};

/// The input of `comment add`. It holds the comment's body, so it derives
/// nothing that could print it.
struct Fields {
    comment_id: ItemId,
    body: String,
    parent_id: Option<ItemId>,
    created_by: Option<String>,
}

/// Reads the four fields. `parent_id` is a root comment when it is absent
/// or null. `created_by` is absent for the Git identity's text, and the
/// library holds the rule for what it may say (R-7).
fn fields(object: &InputObject) -> Result<Fields, InputError> {
    object.only(&["comment_id", "body", "parent_id", "created_by"])?;
    Ok(Fields {
        comment_id: authoring::ulid(object.string("comment_id")?, "comment_id")?,
        body: object.string("body")?,
        parent_id: match object.tristate("parent_id")? {
            Tristate::Missing | Tristate::Null => None,
            Tristate::Value(text) => Some(authoring::ulid(text, "parent_id")?),
        },
        created_by: object.optional_string("created_by")?,
    })
}

pub fn comment_add(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let (flags, root, id, kind, fields) = match (|| {
        let flags = admin::flags(session, spec, args)?;
        let object = admin::input_object(session, spec, args, flags.request_id)?;
        let fields = fields(&object)
            .map_err(|e| admin::input_failure(session, spec, flags.request_id, e))?;
        let root = admin::mutation_root(session, spec, args, flags.request_id)?;
        let id = authoring::item_id(session, spec, args, flags.request_id)?;
        // R-2: C3's fill read gives the item's kind before the request is
        // built, and its refusal is the verb's.
        let (_, item) = fill::read_any_item(service, session, spec, &root, &id, flags.request_id)?;
        Ok((
            flags,
            root,
            id,
            collaboration::authoring_kind(item.kind),
            fields,
        ))
    })() {
        Ok(values) => values,
        Err(out) => return out,
    };
    collaboration::run(service, session, spec, flags, None, &mut |_| {
        Mutation::CommentAdd(CommentAddInput {
            root: root.clone(),
            kind,
            id: id.clone(),
            comment_id: fields.comment_id.clone(),
            parent_id: fields.parent_id.clone(),
            body: fields.body.clone(),
            created_by: fields.created_by.clone(),
        })
    })
}

#[cfg(test)]
#[path = "discussion_tests.rs"]
mod tests;
