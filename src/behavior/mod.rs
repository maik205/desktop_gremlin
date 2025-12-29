use std::collections::HashMap;

use crate::events::{Event, EventData};
use crate::gremlin::DesktopGremlin;
mod click;
mod common;
mod drag;
mod movement;
mod render;

pub use click::*;
pub use common::*;
pub use drag::*;
pub use movement::*;
pub use render::*;
/// Behaviors define actions that the gremlins/application can take and can modify the state of the application/gremlin.<br>
/// This is heavily inspired by Unity's **`MonoBehavior`** superclass. <br>
/// Their lifecycle is as follows:
///
/// `[default()/new()]` -> `setup()` -> `update()` -> `drop()` <br>
/// Note: Behaviors's **initialization** is **not** handled by the runtime, instead requiring each structs to implement their own `new()` or `default()` functions.
/// The runtime only calls `setup()` when behaviors have already been initialized.
pub trait Behavior {
    /// Called once at behavior registration, behaviors can modify the application as necessary.
    fn setup(&mut self, application: &mut DesktopGremlin);

    /// Called every frame and passes the whole execution ctx mutably,
    /// with collected events from the last time the behavior was executed.
    fn update(&mut self, application: &mut DesktopGremlin, context: &ContextData);
}

#[derive(Debug, Default)]
pub struct ContextData {
    pub events: HashMap<Event, Option<EventData>>,
}
