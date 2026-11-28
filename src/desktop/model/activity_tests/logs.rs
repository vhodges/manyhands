//! P2 and P3, the log's two tables: `attention_counts_each_thing_once` (A3's
//! N-table) and `the_log_keeps_times_and_events` (A4's T-table). Both play
//! their rows through a `Scene`, `derive` and `ActivityLog::refresh`.

use super::{intents, states, support, utc};

#[path = "attention.rs"]
mod attention;
#[path = "attention_kept.rs"]
mod attention_kept;
#[path = "attention_more.rs"]
mod attention_more;
#[path = "attention_moved.rs"]
mod attention_moved;
#[path = "attention_retired.rs"]
mod attention_retired;
#[path = "attention_settled.rs"]
mod attention_settled;
#[path = "attention_states.rs"]
mod attention_states;
#[path = "log.rs"]
mod log;
#[path = "log_choices.rs"]
mod log_choices;
#[path = "log_more.rs"]
mod log_more;
#[path = "scene.rs"]
mod scene;
#[path = "table.rs"]
mod table;
