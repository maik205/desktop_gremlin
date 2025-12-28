use std::collections::{HashMap, HashSet};

use sdl3::{EventPump, event::Event as SdlEvent};

use crate::gremlin::MouseKeysState;

// this is to implement eq and hash for event enum
#[derive(PartialEq, Eq, Hash, Debug)]
pub enum Event {
    Quit,
    Click { mouse_btn: MouseButton },
    MouseButtonDown { mouse_btn: MouseButton },
    MouseMove,
    MouseButtonUp { mouse_btn: MouseButton },
    Window { win_event: WindowEvent },
    DragStart { mouse_btn: MouseButton },
    Drag { x: i32, y: i32 },
    DragEnd { mouse_btn: MouseButton },
    Unhandled,
}

#[derive(PartialEq, Debug)]
pub enum EventData {
    Coordinate {
        x: f32,
        y: f32,
    },
    Difference {
        rel_x: f32,
        rel_y: f32,
        x: f32,
        y: f32,
    },
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Unknown,
    X1,
    X2,
}
#[derive(PartialEq, Eq, Hash, Debug)]
pub enum WindowEvent {
    Moved(i32, i32),
    Unhandled,
}

impl From<sdl3::event::Event> for Event {
    fn from(value: sdl3::event::Event) -> Self {
        match value {
            SdlEvent::Quit { .. } => Event::Quit,
            SdlEvent::MouseButtonDown { mouse_btn, .. } => Event::MouseButtonDown {
                mouse_btn: MouseButton::from(mouse_btn),
            },

            SdlEvent::MouseButtonUp { mouse_btn, .. } => Event::MouseButtonUp {
                mouse_btn: MouseButton::from(mouse_btn),
            },
            SdlEvent::MouseMotion { .. } => Event::MouseMove,
            SdlEvent::Window { win_event, .. } => Event::Window {
                win_event: WindowEvent::from(win_event),
            },
            _ => Event::Unhandled,
        }
    }
}

impl From<sdl3::event::WindowEvent> for WindowEvent {
    fn from(value: sdl3::event::WindowEvent) -> Self {
        match value {
            sdl3::event::WindowEvent::Moved(x, y) => WindowEvent::Moved(x, y),
            _ => WindowEvent::Unhandled,
        }
    }
}

impl From<sdl3::mouse::MouseButton> for MouseButton {
    fn from(value: sdl3::mouse::MouseButton) -> Self {
        match value {
            sdl3::mouse::MouseButton::Left => MouseButton::Left,
            sdl3::mouse::MouseButton::Right => MouseButton::Right,
            sdl3::mouse::MouseButton::Middle => MouseButton::Middle,
            sdl3::mouse::MouseButton::Unknown => MouseButton::Unknown,
            sdl3::mouse::MouseButton::X1 => MouseButton::X1,
            sdl3::mouse::MouseButton::X2 => MouseButton::X2,
        }
    }
}

pub struct EventMediator {
    mouse: MouseState,
}
struct MouseState {
    down: MouseKeysState,
    dragging: MouseKeysState,
}

impl EventMediator {
    pub fn pump_events(
        &mut self,
        sdl_event_pump: &mut EventPump,
    ) -> HashMap<Event, Option<EventData>> {
        let mut event_set: HashMap<Event, Option<EventData>> = Default::default();
        for event in sdl_event_pump.poll_iter() {
            let mut parsed_ev = None;
            let mut ev_data = None;
            match event {
                SdlEvent::MouseButtonDown {
                    mouse_btn, x, y, ..
                } => match mouse_btn {
                    sdl3::mouse::MouseButton::Left => {
                        self.mouse.down.left = true;
                    }
                    sdl3::mouse::MouseButton::Middle => {
                        self.mouse.down.middle = true;
                    }
                    sdl3::mouse::MouseButton::Right => {
                        self.mouse.down.right = true;
                    }
                    _ => {}
                },
                SdlEvent::MouseButtonUp {
                    mouse_btn, x, y, ..
                } => {
                    if !self.mouse.dragging.left
                        || !self.mouse.dragging.middle
                        || !self.mouse.dragging.right
                    {
                        parsed_ev = Some(Event::Click {
                            mouse_btn: mouse_btn.into(),
                        });
                        ev_data = Some(EventData::Coordinate { x, y });
                    }

                    match mouse_btn {
                        sdl3::mouse::MouseButton::Left => {
                            self.mouse.down.left = false;
                            self.mouse.dragging.left = false;
                        }
                        sdl3::mouse::MouseButton::Middle => {
                            self.mouse.down.middle = false;
                            self.mouse.dragging.middle = false;
                        }
                        sdl3::mouse::MouseButton::Right => {
                            self.mouse.down.right = false;
                            self.mouse.dragging.right = false;
                        }
                        _ => {}
                    }
                }
                SdlEvent::MouseMotion { mousestate, .. } => {
                    if !self.mouse.dragging.left
                        || !self.mouse.dragging.middle
                        || !self.mouse.dragging.right
                    {}
                }
                _ => {}
            }
            if let Some(parsed_ev) = parsed_ev {
                event_set.insert(parsed_ev, ev_data);
            } else {
                event_set.insert(event.into(), ev_data);
            }
        }

        event_set
    }
}
