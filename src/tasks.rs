use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};

pub type BoxedTask = Box<dyn FnOnce() + Send + 'static>;

/// A very simple thread pool implementation inspired by the Rust book:
/// <https://doc.rust-lang.org/book/ch21-02-multithreaded.html#building-threadpool-using-compiler-driven-development>
pub struct Runner {
    request: mpsc::Sender<BoxedTask>,
    threads: Vec<JoinHandle<()>>,
}

impl Runner {
    pub fn new_thread_pool(count: usize) -> Self {
        let (tx, rx) = mpsc::channel::<BoxedTask>();
        let rx = Arc::new(Mutex::new(rx));
        let threads = (0..count).map(|i| {
            let rx = Arc::clone(&rx);
            thread::spawn(move || {
                loop {
                    let Ok(task) = rx.lock().unwrap().recv() else {
                        break println!("Worker #{i} has quit"); // Breaking news!!
                    };
                    println!("Running task on worker #{i}");
                    task();
                }
            })
        });
        Self {
            request: tx,
            threads: threads.collect(),
        }
    }
    pub fn run<F: FnOnce() + Send + 'static>(&self, task: F) {
        self.request.send(Box::new(task)).unwrap();
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.request = mpsc::channel().0;
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}
