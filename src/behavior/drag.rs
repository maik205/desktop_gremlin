use super::Behavior;
use crate::behavior::ContextData;
use crate::events::{Event, EventData, MouseButton};
use crate::gremlin::{DesktopGremlin, GremlinTask, get_window_pos};

#[derive(Default, Debug, Clone)]
pub struct GremlinDrag {
    should_move: bool,
    drag_start_x: i32,
    drag_start_y: i32,
}

impl GremlinDrag {
    pub fn new() -> Box<Self> {
        Box::new(Default::default())
    }
}

impl Behavior for GremlinDrag {
    fn update(&mut self, application: &mut DesktopGremlin, context: &ContextData) {
        if let Some(Some(EventData::FCoordinate { x, y })) = context.events.get(&Event::DragStart {
            mouse_btn: MouseButton::Left,
        }) {
            let _ = application
                .task_channel
                .0
                .send(GremlinTask::PlayInterrupt("GRAB".to_string()));

            application.task_queue.clear();

            (self.drag_start_x, self.drag_start_y) = (x.round() as i32, y.round() as i32);
        }

        if let Some(Some(EventData::Difference { x, y, .. })) = context.events.get(&Event::Drag {
            mouse_btn: MouseButton::Left,
        }) {
            if self.should_move {
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
            self.should_move = !self.should_move;
        }

        if let Some(_) = context.events.get(&Event::DragEnd {
            mouse_btn: MouseButton::Left,
        }) {
            let _ = application
                .task_channel
                .0
                .send(GremlinTask::PlayInterrupt("PAT".to_string()));
            let _ = application
                .task_channel
                .0
                .send(GremlinTask::Play("IDLE".to_string()));
        }
    }

    fn setup(&mut self, _: &mut DesktopGremlin) {}
}
