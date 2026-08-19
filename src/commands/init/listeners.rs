//! `init`'s own event registration — prints progress and records the
//! outcome to the *new* project's `ci/history.jsonl`. This file (not a
//! shared one) is where a future `init`-specific action would go without
//! touching `update`'s or `db`'s.

use std::path::Path;

use crate::shared::context::Context;
use crate::shared::events::{EventBus, PrintAction};
use crate::shared::history::HistoryAction;

pub fn bus<'a>(ctx: &'a Context, root: &Path) -> EventBus<'a> {
    EventBus::new(vec![
        Box::new(PrintAction::new(ctx.ui.as_ref())),
        Box::new(HistoryAction::new(ctx.fs.as_ref(), root)),
    ])
}
