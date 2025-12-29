use std::time::Instant;

use sdl3::rect::Point;

use crate::{
    behavior::ContextData,
    events::{Event, EventData, MouseButton},
    gremlin::{DesktopGremlin, GremlinTask},
    utils::{DirectionX, DirectionY, get_cursor_position, get_move_direction, win_to_rect},
};

const DEFAULT_VELOCITY: f32 = 250.0;

pub struct GremlinMovement {
    velocity: f32,
    is_active: bool,
    is_dragging: bool,
    current_position: (i32, i32),
    last_moved_at: Instant,
    should_check_position: bool,
}

impl Default for GremlinMovement {
    fn default() -> Self {
        Self {
            velocity: DEFAULT_VELOCITY,
            is_active: Default::default(),
            is_dragging: Default::default(),
            current_position: Default::default(),
            last_moved_at: Instant::now(),
            should_check_position: true,
        }
    }
}
impl super::Behavior for GremlinMovement {
    fn setup(&mut self, _: &mut DesktopGremlin) {}

    fn update(&mut self, application: &mut DesktopGremlin, context: &ContextData) {
        if let Some(_) = context.events.get(&Event::Click {
            mouse_btn: MouseButton::Left,
        }) {
            if !self.is_active {
                self.last_moved_at = Instant::now();
            }

            self.is_active = !self.is_active;
        }
        if let Some(_) = context.events.get(&Event::DragStart {
            mouse_btn: MouseButton::Left,
        }) {
            self.is_dragging = true;
        }
        if let Some(_) = context.events.get(&Event::DragEnd {
            mouse_btn: MouseButton::Left,
        }) {
            self.is_dragging = false;
        }

        if self.is_active
            && !self.is_dragging
            && let Some(ref gremlin) = application.current_gremlin
            && let Some(ref animator) = gremlin.animator
        {
            let (gremlin_x, gremlin_y) = self.current_position;

            let gremlin_center = Point::new(
                gremlin_x + ((application.canvas.window().size().0 / 2) as i32),
                gremlin_y + ((application.canvas.window().size().1 / 2) as i32),
            );

            let (cursor_x, cursor_y) = get_cursor_position();
            let move_target = Point::new(cursor_x as i32, cursor_y as i32);
            let (dir_x, dir_y) = get_move_direction(move_target, {
                let mut win_rect = win_to_rect(application.canvas.window());
                if win_rect.contains_point(move_target) {
                    win_rect.resize(win_rect.width() + 100, win_rect.height() + 100);
                    println!("{:?}", win_rect);
                }
                win_rect
            });
            let tan = ((gremlin_center.y - move_target.y) as f32)
                / ((gremlin_center.x - move_target.x) as f32);
            let alpha = tan.atan();

            let (velo_x, x_anim) = match dir_x {
                DirectionX::None => (0.0, ""),
                DirectionX::Left => (-self.velocity, "LEFT"),
                DirectionX::Right => (self.velocity, "RIGHT"),
            };
            let (velo_y, y_anim) = match dir_y {
                DirectionY::None => (0.0, ""),
                DirectionY::Up => (-self.velocity, "UP"),
                DirectionY::Down => (self.velocity, "DOWN"),
            };

            let animation_name = match (dir_x, dir_y) {
                (DirectionX::None, DirectionY::None) => "RUNIDLE".to_string(),
                (DirectionX::None, _) => "RUN".to_string() + y_anim,
                (_, DirectionY::None) => "RUN".to_string() + x_anim,
                (_, _) => y_anim.to_string() + x_anim,
            };
            if animator.animation_properties.animation_name != animation_name {
                let _ = application
                    .task_channel
                    .0
                    .send(GremlinTask::PlayInterrupt(animation_name));
                application.task_queue.clear();
            }

            let (velo_x, velo_y) = (velo_x * alpha.cos().abs(), velo_y * alpha.sin().abs());

            application.canvas.window_mut().set_position(
                sdl3::video::WindowPos::Positioned(
                    ((gremlin_x as f32) + velo_x * self.last_moved_at.elapsed().as_secs_f32())
                        as i32,
                ),
                sdl3::video::WindowPos::Positioned(
                    ((gremlin_y as f32) + velo_y * self.last_moved_at.elapsed().as_secs_f32())
                        as i32,
                ),
            );

            self.last_moved_at = Instant::now();
        }

        if self.should_check_position
            && let Some(Some(EventData::Coordinate { x, y })) = context.events.get(&Event::Window {
                win_event: crate::events::WindowEvent::Moved,
            })
        {
            self.current_position.0 = *x;
            self.current_position.1 = *y;
        }
        self.should_check_position = !self.should_check_position;
    }
}

impl GremlinMovement {
    pub fn new() -> Box<Self> {
        Default::default()
    }
}
