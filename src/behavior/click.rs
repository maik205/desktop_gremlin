use crate::{behavior::Behavior, gremlin::GremlinTask};

pub struct GremlinClick {}

impl Behavior for GremlinClick {
    fn setup(&mut self, _: &mut crate::gremlin::DesktopGremlin) {}

    fn update(
        &mut self,
        application: &mut crate::gremlin::DesktopGremlin,
        context: &super::ContextData,
    ) {
        if let Some(_) = context.events.get(&crate::events::Event::Click {
            mouse_btn: crate::events::MouseButton::Left,
        }) {
            let _ = application
                .task_channel
                .0
                .send(GremlinTask::PlayInterrupt("CLICK".to_string()));
        }
    }
}
