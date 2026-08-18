//! `db`'s own event registration — today just prints, but this file (not
//! a shared one) is where a future `db`-specific action would go without
//! touching `init`'s or `update`'s.

use crate::shared::context::Context;
use crate::shared::events::{EventBus, PrintAction};

pub fn bus(ctx: &Context) -> EventBus<'_> {
    EventBus::new(vec![Box::new(PrintAction::new(ctx.ui.as_ref()))])
}
