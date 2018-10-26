use std::thread;
use std::sync::Arc;
use std::sync::mpsc::{self, Sender, SyncSender, Receiver};
use std::collections::LinkedList;
use std::fmt;
use std::error;

pub struct ThreadPool<Payload: 'static + Send> {
    queue_tx: SyncSender<Signal<Payload>>,
    term_rx: Receiver<()>,
}

impl<Payload: 'static + Send> ThreadPool<Payload> {
    pub fn spawn<JobRunner>(size: usize, queue_size: usize, job_runner: JobRunner) -> ThreadPool<Payload>
        where JobRunner: Fn(Payload) + 'static + Send + Sync
    {
        let (queue_tx, queue_rx) = mpsc::sync_channel(queue_size);
        let (term_tx, term_rx) = mpsc::channel();

        WorkerMux::spawn(size, job_runner, queue_rx, term_tx);

        ThreadPool {
            queue_tx,
            term_rx,
        }
    }

    pub fn enqueue(&self, payload: Payload) -> Result<(), ThreadPoolError> {
        match self.queue_tx.send(Signal::JOB(payload)) {
            Err(err) => Err(ThreadPoolError::QueueErr(err.to_string())),
            Ok(()) => Ok(())
        }
    }

    pub fn terminate_and_join(&self) -> Result<(), ThreadPoolError> {
        match self.queue_tx.send(Signal::TERM) {
            Err(err) => Err(ThreadPoolError::TermError(err.to_string())),
            Ok(()) => match self.term_rx.recv() {
                Err(err) => panic!("Worker mux term tx closed before termination signal: {}", err),
                Ok(()) => Ok(())
            }
        }
    }
}

#[derive(Debug)]
pub enum ThreadPoolError {
    QueueErr(String),
    TermError(String),
}

impl fmt::Display for ThreadPoolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ThreadPoolError::QueueErr(ref err) =>
                write!(f, "Error enqueuing worker job: {}", err),
            ThreadPoolError::TermError(ref err) =>
                write!(f, "Error enqueuing worker termination signal: {}", err),
        }
    }
}

impl error::Error for ThreadPoolError {
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            ThreadPoolError::QueueErr(_) => None,
            ThreadPoolError::TermError(_) => None,
        }
    }
}

struct WorkerMux {}

//TODO use helper objects to reduce number of parameters
impl WorkerMux {
    fn spawn<JobRunner, Payload: 'static + Send>(
        size: usize,
        job_runner: JobRunner,
        queue_rx: Receiver<Signal<Payload>>,
        term_tx: Sender<()>,
    ) -> WorkerMux
        where JobRunner: Fn(Payload) + 'static + Send + Sync
    {
        let job_runner_arc: Arc<JobRunner> = Arc::new(job_runner);

        let (mux_tx, mux_rx) = mpsc::channel();

        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::spawn(id, mux_tx.clone(), job_runner_arc.clone()));
        }

        thread::spawn(move || {
            let mut idle_workers: LinkedList<WorkerId> = LinkedList::new();

            loop {
                while idle_workers.is_empty() {
                    let report = WorkerMux::join_worker_report(&mux_rx);
                    match report.status {
                        WorkerStatus::IDLE => idle_workers.push_back(report.id),
                        WorkerStatus::TERM => {}
                    }
                }

                let sig = WorkerMux::join_job_queue(&queue_rx);
                match sig {
                    Signal::TERM => break,
                    Signal::JOB(payload) => {
                        let worker_id = idle_workers.pop_back().unwrap();
                        workers.get(worker_id).unwrap().run_job(payload);
                    }
                }
            }

            for worker in &workers {
                worker.terminate();
            }

            let mut terminated_workers = 0;
            while terminated_workers < size {
                match WorkerMux::join_worker_report(&mux_rx).status {
                    WorkerStatus::IDLE => {}
                    WorkerStatus::TERM => terminated_workers += 1
                }
            }

            term_tx.send(()).unwrap();
        });

        WorkerMux {}
    }

    fn join_worker_report(mux_rx: &Receiver<WorkerReport>) -> WorkerReport {
        match mux_rx.recv() {
            Err(err) => panic!("Worker rx error on threadpool worker mux: {}", err),
            Ok(report) => report
        }
    }

    fn join_job_queue<Payload: 'static + Send>(queue_rx: &Receiver<Signal<Payload>>) -> Signal<Payload> {
        match queue_rx.recv() {
            Err(err) => panic!("Job queue rx error on threadpool worker mux: {}", err),
            Ok(sig) => sig
        }
    }
}

struct Worker<Payload: 'static + Send> {
    tx: Sender<Signal<Payload>>
}

impl<Payload: 'static + Send> Worker<Payload> {
    fn spawn<JobRunner>(id: WorkerId, mux_tx: Sender<WorkerReport>, job_runner: Arc<JobRunner>) -> Worker<Payload>
        where JobRunner: Fn(Payload) + 'static + Send + Sync
    {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            loop {
                let report = WorkerReport {
                    id,
                    status: WorkerStatus::IDLE,
                };
                mux_tx.send(report).unwrap();

                let sig = Worker::join_job(&rx, id);
                match sig {
                    Signal::TERM => break,
                    Signal::JOB(payload) => {
                        job_runner(payload);
                    }
                }
            }

            let report = WorkerReport {
                id,
                status: WorkerStatus::TERM,
            };
            mux_tx.send(report).unwrap();
        });

        Worker { tx }
    }

    fn run_job(&self, payload: Payload) {
        self.tx.send(Signal::JOB(payload)).unwrap();
    }

    fn terminate(&self) {
        self.tx.send(Signal::TERM).unwrap();
    }

    fn join_job(rx: &Receiver<Signal<Payload>>, id: WorkerId) -> Signal<Payload> {
        match rx.recv() {
            Err(err) => panic!("Mux rx error on worker thread {}: {}", id, err),
            Ok(sig) => sig
        }
    }
}

#[derive(Debug)]
pub enum Signal<Payload: 'static + Send> {
    TERM,
    JOB(Payload),
}

struct WorkerReport {
    id: WorkerId,
    status: WorkerStatus,
}

type WorkerId = usize;

enum WorkerStatus {
    TERM,
    IDLE,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use std::error::Error;

    #[test]
    fn single_worker_fast_main() {
        //setup
        let counter = Arc::new(AtomicUsize::new(0));

        let pool = ThreadPool::spawn(1, 10, |(payload, counter): (usize, Arc<AtomicUsize>)| {
            assert_eq!(counter.fetch_add(1, Ordering::SeqCst), payload);
        });

        //exercise/verify
        for i in 0..10 {
            pool.enqueue((i, counter.clone())).unwrap();
        }

        assert_eq!(counter.load(Ordering::Relaxed), 0);

        pool.terminate_and_join().unwrap();

        assert_eq!(counter.load(Ordering::Relaxed), 10);
    }

    #[test]
    fn single_worker_slow_main() {
        //setup
        let counter = Arc::new(AtomicUsize::new(0));

        let pool = ThreadPool::spawn(1, 10, |(payload, counter): (usize, Arc<AtomicUsize>)| {
            assert_eq!(counter.fetch_add(1, Ordering::SeqCst), payload);
        });

        //exercise/verify
        for i in 0..10 {
            pool.enqueue((i, counter.clone())).unwrap();
            thread::sleep(Duration::new(0, 1000000))
        }

        assert!(counter.load(Ordering::Relaxed) > 0);

        pool.terminate_and_join().unwrap();

        assert_eq!(counter.load(Ordering::Relaxed), 10);
    }

    #[test]
    fn many_workers() {
        //setup
        let counter = Arc::new(AtomicUsize::new(0));

        let pool = ThreadPool::spawn(
            4,
            16,
            |(_, counter): (usize, Arc<AtomicUsize>)| {
                thread::sleep(Duration::new(0, 100000000));
                counter.fetch_add(1, Ordering::SeqCst);
            });

        //exercise
        for i in 0..16 {
            pool.enqueue((i, counter.clone())).unwrap();
        }

        //verify
        assert_eq!(counter.load(Ordering::Relaxed), 0);

        thread::sleep(Duration::new(0, 120000000));
        assert_eq!(counter.load(Ordering::Relaxed), 4);

        thread::sleep(Duration::new(0, 120000000));
        assert_eq!(counter.load(Ordering::Relaxed), 8);

        thread::sleep(Duration::new(0, 120000000));
        assert_eq!(counter.load(Ordering::Relaxed), 12);

        thread::sleep(Duration::new(0, 120000000));
        assert_eq!(counter.load(Ordering::Relaxed), 16);

        pool.terminate_and_join().unwrap();

        assert_eq!(counter.load(Ordering::Relaxed), 16);
    }

    #[test]
    fn terminate_and_join() {
        //setup
        let counter = Arc::new(AtomicUsize::new(0));

        let pool = ThreadPool::spawn(
            4,
            16,
            |(_, counter): (usize, Arc<AtomicUsize>)| {
                thread::sleep(Duration::new(0, 1000));
                counter.fetch_add(1, Ordering::SeqCst);
            });

        //exercise
        for i in 0..16 {
            pool.enqueue((i, counter.clone())).unwrap();
        }

        assert_eq!(counter.load(Ordering::Relaxed), 0);

        //verify
        pool.terminate_and_join().unwrap();
        assert_eq!(counter.load(Ordering::Relaxed), 16);
    }

    #[test]
    fn sync_sender_throttle() {
        //setup
        let counter = Arc::new(AtomicUsize::new(0));

        let pool = ThreadPool::spawn(
            4,
            4,
            |(_, counter): (usize, Arc<AtomicUsize>)| {
                thread::sleep(Duration::new(0, 1000000));
                counter.fetch_add(1, Ordering::SeqCst);
            });

        //exercise
        for i in 0..100 {
            pool.enqueue((i, counter.clone())).unwrap();
        }

        //verify
        let result = counter.load(Ordering::SeqCst);
        assert!(result > 90 && result < 96);

        pool.terminate_and_join().unwrap();
        assert_eq!(counter.load(Ordering::Relaxed), 100);
    }

    #[test]
    fn sync_sender_large_buffer() {
        //setup
        let counter = Arc::new(AtomicUsize::new(0));

        let pool = ThreadPool::spawn(
            4,
            100,
            |(_, counter): (usize, Arc<AtomicUsize>)| {
                thread::sleep(Duration::new(0, 1000000));
                counter.fetch_add(1, Ordering::SeqCst);
            });

        //exercise
        for i in 0..100 {
            pool.enqueue((i, counter.clone())).unwrap();
        }

        //verify
        assert_eq!(counter.load(Ordering::Relaxed), 0);

        pool.terminate_and_join().unwrap();
        assert_eq!(counter.load(Ordering::Relaxed), 100);
    }

    #[test]
    fn queue_error() {
        //setup
        let pool = ThreadPool::spawn(1, 1, |_: usize| {});
        pool.terminate_and_join().unwrap();
        thread::sleep(Duration::new(0, 1000000));

        //exercise
        let result = pool.enqueue(1);

        //verify
        let err = result.err().unwrap();
        assert_eq!(format!("{}", err), "Error enqueuing worker job: sending on a closed channel");

        assert!(err.cause().is_none());
    }

    #[test]
    fn termination_error() {
        //setup
        let pool = ThreadPool::spawn(1, 1, |_: usize| {});
        pool.terminate_and_join().unwrap();
        thread::sleep(Duration::new(0, 1000000));

        //exercise
        let result = pool.terminate_and_join();

        //verify
        let err = result.err().unwrap();
        assert_eq!(format!("{}", err), "Error enqueuing worker termination signal: sending on a closed channel");

        assert!(err.cause().is_none());
    }
}
