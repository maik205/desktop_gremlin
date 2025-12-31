use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
};

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    work_sender: Option<Sender<Job>>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        let (tx, rx): (
            Sender<Box<dyn FnOnce() + Send + 'static>>,
            Receiver<Box<dyn FnOnce() + Send + 'static>>,
        ) = mpsc::channel();
        let work_receiver: Arc<Mutex<Receiver<Job>>> = Arc::new(Mutex::new(rx));
        let mut workers: Vec<Worker> = Vec::new();
        for i in 0..size {
            workers.push(Worker::new(i, &work_receiver));
        }
        ThreadPool {
            workers,
            work_sender: Some(tx),
        }
    }
    #[allow(unused)]
    pub fn exec<Fn>(&self, work: Fn)
    where
        Fn: FnOnce() + Send + 'static,
    {
        match &self.work_sender {
            Some(sender) => {
                sender.send(Box::new(work)).unwrap();
            }
            _ => (),
        }
    }
}
struct Worker {
    id: usize,
    thread: JoinHandle<()>,
}
impl Worker {
    pub fn new(id: usize, recv: &Arc<Mutex<Receiver<Job>>>) -> Worker {
        let recv: Arc<Mutex<Receiver<Box<dyn FnOnce() + Send + 'static>>>> = Arc::clone(recv);

        let thread = thread::spawn(move || {
            loop {
                // This acquires and unwraps the value of the Mutex lock fyi
                let message = recv.lock().unwrap().recv();
                match message {
                    Ok(job) => {
                        job();
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
        });

        Worker { id, thread }
    }
}
impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.work_sender.take());

        for worker in &mut self.workers.drain(..) {
            match worker.thread.join() {
                Err(_) => {
                    panic!("Unable to stop thread {}", worker.id);
                }
                _ => (),
            }
        }
    }
}
