use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};

use crate::excuses::{EXP_INIT, INIT_ERR};

pub type BoxedTask = Box<dyn FnOnce() + Send + 'static>;

/// A very simple thread pool implementation inspired by the Rust book:
/// <https://doc.rust-lang.org/book/ch21-02-multithreaded.html#building-threadpool-using-compiler-driven-development>
pub struct Runner {
    request: mpsc::Sender<BoxedTask>,
    threads: Vec<JoinHandle<()>>,
}

impl Runner {
    /// Creates a new instance of with a specified number
    /// of worker threads (must be at least 1)
    ///
    /// # Panics
    /// The function panics if any threads fail to spawn
    #[must_use]
    pub fn new(count: usize) -> Self {
        debug_assert!(count > 0, "Cannot create a thread pool with no threads");

        let (tx, rx) = mpsc::channel::<BoxedTask>();
        let rx = Arc::new(Mutex::new(rx));
        let threads = (0..count).map(|i| {
            let rx = Arc::clone(&rx);
            thread::Builder::new()
                .name(format!("worker_{i}"))
                .spawn(move || {
                    loop {
                        let Ok(task) = rx.lock().unwrap().recv() else {
                            break println!("Worker #{i} has quit"); // Breaking news!!
                        };
                        // println!("Running task on worker #{i}");
                        task();
                    }
                })
                .expect(INIT_ERR)
        });
        Self {
            request: tx,
            threads: threads.collect(),
        }
    }
    /// Runs a new task in the thread pool. If all available
    /// threads are busy, the task will wait in a queue.
    ///
    /// # Panics
    /// The function panics if no pool threads are running
    pub fn run<T>(&self, task: T)
    where
        T: FnOnce() + Into<Box<T>> + Send + 'static,
    {
        self.request.send(task.into()).expect(EXP_INIT);
    }

    /// Blocks until all tasks are done then shuts down its worker
    /// threads, leaving the `Runner` in an unusable state
    pub fn shutdown(&mut self) {
        self.request = mpsc::channel().0;
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.shutdown();
    }
}
