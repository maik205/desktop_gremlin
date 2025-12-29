use std::{sync::mpsc, thread, time::Duration};

use crate::{
    behavior::{Behavior, ContextData},
    events::EventMediator,
    gremlin::{DesktopGremlin, GLOBAL_FRAMERATE},
};

#[derive(Default)]
pub struct DGRuntime {
    behaviors: Vec<Box<dyn Behavior>>,
}

impl DGRuntime {
    pub fn _register_behavior(&mut self, behavior: Box<dyn Behavior>) {
        self.behaviors.push(behavior);
    }
    pub fn register_behaviors(&mut self, behavior: Vec<Box<dyn Behavior>>) {
        let mut behavior = behavior;
        self.behaviors.append(&mut behavior);
    }

    pub fn go(&mut self) {
        let (heartbeat_tx, heartbeat_rx) = mpsc::sync_channel::<()>(1);

        let heartbeat = thread::spawn(move || {
            while let Ok(_) = heartbeat_tx.send(()) {
                thread::sleep(Duration::from_secs_f64(1.0 / (GLOBAL_FRAMERATE as f64)));
            }
            println!("Heartbeat stopped, someone get the zapper!");
        });

        if let Ok(mut application) = DesktopGremlin::new(None) {
            application.current_gremlin = application
            .load_gremlin(
                r"C:\Users\ASUS\Documents\Projects\desktop_gremlin\assets\Gremlins\Mambo\config.txt".to_string()
            )
            .ok();

            let mut event_pump = application.sdl.event_pump().unwrap();
            let mut event_mediator = EventMediator::default();

            for behavior in self.behaviors.iter_mut() {
                behavior.setup(&mut application);
            }

            while let Ok(_) = heartbeat_rx.recv() {
                let events = event_mediator.pump_events(&mut event_pump);
                let context = ContextData { events: events };
                for behavior in self.behaviors.iter_mut() {
                    behavior.update(&mut application, &context);
                }

                if let Ok(should_exit_lock) = application.should_exit.lock()
                    && *should_exit_lock == true
                {
                    break;
                }
            }
        }
        drop(heartbeat_rx);
        let _ = heartbeat.join();
    }
}
