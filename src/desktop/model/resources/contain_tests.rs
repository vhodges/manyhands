//! R-34 checks 1 to 3, decided without following a symbolic link (Part B
//! review, Important 1), over temporary trees with no network. A recorder
//! stands in for the filesystem and keeps every path each call is given.
//! Each case checks that no `symlink_metadata` or `read_link` went outside
//! the tree, or through a link to get there, and that a path refused for
//! a link was never made canonical.

use std::{
    cell::RefCell,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

use super::{Found, HOP_LIMIT, Lookups, device, locate_with};

/// The filesystem, recording the paths it is given.
#[derive(Default)]
struct Recorder {
    /// The paths given to `symlink_metadata` and `read_link`.
    looked: RefCell<Vec<PathBuf>>,
    /// The paths given to `canonicalize`.
    canonicalized: RefCell<Vec<PathBuf>>,
}

impl Lookups for Recorder {
    fn symlink_metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
        self.looked.borrow_mut().push(path.to_owned());
        fs::symlink_metadata(path)
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        self.looked.borrow_mut().push(path.to_owned());
        fs::read_link(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        self.canonicalized.borrow_mut().push(path.to_owned());
        path.canonicalize()
    }
}

/// What a lookup found, as a test compares it.
#[derive(Debug, PartialEq)]
enum Seen {
    Nothing,
    Refused,
    Contained(PathBuf),
}

/// A temporary directory holding `repo`, the tree, and `outside`, a
/// folder beside it, in the system's spelling (see `resources_tests`).
struct Trees {
    _dir: tempfile::TempDir,
    outer: PathBuf,
    root: PathBuf,
}

fn trees() -> Trees {
    let dir = tempfile::tempdir().unwrap();
    let outer = dir.path().to_owned();
    let root = outer.join("repo");
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::create_dir_all(outer.join("outside")).unwrap();
    Trees {
        _dir: dir,
        outer,
        root,
    }
}

/// Writes `bytes` at `path`, making its folders.
fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

/// A symbolic link at `at` to `target`, as written.
#[cfg(unix)]
fn link(target: impl AsRef<Path>, at: &Path) {
    fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(target, at).unwrap();
}

/// Looks `written` up below `root` through a recorder. Every path a
/// `symlink_metadata` or `read_link` was given is below the root, and each
/// folder on its way is there and a real folder, so the call followed no
/// link and passed no folder the walk had not. Every path given to
/// `canonicalize`, but the root, is below the root with no link anywhere
/// on it, so no system resolves a link the walk did not decide (Windows
/// takes a relative target's `..` by name, not as the walk does).
fn look(root: &Path, written: &str) -> (Seen, Recorder) {
    let recorder = Recorder::default();
    let seen = match locate_with(root, &root.join(written), &recorder) {
        Found::Nothing => Seen::Nothing,
        Found::Refused => Seen::Refused,
        Found::Contained(canonical) => Seen::Contained(canonical),
    };
    let base = root.canonicalize().unwrap();
    for path in recorder.looked.borrow().iter() {
        let below = path
            .strip_prefix(root)
            .unwrap_or_else(|_| panic!("{written}: {path:?} is outside the tree"));
        let folder = below.parent().unwrap_or(Path::new(""));
        assert_eq!(
            root.join(folder).canonicalize().ok(),
            Some(base.join(folder)),
            "{written}: {path:?} was looked up through a link or a missing folder"
        );
    }
    for path in recorder.canonicalized.borrow().iter() {
        if path == root {
            continue;
        }
        let below = path
            .strip_prefix(root)
            .unwrap_or_else(|_| panic!("{written}: {path:?} was made canonical outside the tree"));
        assert_eq!(
            path.canonicalize().ok(),
            Some(base.join(below)),
            "{written}: {path:?} was made canonical through a link"
        );
    }
    (seen, recorder)
}

/// `written` is refused by the walk itself: before anything was made
/// canonical.
fn refused_unfollowed(root: &Path, written: &str) {
    let (seen, recorder) = look(root, written);
    assert_eq!(seen, Seen::Refused, "{written}");
    assert_eq!(
        recorder.canonicalized.borrow().as_slice(),
        &[] as &[PathBuf],
        "{written}: made canonical before its links passed"
    );
}

/// `path`'s canonical path, as `Contained` holds it.
fn contained_at(path: &Path) -> Seen {
    Seen::Contained(path.canonicalize().unwrap())
}

/// `docs/n -> <outside>`, an absolute target out of the tree: refused from
/// the link alone, so nothing at the target is looked up.
#[cfg(unix)]
#[test]
fn an_absolute_link_out_of_the_tree_is_refused_without_following_it() {
    let Trees { _dir, outer, root } = trees();
    write(&outer.join("outside/pic.png"), b"image");
    link(outer.join("outside"), &root.join("docs/n"));
    refused_unfollowed(&root, "docs/n/pic.png");
    refused_unfollowed(&root, "docs/n");
}

/// `docs/n -> ../../../../elsewhere`: the climb above the root is refused,
/// whether or not anything is there.
#[cfg(unix)]
#[test]
fn a_relative_link_climbing_out_of_the_tree_is_refused_without_following_it() {
    let Trees { _dir, outer, root } = trees();
    write(&outer.join("outside/pic.png"), b"image");
    fs::create_dir_all(root.join("sub")).unwrap();
    link("../../../../elsewhere", &root.join("docs/n"));
    link("../../outside", &root.join("docs/o"));
    link("../sub/../../../outside", &root.join("docs/p"));
    for written in ["docs/n/pic.png", "docs/o/pic.png", "docs/p/pic.png"] {
        refused_unfollowed(&root, written);
    }
}

/// A two-hop chain, `docs/n -> m` with `docs/m -> <outside>`: each link is
/// decided in turn, and the second is refused.
#[cfg(unix)]
#[test]
fn a_chain_of_links_out_of_the_tree_is_refused_link_by_link() {
    let Trees { _dir, outer, root } = trees();
    write(&outer.join("outside/pic.png"), b"image");
    link(outer.join("outside"), &root.join("docs/m"));
    link("m", &root.join("docs/n"));
    refused_unfollowed(&root, "docs/n/pic.png");
}

/// The last step a link out of the tree, by a relative target and by an
/// absolute one.
#[cfg(unix)]
#[test]
fn a_final_link_out_of_the_tree_is_refused_without_following_it() {
    let Trees { _dir, outer, root } = trees();
    write(&outer.join("outside/pic.png"), b"image");
    link("../../outside/pic.png", &root.join("docs/relative.png"));
    link(
        outer.join("outside/pic.png"),
        &root.join("docs/absolute.png"),
    );
    refused_unfollowed(&root, "docs/relative.png");
    refused_unfollowed(&root, "docs/absolute.png");
}

/// Ruling (Known Limit): an absolute target is refused even when it is
/// inside the tree, and a relative one to the same file is followed.
#[cfg(unix)]
#[test]
fn an_absolute_link_inside_the_tree_is_refused() {
    let Trees { _dir, root, .. } = trees();
    let picture = root.join("imgs/pic.png");
    write(&picture, b"image");
    link(&picture, &root.join("docs/file.png"));
    link(root.join("imgs"), &root.join("docs/folder"));
    refused_unfollowed(&root, "docs/file.png");
    refused_unfollowed(&root, "docs/folder/pic.png");
    link("../imgs/pic.png", &root.join("docs/relative.png"));
    assert_eq!(look(&root, "docs/relative.png").0, contained_at(&picture));
}

/// A `..` in a target goes back from where the links before it led, as
/// `realpath` takes it, with no lookup. `docs/n -> m/../x` with
/// `docs/m -> <outside>/a/b` is refused at `m`: collapsing the target
/// first would give `docs/x`, which is there, and making that path
/// canonical would follow `m` out. Inside the tree, `docs/n2 -> m2/../y`
/// with `docs/m2 -> sub/deep` is `docs/sub/y`, not `docs/y`.
#[cfg(unix)]
#[test]
fn a_dot_dot_after_a_link_is_taken_from_where_the_link_led() {
    let Trees { _dir, outer, root } = trees();
    fs::create_dir_all(outer.join("outside/a/b")).unwrap();
    write(&outer.join("outside/a/x"), b"outside");
    write(&root.join("docs/x"), b"inside");
    link(outer.join("outside/a/b"), &root.join("docs/m"));
    link("m/../x", &root.join("docs/n"));
    refused_unfollowed(&root, "docs/n");

    fs::create_dir_all(root.join("docs/sub/deep")).unwrap();
    write(&root.join("docs/sub/y"), b"deep");
    write(&root.join("docs/y"), b"shallow");
    link("sub/deep", &root.join("docs/m2"));
    link("m2/../y", &root.join("docs/n2"));
    assert_eq!(
        look(&root, "docs/n2").0,
        contained_at(&root.join("docs/sub/y"))
    );
}

/// A relative link that stays inside is followed, and so is its chain:
/// `docs/n -> ../imgs`, `docs/c -> n`, and a final link through both.
#[cfg(unix)]
#[test]
fn a_relative_link_inside_the_tree_is_followed_with_its_chain() {
    let Trees { _dir, root, .. } = trees();
    let picture = root.join("imgs/pic.png");
    write(&picture, b"image");
    link("../imgs", &root.join("docs/n"));
    link("n", &root.join("docs/c"));
    link("../c/./pic.png", &root.join("docs/sub/alias.png"));
    for written in ["docs/n/pic.png", "docs/c/pic.png", "docs/sub/alias.png"] {
        let (seen, recorder) = look(&root, written);
        assert_eq!(seen, contained_at(&picture), "{written}");
        assert!(
            !recorder.canonicalized.borrow().is_empty(),
            "{written}: checks 1 and 2 ran on the canonical path"
        );
    }
}

/// A chain is followed up to `HOP_LIMIT` links, as on Linux; one more is
/// refused by the walk, and so is a loop.
#[cfg(unix)]
#[test]
fn a_chain_past_the_hop_limit_is_refused() {
    assert_eq!(HOP_LIMIT, 40);
    let Trees { _dir, root, .. } = trees();
    let picture = root.join("docs/pic.png");
    write(&picture, b"image");
    let chain = |name: &str, links: usize| {
        for at in 1..links {
            link(
                format!("{name}{}", at + 1),
                &root.join(format!("docs/{name}{at}")),
            );
        }
        link("pic.png", &root.join(format!("docs/{name}{links}")));
    };
    chain("a", HOP_LIMIT);
    chain("b", HOP_LIMIT + 1);

    // Only the place the walk reached is made canonical, so no system's own
    // link limit meets the chain.
    let (seen, recorder) = look(&root, "docs/a1");
    assert_eq!(seen, contained_at(&picture), "the walk passed 40 links");
    assert_eq!(
        recorder.canonicalized.borrow().as_slice(),
        &[root.clone(), picture]
    );
    refused_unfollowed(&root, "docs/b1");

    link("loop", &root.join("docs/loop"));
    link("there", &root.join("docs/back"));
    link("back", &root.join("docs/there"));
    refused_unfollowed(&root, "docs/loop");
    refused_unfollowed(&root, "docs/back/pic.png");
}

/// Ruling 3: a missing step of the written path is nothing there, so the
/// next tree is tried. Once its last step is there, a dangling final link,
/// or a reserved step, is something, refused.
#[test]
fn nothing_there_is_told_from_refused() {
    let Trees { _dir, root, .. } = trees();
    write(&root.join(".git/x.png"), b"image");
    for missing in ["docs/missing.png", "docs/none/pic.png", ".git/none.png"] {
        assert_eq!(look(&root, missing).0, Seen::Nothing, "{missing}");
    }
    assert_eq!(look(&root, ".git/x.png").0, Seen::Refused);

    #[cfg(unix)]
    {
        // A link's folder that is not there is a missing step of the
        // path; a final link with nothing at its end is refused.
        link("../imgs", &root.join("docs/n"));
        assert_eq!(look(&root, "docs/n/pic.png").0, Seen::Nothing);
        link("gone.png", &root.join("docs/dangling.png"));
        assert_eq!(look(&root, "docs/dangling.png").0, Seen::Refused);
    }
}

/// Re-review N1: Windows takes a relative target's `..` by name, from the
/// link's folder, so the written path is never made canonical. Here
/// `docs/win -> m2/../L/pic.png`, with `docs/m2 -> sub/deep`, passes the
/// walk to the real `docs/sub/L/pic.png`; by name it would be
/// `docs/L/pic.png`, through `docs/L`, a link out (a temporary folder
/// stands for `//host/share`). Only the place the walk reached, with no
/// link on it, is made canonical.
#[cfg(unix)]
#[test]
fn only_the_place_the_walk_reached_is_made_canonical() {
    let Trees { _dir, outer, root } = trees();
    write(&outer.join("outside/pic.png"), b"share");
    let picture = root.join("docs/sub/L/pic.png");
    write(&picture, b"image");
    fs::create_dir_all(root.join("docs/sub/deep")).unwrap();
    link("sub/deep", &root.join("docs/m2"));
    link(outer.join("outside"), &root.join("docs/L"));
    link("m2/../L/pic.png", &root.join("docs/win"));
    let (seen, recorder) = look(&root, "docs/win");
    assert_eq!(seen, contained_at(&picture));
    assert_eq!(
        recorder.canonicalized.borrow().as_slice(),
        &[root.clone(), picture],
        "the root, and the place the walk reached"
    );
}

/// A `..` after a file is no path, as the system finds it: refused, not
/// taken as a step back from the file.
#[cfg(unix)]
#[test]
fn a_dot_dot_after_a_file_is_refused() {
    let Trees { _dir, root, .. } = trees();
    write(&root.join("docs/a.png"), b"image");
    write(&root.join("docs/x.png"), b"image");
    link("a.png/../x.png", &root.join("docs/n.png"));
    refused_unfollowed(&root, "docs/n.png");
}

/// Re-review N2: a relative link into `.git` is refused, through it and
/// as the last step. The walk notes the reserved step and goes on to the
/// end of the path, inside the tree, so a missing file there is nothing
/// there (ruling 3); then it refuses, before anything is made canonical.
#[cfg(unix)]
#[test]
fn a_relative_link_into_git_is_refused() {
    let Trees { _dir, root, .. } = trees();
    write(&root.join(".git/x.png"), b"image");
    link("../.git", &root.join("docs/g"));
    link("../.git/x.png", &root.join("docs/gx.png"));
    for (written, walked) in [
        (
            "docs/g/x.png",
            ["docs", "docs/g", "docs/g", ".git", ".git/x.png"],
        ),
        (
            "docs/gx.png",
            ["docs", "docs/gx.png", "docs/gx.png", ".git", ".git/x.png"],
        ),
    ] {
        refused_unfollowed(&root, written);
        let (_, recorder) = look(&root, written);
        let walked: Vec<PathBuf> = walked.iter().map(|path| root.join(path)).collect();
        assert_eq!(*recorder.looked.borrow(), walked, "{written}");
    }
}

/// What Windows reads as a device name: the part before the first `.`,
/// its trailing spaces trimmed, ignoring ASCII case, is CON, PRN, AUX,
/// NUL, COM0 to COM9 or LPT0 to LPT9 (and COM or LPT with a superscript
/// 1, 2 or 3), CONIN$ or CONOUT$. A `:` ends the name too, as `reserved`
/// reads it.
#[test]
fn a_windows_device_name_is_matched_as_windows_reads_it() {
    for name in [
        "CON",
        "con.png",
        "PRN",
        "aux .png",
        "NUL",
        "nul",
        "nul.",
        "Nul .tar.gz",
        "COM0",
        "com9.png",
        "Com3.tar.png",
        "LPT0",
        "lpt9.jpg",
        "LPT1.png",
        "COM\u{b9}",
        "lpt\u{b2}.png",
        "LPT\u{b3}",
        "CONIN$",
        "conout$.png",
        "CON:",
        "lpt1:x.png",
    ] {
        assert!(device(OsStr::new(name)), "{name}");
    }
    for name in [
        "console.png",
        "lpt10.png",
        "com.png",
        "con_x.png",
        "connect",
        "auxiliary",
        "nul_",
        " con.png",
        "COM\u{b4}",
        "LPT\u{2074}",
        "CONIN",
        "CONOUT",
        "x.con",
        "docs",
    ] {
        assert!(!device(OsStr::new(name)), "{name}");
    }
}

/// Contract 12: a device is never opened. A step Windows reads as a
/// device, written or in a link's target, is refused on every system,
/// before any lookup of that step; a name that only starts like one is a
/// name. Here the devices are plain files, so only the name refuses them.
#[cfg(unix)]
#[test]
fn a_windows_device_name_is_refused_without_a_lookup() {
    let Trees { _dir, root, .. } = trees();
    let devices = [
        "LPT1.png",
        "con.png",
        "nul",
        "Com3.tar.png",
        "aux .png",
        "NUL.png",
    ];
    for name in devices {
        write(&root.join("docs").join(name), b"image");
    }
    link("../docs/NUL.png", &root.join("imgs/n.png"));
    let refused = devices[..5]
        .iter()
        .map(|name| (format!("docs/{name}"), format!("docs/{name}")))
        .chain([("imgs/n.png".to_owned(), "docs/NUL.png".to_owned())]);
    for (written, device) in refused {
        refused_unfollowed(&root, &written);
        let (_, recorder) = look(&root, &written);
        assert!(
            !recorder.looked.borrow().contains(&root.join(&device)),
            "{written}: {device} was looked up"
        );
    }
    for name in ["console.png", "lpt10.png", "com.png"] {
        let path = root.join("docs").join(name);
        write(&path, b"image");
        assert_eq!(
            look(&root, &format!("docs/{name}")).0,
            contained_at(&path),
            "{name}"
        );
    }
}
