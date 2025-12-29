use crate::{behavior::*, runtime::DGRuntime};

mod behavior;
mod events;
mod gremlin;
mod io;
mod runtime;
mod ui;
mod utils;

fn main() {
    let mut rt = DGRuntime::default();

    let behaviors: Vec<Box<dyn Behavior>> = vec![
        CommonBehavior::new(),
        GremlinDrag::new(),
        GremlinMovement::new(),
        GremlinRender::new(),
        GremlinClick::new(),
    ];

    rt.register_behaviors(behaviors);
    rt.go();
}
