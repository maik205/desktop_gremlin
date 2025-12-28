use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::gremlin::{Animation, AnimationProperties};

pub enum LoaderTask {
    Load(AnimationProperties),
    Die,
}

pub struct AsyncAnimationLoader {
    thread_handle: Option<JoinHandle<()>>,
    pub task_tx: Sender<LoaderTask>,
    pub result_rx: Receiver<(String, Animation)>,
}

impl Default for AsyncAnimationLoader {
    fn default() -> Self {
        let (task_tx, task_rx): (Sender<LoaderTask>, Receiver<LoaderTask>) = mpsc::channel();
        let (result_tx, result_rx): (Sender<(String, Animation)>, Receiver<(String, Animation)>) =
            mpsc::channel();

        Self {
            thread_handle: Some(thread::spawn(move || {
                let handle_list: Arc<Mutex<Vec<JoinHandle<(String, Animation)>>>> =
                    Default::default();
                let checker_handle_list = Arc::clone(&handle_list);
                let (checker_heartbeat_tx, checker_heartbeat_rx): (Sender<bool>, Receiver<bool>) =
                    mpsc::channel();
                let checker_heartbeat_tx_outer = checker_heartbeat_tx.clone();

                // the checker
                thread::spawn(move || {
                    while let Ok(true) = checker_heartbeat_rx.recv_timeout(Duration::from_secs(1)) {
                        let mut finished_handles: Vec<usize> = Default::default();
                        let mut handle_list = checker_handle_list.lock().unwrap();
                        if handle_list.len() > 0 {
                            for (index, handle) in handle_list.iter().enumerate() {
                                if handle.is_finished() {
                                    finished_handles.push(index);
                                }
                            }
                        }

                        for handle_indx in finished_handles.iter() {
                            if let Ok(result) = handle_list.remove(*handle_indx).join() {
                                let _ = result_tx.send(result);
                            }
                        }

                        finished_handles.clear();
                    }
                    println!("loader killed");
                });

                // the processor
                thread::spawn(move || {
                    while let Ok(task) = task_rx.recv() {
                        match task {
                            LoaderTask::Load(animation_properties) => {
                                handle_list.lock().unwrap().push(thread::spawn(move || {
                                    (
                                        animation_properties.animation_name.clone(),
                                        <&AnimationProperties as TryInto<Animation>>::try_into(
                                            &animation_properties,
                                        )
                                        .unwrap(),
                                    )
                                }));
                            }
                            LoaderTask::Die => {
                                let _ = checker_heartbeat_tx.send(false);
                                break;
                            }
                        }
                    }
                    println!("processor killed");
                });
                loop {
                    if let Ok(_) = checker_heartbeat_tx_outer.send(true) {
                        thread::sleep(Duration::from_micros(500));
                    } else {
                        break;
                    }
                }
            })),
            task_tx,
            result_rx,
        }
    }
}

impl Drop for AsyncAnimationLoader {
    fn drop(&mut self) {
        let _ = self.task_tx.send(LoaderTask::Die);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}
