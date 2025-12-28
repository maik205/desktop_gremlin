use std::collections::HashMap;
use std::{collections::HashSet, time::Instant};

use super::Behavior;
use crate::behavior::ContextData;
use crate::events::{Event, EventData, MouseButton};
use crate::gremlin::{DesktopGremlin, GremlinTask, MouseKeysState, get_window_pos};

#[derive(Debug, Clone)]
pub struct GremlinDrag {
    is_dragging: bool,
    key_state: MouseKeysState,
    move_torwards_cursor: bool,
    last_moved_at: Instant,
    should_check_drag: bool,
    drag_start_x: i32,
    drag_start_y: i32,
}

impl GremlinDrag {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            is_dragging: Default::default(),
            key_state: Default::default(),
            move_torwards_cursor: Default::default(),
            last_moved_at: Instant::now(),
            should_check_drag: Default::default(),
            drag_start_x: Default::default(),
            drag_start_y: Default::default(),
        })
    }
}

impl Behavior for GremlinDrag {
    fn update(&mut self, application: &mut DesktopGremlin, context: &ContextData) {
        if let Some(_) = context.events.get(
            &(Event::MouseButtonDown {
                mouse_btn: MouseButton::Left,
            }),
        ) {
            self.key_state.left = true;
        }

        if let Some(_) = context.events.get(
            &(Event::MouseButtonUp {
                mouse_btn: MouseButton::Left,
            }),
        ) {
            if !self.is_dragging && self.key_state.left {
                let _ = application
                    .task_channel
                    .0
                    .send(GremlinTask::PlayInterrupt("CLICK".to_string()));
                self.move_torwards_cursor = !self.move_torwards_cursor;
                self.last_moved_at = Instant::now();
            }
            if self.is_dragging && self.key_state.left {
                let _ = application
                    .task_channel
                    .0
                    .send(GremlinTask::PlayInterrupt("PAT".to_string()));
            }
            let _ = application
                .task_channel
                .0
                .send(GremlinTask::Play("IDLE".to_string()));
            self.is_dragging = false;
            self.key_state.left = false;
        }
        if let Some(EventData::Coordinate { x, y }) = context.events.get(&Event::MouseMove) {
            if self.key_state.left && !self.is_dragging {
                self.is_dragging = true;
                let _ = application
                    .task_channel
                    .0
                    .send(GremlinTask::PlayInterrupt("GRAB".to_string()));
                application.task_queue.clear();
                (self.drag_start_x, self.drag_start_y) = (x.round() as i32, y.round() as i32);
            }
            if self.is_dragging && self.should_check_drag {
                let (gremlin_x, gremlin_y) = get_window_pos(&application.canvas);
                application.canvas.window_mut().set_position(
                    sdl3::video::WindowPos::Positioned(
                        gremlin_x.saturating_add(((x.round() as i32) - self.drag_start_x) as i32),
                    ),
                    sdl3::video::WindowPos::Positioned(
                        gremlin_y.saturating_add(((y.round() as i32) - self.drag_start_y) as i32),
                    ),
                );
            }
            // only move every odd frame because moving the window will trigger another mousemove event
            self.should_check_drag = !self.should_check_drag;
        }
    }

    fn setup(&mut self, _: &mut DesktopGremlin) {}
}
