---
title: "Project Charter and other initial ramblings"
date: 2026-09-26
draft: false
tags:
  - What
  - Why
  - initial design
  - initial requirements
author: "Vince Hodges <vhodges@gmail.com>"
manyhands_managed: true
---
# Introduction

Manyhands is a local first, Git backed, project management and documentation tool similar to tools like Jira and Confluence.

With the rise in agentic workflows and development methodologies, it's ideal to keep documentation and state as close to 
the source code as possible, ideally in the same repository.  This would tend to require some form of version control for sharing and coordination eg Git. 

And while Git is not generally an issue for agents and developers, it tends to be a more awkward experience for 
non-developers such as QA Analysts, Product Managers and Subject Matter Experts (SMEs).  Manyhands is my experiment in 
making it easier for everyone.

## Nomenclature

A brief description of some of the main concepts in the application.

### Tickets

A ticket is a markdown file with 'ticket like' fields in the front matter section.  Tickets have type, state|status, title, short description?, datestamps.  Git history probably provides: author, datestamps, etc  Types and Status can be configured on a per repo basis.

### Managed Documents

Similarly, a managed document is a markdown file with front matter located in a Docs or docs folder at the root of the repository.  The only manadatory front matter to be considered a manyhands document is a field: manyhands_managed: true (for example this document is one).

Documents can live in subfolders of the root docs folder, perhaps organized by document type, but this is up to each project and repo.

### Teams and Projects

Tickets and documents may have team and project fields.  This can be eventually used to group items, allowing for rolled up statuses etc, but this will be a future round of design and development (but maybe take it into account now)

### Skills

We'll provide a set of skills not only for interacting with manyhands but also authoring tickets, documents, asking questions,
etc.  The authoring skills be template driven, so they are adaptable to your local conventions and structures, workflows, etc

### Desktop

There will be a locally run desktop application, written in Rust and pre-built binaries for Windows, MacOS and 
Linux (Wayland). The BSDs and other platforms can build from source and/or are on their own (depends on GPUI support).

### Cli

There will be a command line version of the application for the same targets, to be used agents, ci, etc.

## High Level Requirements/features/etc

- The presence of a .manyhands directory at the root of the repo is enough to indicate the repo is manyhands enabled 
  - We may have a meta/project settings json or yaml file in that folder at some point. Some ideas of what might live there
    - Poll frequency
    - Governance
    - Workflows
    - Pointer to templates for various ticket and document types.
    - Allowed states
    - Contributers(?)
  - We may have a templates folder with templates for tickets and document types. 

- Create, edit and manage tickets.
- Create, edit and manage 'managed documents'
- Manage worktrees and branches
- Poll for updates
- Update the cache (tickets, documents and their state)
- When editing tickets and documents, users can rollback/undo, commit and sync their changes (sync is a git push)
- Users can add comments to tickets and documents (subfolder, eg document-attachments/ or ticket-attachments/ ?)
- Users can reply to comments.  They should thread.
- Provide a view/list of tickets (it should list tickets across branches/worktrees)
- Provide a view/list of documents (same, it should show across branches/worktrees)
  - A note on documents in Main AND being edited.  The one being edited should be the one being displayed in lists and when viewing
- Allow users to list, remove and add a folder and/or a remote to work in
- Allow users to enable a folder to be manyhands enabled
- Allow users to create a new folder to be manyhands enabled?
  - it would do the 'git init' step
  - It allows a remote to be added (at any point - you CAN work locally without a remote)
  - maybe allow to autimatically setup the project structure (Kickstart https://github.com/keats/kickstart or Scaffold https://github.com/hay-kot/scaffold) or maybe exclude for now.
  - This implies the ability to add a templates/standards repo (a Meta repo with that and if we do do that, then a way to list a set of repos or repo sets people could just bootstrap from?)
    - This implies that we'd need to poll for changes to the meta repo and update the local repos that get checked out

- Closing a ticket merges it's branch to main (and pushes) and deletes the branch (local and remote)  and worktree for it.
- Closed tickets CAN be viewed, but are not shown by default in the list of tickets.
- I am sure there will be more to come

## Initial technical decisions and directions

(AGENTS - Note this section is more appropriate for a/the RFC document than the prd, but these will drive some of the requirements and constraints of the PRD)

- Rust programming language
- WYSIWYG Markdown editor (https://lib.rs/crates/zorite-editor? (minor preference for this), extract from https://github.com/manyougz/velotype?)

- Use Git as the datastore and for synchronization between users.
- sqlite as a local cache/index of the data found in the repo
- Targeting Windows, Linux and MacOS for delivery
- GPUI/GPUI-KIT for the desktop application framework (Used by the Zed editor)
- libgit2 (and the crate for same) for git functionality
- OpenSource (MIT license)

- Tickets and managed docs editing happens on a branch with it's own worktree
  - suggestion that tickets live in $REPO/.manyhands/tickets/xxxxxx/
  - suggestion that worktrees specific to manyhands live in $REPO/.manyhands/worktrees/xxxxx/ (tickets and docs)
    - Note for enabling manyhands in a repo is to gitignore .manyhands/worktrees

- Initial/default polling frequency

- Comments should be stored as plain md files (perhaps in a manyhands comments folder or a child folder of a ticket )
  - front matter can store reference to the parent your replying to and/or the ticket/document you're commenting on/

- Tickets are probably a md file AND a folder (attachments, comments, etc)

- Document and tickets (meta data at least, contents no) should be indexed in a sqlite database (app wide). This would include branch and worktree.
  - the ui should be driven from that index (eg the db, rather than the filesystem).  Editing is from the filesystem of course.
  - The index can be lost and rebuilt - it's used as a cache.  The filesystem is the source of truth.
  - This kind of implies that the list of repos for the user is a separate list/store (it could be a second sqlite db)

- I am unsure of the exact navigation/layout of the ui should be but I am thinking right now:
  - A dock based ui with:
  - Three main regions: Left most panel 
    - List of Repos, controls to manage that list
    - Under each repo (as children) are two items: Docs and Tickets
    - Clicking on either of those opens a second panel (to the right of the first). With a list of documents or tickets as appropriate
      - This panel changes depending on whether tickets is the active item or documents (eg, one panel container is shared/replaced)
      - list of items should be a small card with a bit of the meta data (tbd - but name, date, type and status (for tickets), perhaps branch and worktree (small, muted as it's secondary) etc)
      - Controls for adding new documents, folders and tickets
      - Perhaps controls filters (tickets: State, team, project; Documents: Document type/class; Both: ownership) probably more
      - Documents can of course be hiearchical,so it should display the folder name and allow that to be opened tree style
      - Clicking on a card, should add a third panel with the contents (the view/editor panel)
        - Document Controls for editing, commiting (save?), syncing (pull & push to branch) and approving/merging/close (Merge to main)
          - these controls should be sensitve to the state (eg commit active when there are changes)
          - It's possible that syncing could commit any uncommitted changes with save just being save
          - Meta data editable as a 'Sheet' component and form appropriate for the item being viewed/edited
        - The editor region can contain multiple open items (tickets AND documents) at once in a tabbed interface.

  - Eventually a status/planning/scorcard/kanban view (by state, including done and recently closed) This is where I'd like to view all, by team, by project or by repo (also 'mine'). 

  - But like I said, I am not entirely sure what's best, so I am open to suggestions.  The real trick will be to make branches and worktrees (and git operations somewhat) invisible/less intrusive to users.  My idea seems a reasonable place to start.


## Step One

We are using spec drive development in this project. This means the process/flow will look something like this:

Gather Requirements -> Notes (We are here) -> PRD -> RFC -> Waves(s) -> Cycle(s) -> Code

The first task is to take these notes and develope a detailed and complete formal PRD stored in docs/PRD. Feel free to suggest things I might have missed, look for any gaps or ambiguity, ask questions as needed. Note any risks or uncertainties. Once we reach a minimum of functionality we will use the software to manage, track and review these documents (so called Dog Fooding our own project).
