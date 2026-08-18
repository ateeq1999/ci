//! Cross-cutting code shared by multiple `commands::*` modules: dependency
//! abstractions (`context`, `fs`), config types (`db_orm`), and small
//! utilities (`json_payload`, `ui`).

pub mod context;
pub mod db_orm;
pub mod fs;
pub mod json_payload;
pub mod ui;
