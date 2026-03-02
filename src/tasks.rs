use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};

use crate::excuses::INIT_ERR;

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
    #[inline]
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
    #[inline]
    pub fn run<T>(&self, task: T)
    where
        T: FnOnce() + Into<Box<T>> + Send + 'static,
    {
        if let Err(e) = self.request.send(task.into()) {
            eprintln!("Could not send task to the thread pool: {e}");
        }
    }

    /// Causes the thread pool to wait until all current
    /// tasks have finished before running any new ones
    ///
    /// The caller may use the returned channel to be
    /// notified when there are no more running tasks
    ///
    /// Note: The caller must not use the receiver to handle more than
    /// one message, otherwise workers could become permanently stuck
    #[inline]
    #[must_use]
    pub fn await_all_tasks(&self) -> Arc<Mutex<mpsc::Receiver<()>>> {
        let (unblock_tx, unblock_rx) = mpsc::channel();
        let unblock_rx = Arc::new(Mutex::new(unblock_rx));
        let num_tasks = self.threads.len();

        // Occupy all but one of the workers with a blocking operation
        for _ in 1..num_tasks {
            let unblock_rx = Arc::clone(&unblock_rx);
            self.run(move || unblock_rx.lock().unwrap().recv().unwrap());
        }

        // When this task gets its turn in the queue, all tasks
        // started prior to this function have finished processing
        self.run(move || {
            // Notify the other workers to stop waiting
            // (and one extra for the function caller)
            for _ in 0..num_tasks {
                let _ = unblock_tx.send(());
            }
        });

        unblock_rx
    }

    /// Blocks until all tasks are done then shuts down its worker
    /// threads, leaving the `Runner` in an unusable state
    pub fn shutdown(&mut self) {
        self.request = mpsc::channel().0;
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }

    #[inline]
    #[must_use]
    pub const fn num_workers(&self) -> usize {
        self.threads.len()
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.shutdown();
    }
}
