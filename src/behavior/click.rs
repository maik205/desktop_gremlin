use crate::{
    behavior::Behavior,
    gremlin::{DesktopGremlin, GremlinTask},
};
#[derive(Default)]
pub struct GremlinClick {}

impl GremlinClick {
    pub fn new() -> Box<Self> {
        Default::default()
    }
}

impl Behavior for GremlinClick {
    fn setup(&mut self, _: &mut crate::gremlin::DesktopGremlin) {}

    fn update(&mut self, application: &mut DesktopGremlin, context: &super::ContextData) {
        if let Some(_) = context.events.get(&crate::events::Event::Click {
            mouse_btn: crate::events::MouseButton::Left,
        }) {
            let _ = application
                .task_channel
                .0
                .send(GremlinTask::PlayInterrupt("CLICK".to_string()));
            let _ = application
                .task_channel
                .0
                .send(GremlinTask::Play("IDLE".to_string()));
        }
    }
}
