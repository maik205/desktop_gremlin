use super::Behavior;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct CommonBehavior {}

impl Behavior for CommonBehavior {
    fn setup(&mut self, application: &mut crate::gremlin::DesktopGremlin) {
        application.current_gremlin = application
            .load_gremlin(
                r"C:\Users\ASUS\Documents\Projects\desktop_gremlin\assets\Gremlins\Mambo\config.txt".to_string()
            )
            .ok();

        let _ = application
            .task_channel
            .0
            .send(crate::gremlin::GremlinTask::Play("INTRO".to_string()));

        let _ = application
            .task_channel
            .0
            .send(crate::gremlin::GremlinTask::Play("IDLE".to_string()));
    }

    fn update(
        &mut self,
        application: &mut crate::gremlin::DesktopGremlin,
        context: &super::ContextData,
    ) {
        if let Some(_) = context.events.get(&crate::events::Event::Quit) {
            application.task_queue.clear();
            let _ = application
                .task_channel
                .0
                .send(crate::gremlin::GremlinTask::PlayInterrupt(
                    "OUTRO".to_string(),
                ));
        }
    }
}

impl CommonBehavior {
    pub fn new() -> Box<Self> {
        Default::default()
    }
}
