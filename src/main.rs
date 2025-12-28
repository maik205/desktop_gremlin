use std::collections::HashSet;

use crate::{
    behavior::{Behavior, drag::GremlinDrag},
    gremlin::DesktopGremlin,
};

pub mod behavior;
mod events;
pub mod gremlin;
mod io;
pub mod ui;
pub mod utils;

fn main() {
    let mut app = DesktopGremlin::new(None).unwrap();
    let mut behaviors: Vec<Box<dyn Behavior>> = vec![GremlinDrag::new()];
    // app.register_behaviors(behaviors);
    // move this to the go() function after refactor
    loop {
        app.update();
        for behavior in behaviors.iter_mut() {
            behavior.update(&mut app, &Default::default());
        }
        if let true = *app.should_exit.lock().unwrap() {
            break;
        }
    }
    // app.go();
}
