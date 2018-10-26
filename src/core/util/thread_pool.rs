use std::thread;
use std::sync::Arc;
use std::sync::mpsc::{self, Sender, SyncSender, Receiver};
use std::collections::LinkedList;

//TODO add tests

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

    pub fn enqueue(&self, payload: Payload) {
        self.queue_tx.send(Signal::JOB(payload));
    }

    pub fn terminate_and_join(&self) {
        self.queue_tx.send(Signal::TERM);
        self.term_rx.recv();
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

            term_tx.send(());
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
                mux_tx.send(WorkerReport {
                    id,
                    status: WorkerStatus::IDLE,
                });

                let sig = Worker::join_job(&rx, id);
                match sig {
                    Signal::TERM => break,
                    Signal::JOB(payload) => {
                        job_runner(payload);
                    }
                }
            }

            mux_tx.send(WorkerReport {
                id,
                status: WorkerStatus::TERM,
            });
        });

        Worker { tx }
    }

    fn run_job(&self, payload: Payload) {
        self.tx.send(Signal::JOB(payload));
    }

    fn terminate(&self) {
        self.tx.send(Signal::TERM);
    }

    fn join_job(rx: &Receiver<Signal<Payload>>, id: WorkerId) -> Signal<Payload> {
        match rx.recv() {
            Err(err) => panic!("Mux rx error on worker thread {}: {}", id, err),
            Ok(sig) => sig
        }
    }
}

enum Signal<Payload: 'static + Send> {
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

    #[test]
    fn single_worker_fast_main() {
        //setup
        let counter = Arc::new(AtomicUsize::new(0));

        let pool = ThreadPool::spawn(1, 10, |(payload, counter): (usize, Arc<AtomicUsize>)| {
            assert_eq!(counter.fetch_add(1, Ordering::SeqCst), payload);
        });

        //exercise/verify
        for i in 0..10 {
            pool.enqueue((i, counter.clone()));
        }

        assert_eq!(counter.load(Ordering::Relaxed), 0);

        pool.terminate_and_join();

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
            pool.enqueue((i, counter.clone()));
            thread::sleep(Duration::new(0, 10))
        }

        assert_eq!(counter.load(Ordering::Relaxed), 10);

        pool.terminate_and_join();

        assert_eq!(counter.load(Ordering::Relaxed), 10);
    }

    #[test]
    fn many_workers() {
        //setup
        let counter = Arc::new(AtomicUsize::new(0));

        let pool = ThreadPool::spawn(
            4,
            16,
            |(payload, counter): (usize, Arc<AtomicUsize>)| {
                thread::sleep(Duration::new(0, 100000000));
                counter.fetch_add(1, Ordering::SeqCst);
            });

        //exercise
        for i in 0..16 {
            pool.enqueue((i, counter.clone()));
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

        pool.terminate_and_join();

        assert_eq!(counter.load(Ordering::Relaxed), 16);
    }

    #[test]
    fn terminate_and_join() {
        //setup
        let counter = Arc::new(AtomicUsize::new(0));

        let pool = ThreadPool::spawn(
            4,
            16,
            |(payload, counter): (usize, Arc<AtomicUsize>)| {
                thread::sleep(Duration::new(0, 1000));
                counter.fetch_add(1, Ordering::SeqCst);
            });

        //exercise
        for i in 0..16 {
            pool.enqueue((i, counter.clone()));
        }

        assert_eq!(counter.load(Ordering::Relaxed), 0);

        //verify
        pool.terminate_and_join();
        assert_eq!(counter.load(Ordering::Relaxed), 16);
    }

    #[test]
    fn sync_sender_throttle() {
        //setup
        let counter = Arc::new(AtomicUsize::new(0));

        let pool = ThreadPool::spawn(
            4,
            4,
            |(payload, counter): (usize, Arc<AtomicUsize>)| {
                thread::sleep(Duration::new(0, 1000000));
                counter.fetch_add(1, Ordering::SeqCst);
            });

        //exercise
        for i in 0..100 {
            pool.enqueue((i, counter.clone()));
        }

        //verify
        let result = counter.load(Ordering::SeqCst);
        assert!(result > 90 && result < 96);

        pool.terminate_and_join();
        assert_eq!(counter.load(Ordering::Relaxed), 100);
    }

    #[test]
    fn sync_sender_large_buffer() {
        //setup
        let counter = Arc::new(AtomicUsize::new(0));

        let pool = ThreadPool::spawn(
            4,
            100,
            |(payload, counter): (usize, Arc<AtomicUsize>)| {
                thread::sleep(Duration::new(0, 1000000));
                counter.fetch_add(1, Ordering::SeqCst);
            });

        //exercise
        for i in 0..100 {
            pool.enqueue((i, counter.clone()));
        }

        //verify
        assert_eq!(counter.load(Ordering::Relaxed), 0);

        pool.terminate_and_join();
        assert_eq!(counter.load(Ordering::Relaxed), 100);
    }
}
