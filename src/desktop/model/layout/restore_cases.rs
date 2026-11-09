//! The one restore table (Part G; P24): `restore` over every case, each
//! asserting the result and whether the notice shows. A bad file is the
//! valid one with one thing changed.

use serde_json::{Value, json};

use super::{Layout, layout};
use crate::desktop::{
    model::layout::{LAYOUT_VERSION, Restore, encode, restore, restore::Saved, state_for},
    services::layout::LayoutStoreError,
};

const SIZES: [f32; 3] = [290., 440., 150.];

/// A valid file for `layout`, as JSON to change.
fn file(layout: &Layout, sizes: [f32; 3]) -> Value {
    serde_json::from_slice(&encode(layout, &state_for(layout, sizes))).unwrap()
}

fn bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

fn leaf(name: &str) -> Value {
    json!({"panel_name": name, "children": [], "info": {"panel": null}})
}

fn removing(value: &mut Value, parent: &str, key: &str) {
    value
        .pointer_mut(parent)
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove(key);
}

fn push(value: &mut Value, pointer: &str, item: Value) {
    value
        .pointer_mut(pointer)
        .unwrap()
        .as_array_mut()
        .unwrap()
        .push(item);
}

type Edit = fn(&mut Value);

/// The default arrangement's file changed by one edit.
fn edited(case: &'static str, edit: Edit) -> (&'static str, Value) {
    let mut value = file(&layout("R/D|T|A", "", ""), SIZES);
    edit(&mut value);
    (case, value)
}

pub(super) fn run() {
    let default = || layout("R/D|T|A", "", "");

    // No file is a first launch, and an unreadable one is not.
    assert_eq!(restore(Ok(None)), Restore::Default { notice: false });
    assert_eq!(
        restore(Err(LayoutStoreError::ProtectionUnavailable)),
        Restore::Default { notice: true }
    );
    // Not JSON, and JSON of another shape.
    for (case, raw) in [
        ("not JSON", b"{not json".to_vec()),
        ("not UTF-8", vec![0xff, 0xfe, 0x00]),
        ("an array", b"[1, 2]".to_vec()),
        ("an object of another shape", b"{\"hello\": 1}".to_vec()),
        ("empty", Vec::new()),
    ] {
        assert_eq!(
            restore(Ok(Some(raw))),
            Restore::Default { notice: true },
            "{case}"
        );
    }

    let rows: Vec<(&'static str, Value)> = vec![
        edited("version 0", |v| v["version"] = json!(0)),
        edited("version 2", |v| v["version"] = json!(2)),
        edited("an unknown top-level key", |v| v["extra"] = json!(1)),
        edited("a dock in the wrong slot", |v| {
            v["area"]["left_dock"]["placement"] = json!("right")
        }),
        edited("an unknown leaf", |v| {
            v["area"]["right_dock"]["panel"]["children"][0]["panel_name"] = json!("Nope")
        }),
        edited("a missing panel", |v| {
            removing(v, "/area", "bottom_dock");
        }),
        edited("a repeated panel", |v| {
            push(
                v,
                "/area/bottom_dock/panel/children",
                leaf("RepositoriesPanel"),
            )
        }),
        edited("a panel in the centre", |v| {
            push(v, "/area/center/children/0/children", leaf("TicketsPanel"))
        }),
        edited("no item area in the centre", |v| {
            v["area"]["center"]["children"][0]["children"][0]["panel_name"] = json!("Nope")
        }),
        edited("the item area in a dock", |v| {
            push(v, "/area/bottom_dock/panel/children", leaf("ItemAreaPanel"))
        }),
        edited("a TabPanel with Panel info", |v| {
            v["area"]["right_dock"]["panel"]["info"] = json!({"panel": null})
        }),
        edited("a StackPanel with Tabs info", |v| {
            v["area"]["left_dock"]["panel"]["info"] = json!({"tabs": {"active_index": 0}})
        }),
        edited("a leaf with Stack info", |v| {
            v["area"]["right_dock"]["panel"]["children"][0]["info"] =
                json!({"stack": {"sizes": [100.0], "axis": 0}})
        }),
        edited("a leaf named for a container", |v| {
            v["area"]["right_dock"]["panel"]["children"][0]["panel_name"] = json!("TabPanel")
        }),
        edited("an unknown container", |v| {
            v["area"]["right_dock"]["panel"]["panel_name"] = json!("Foo")
        }),
        edited("a StackPanel holding a leaf", |v| {
            v["area"]["left_dock"]["panel"]["children"][0] = leaf("RepositoriesPanel")
        }),
        edited("a TabPanel holding a container", |v| {
            let inner = v["area"]["right_dock"]["panel"].clone();
            v["area"]["right_dock"]["panel"]["children"] = json!([inner]);
        }),
        edited("a sizes count unlike the children", |v| {
            push(v, "/area/left_dock/panel/info/stack/sizes", json!(100.0))
        }),
        edited("a split size of 0", |v| {
            v["area"]["left_dock"]["panel"]["info"]["stack"]["sizes"][0] = json!(0.0)
        }),
        edited("a negative split size", |v| {
            v["area"]["left_dock"]["panel"]["info"]["stack"]["sizes"][1] = json!(-5.0)
        }),
        edited("a split axis of 2", |v| {
            v["area"]["left_dock"]["panel"]["info"]["stack"]["axis"] = json!(2)
        }),
        edited("a dock of 99 px", |v| {
            v["area"]["right_dock"]["size"] = json!(99.0)
        }),
        edited("a dock with no panels", |v| {
            v["area"]["bottom_dock"]["panel"]["children"] = json!([]);
            v["area"]["bottom_dock"]["panel"]["info"] = json!({"tabs": {"active_index": 0}});
        }),
        edited("an empty group", |v| {
            push(
                v,
                "/area/left_dock/panel/children",
                json!({"panel_name": "TabPanel", "children": [],
                       "info": {"tabs": {"active_index": 0}}}),
            );
            push(v, "/area/left_dock/panel/info/stack/sizes", json!(100.0));
        }),
        edited("a tab selected past the last", |v| {
            v["area"]["right_dock"]["panel"]["info"] = json!({"tabs": {"active_index": 1}})
        }),
        edited("a leaf as a dock's root", |v| {
            v["area"]["right_dock"]["panel"] = leaf("TicketsPanel")
        }),
        edited("on missing a key", |v| {
            removing(v, "/on", "TicketsPanel");
        }),
        edited("on with an unknown key", |v| v["on"]["Nope"] = json!(true)),
        edited("shown missing a key", |v| {
            removing(v, "/shown", "left");
        }),
        edited("shown with an unknown key", |v| {
            v["shown"]["top"] = json!(true)
        }),
    ];
    for (case, value) in rows {
        assert_eq!(
            restore(Ok(Some(bytes(&value)))),
            Restore::Default { notice: true },
            "{case}"
        );
    }

    // Valid files restore with their on flags, shown docks, groups, active
    // tabs and sizes, and no notice.
    let mut sized = layout("R/D|T|A", "", "");
    sized.sizes = [300., 500., 220.];
    let valid: [(&str, Layout, [f32; 3], Layout); 5] = [
        ("the default", default(), SIZES, default()),
        (
            "a closed dock and a hidden panel",
            layout("R/D|T|A", "D", "r"),
            SIZES,
            layout("R/D|T|A", "D", "r"),
        ),
        (
            "a moved panel and a side with no dock",
            layout("R/D/T||A", "", ""),
            SIZES,
            layout("R/D/T||A", "", ""),
        ),
        (
            "a tab group",
            layout("R+D*|T|A", "", ""),
            SIZES,
            layout("R+D*|T|A", "", ""),
        ),
        ("changed sizes", sized.clone(), [300., 500., 220.], sized),
    ];
    for (case, saved, sizes, want) in valid {
        let got = restore(Ok(Some(encode(&saved, &state_for(&saved, sizes)))));
        match got {
            Restore::Saved(got) => {
                let Saved { layout, area } = *got;
                assert_eq!(layout, want, "{case}");
                assert_eq!(area.version, Some(LAYOUT_VERSION), "{case}");
            }
            got => panic!("{case}: {got:?}"),
        }
    }
    // A file whose area carries another version is read at the file's.
    let mut other = file(&default(), SIZES);
    other["area"]["version"] = json!(7);
    match restore(Ok(Some(bytes(&other)))) {
        Restore::Saved(saved) => assert_eq!(saved.area.version, Some(LAYOUT_VERSION)),
        got => panic!("an area version is not trusted: {got:?}"),
    }
}
