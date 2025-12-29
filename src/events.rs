use std::collections::HashMap;

use sdl3::{EventPump, event::Event as SdlEvent};

use crate::utils::MouseKeysState;

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
    Drag { mouse_btn: MouseButton },
    DragEnd { mouse_btn: MouseButton },
    Unhandled,
}

#[derive(PartialEq, Debug)]
pub enum EventData {
    Coordinate {
        x: i32,
        y: i32,
    },
    FCoordinate {
        x: f32,
        y: f32,
    },
    Difference {
        x_rel: f32,
        y_rel: f32,
        x: f32,
        y: f32,
    },
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
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
    Moved,
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
            sdl3::event::WindowEvent::Moved(x, y) => WindowEvent::Moved,
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
#[derive(Debug, Default)]
pub struct EventMediator {
    mouse: MouseState,
    should_check_drag: bool,
}
#[derive(Debug, Default)]

struct MouseState {
    down: MouseKeysState,
    dragging: MouseKeysState,
}

impl MouseState {
    pub fn any_down(&self) -> bool {
        self.down.left || self.down.right || self.down.middle
    }
    pub fn any_drag(&self) -> bool {
        self.dragging.left || self.dragging.right || self.dragging.middle
    }

    pub fn reset_key(&mut self, button: MouseButton) {
        match button {
            MouseButton::Left => {
                self.down.left = false;
                self.dragging.left = false;
            }
            MouseButton::Middle => {
                self.down.middle = false;
                self.dragging.middle = false;
            }
            MouseButton::Right => {
                self.down.right = false;
                self.dragging.right = false;
            }
            _ => {}
        }
    }
}

impl EventMediator {
    pub fn pump_events(
        &mut self,
        sdl_event_pump: &mut EventPump,
    ) -> HashMap<Event, Option<EventData>> {
        let mut event_set: HashMap<Event, Option<EventData>> = Default::default();
        for event in sdl_event_pump.poll_iter() {
            let mut parsed_ev: Option<Event> = None;
            let mut ev_data: Option<EventData> = None;
            match event {
                SdlEvent::MouseButtonDown {
                    mouse_btn, x, y, ..
                } => {
                    self.mouse.down.set_button(&(mouse_btn.into()), true);
                }

                SdlEvent::MouseButtonUp {
                    mouse_btn, x, y, ..
                } => {
                    if !self.mouse.any_drag() {
                        parsed_ev = Some(Event::Click {
                            mouse_btn: mouse_btn.into(),
                        });
                        ev_data = Some(EventData::FCoordinate { x, y });
                    } else if self.mouse.dragging.is_active(&(mouse_btn.into())) {
                        parsed_ev = Some(Event::DragEnd {
                            mouse_btn: mouse_btn.into(),
                        });
                        ev_data = Some(EventData::FCoordinate { x, y });
                    }

                    self.mouse.reset_key(mouse_btn.into());
                }
                SdlEvent::MouseMotion {
                    x, y, xrel, yrel, ..
                } => {
                    for (btn, is_down, is_dragging) in [
                        (
                            MouseButton::Left,
                            self.mouse.down.left,
                            self.mouse.dragging.left,
                        ),
                        (
                            MouseButton::Middle,
                            self.mouse.down.middle,
                            self.mouse.dragging.middle,
                        ),
                        (
                            MouseButton::Right,
                            self.mouse.down.right,
                            self.mouse.dragging.right,
                        ),
                    ] {
                        if is_down && !is_dragging {
                            event_set.insert(
                                Event::DragStart { mouse_btn: btn },
                                Some(EventData::FCoordinate { x, y }),
                            );
                            self.mouse.dragging.set_button(&btn, true);
                        }
                        if is_down && is_dragging {
                            event_set.insert(
                                Event::Drag { mouse_btn: btn },
                                Some(EventData::Difference {
                                    x_rel: xrel,
                                    y_rel: yrel,
                                    x,
                                    y,
                                }),
                            );
                        }
                    }
                }

                SdlEvent::Window {
                    win_event: sdl3::event::WindowEvent::Moved(x, y),
                    ..
                } => {
                    let _ = ev_data.insert(EventData::Coordinate { x, y });
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
