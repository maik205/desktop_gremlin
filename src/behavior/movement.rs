use std::collections::{HashMap, HashSet};

use crate::{behavior::ContextData, events::{Event, EventData}, gremlin::DesktopGremlin};

const DEFAULT_VELOCITY: f32 = 250.0;

pub struct GremlinMovement {
    _velocity: f32,
    _is_active: bool,
}

impl Default for GremlinMovement {
    fn default() -> Self {
        Self {
            _velocity: DEFAULT_VELOCITY,
            _is_active: Default::default(),
        }
    }
}
impl super::Behavior for GremlinMovement {
    fn setup(&mut self, _: &mut DesktopGremlin) {}

    fn update(&mut self, _: &mut DesktopGremlin, context_data: &ContextData) {
        if let Some(_) = context_data.events.get(
            &(Event::MouseButtonDown {
                mouse_btn: crate::events::MouseButton::Left,
            }),
        ) {}
    }
}
