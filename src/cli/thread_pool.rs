use std::thread;
use std::sync::Arc;
use std::sync::mpsc::{self, Sender, Receiver};
use std::collections::LinkedList;

//TODO move this into the library code?
//TODO add tests

pub struct ThreadPool<Payload: 'static + Send> {
    mux: WorkerMux,
    queue_tx: Sender<Signal<Payload>>,
    term_rx: Receiver<()>
}

impl<Payload: 'static + Send> ThreadPool<Payload> {
    pub fn new<JobRunner>(size: usize, job_runner: JobRunner) -> ThreadPool<Payload>
        where JobRunner: Fn(Payload) + 'static + Send + Sync
    {
        let (queue_tx, queue_rx): (Sender<Signal<Payload>>, Receiver<Signal<Payload>>) = mpsc::channel();
        let (term_tx, term_rx): (Sender<()>, Receiver<()>) = mpsc::channel();

        ThreadPool {
            mux: WorkerMux::new(size, job_runner, queue_rx, term_tx),
            queue_tx,
            term_rx
        }
    }

    pub fn enqueue(&self, payload: Payload){
        self.queue_tx.send(Signal::JOB(payload));
    }

    pub fn terminate_and_join(&self){
        self.queue_tx.send(Signal::TERM);
        self.term_rx.recv();
    }
}

struct WorkerMux {}

//TODO use helper objects to reduce number of parameters
impl WorkerMux {
    fn new<JobRunner, Payload: 'static + Send>(size: usize, job_runner: JobRunner, queue_rx: Receiver<Signal<Payload>>, term_tx: Sender<()>) -> WorkerMux
        where JobRunner: Fn(Payload) + 'static + Send + Sync
    {
        let job_runner_arc: Arc<JobRunner> = Arc::new(job_runner);

        let (mux_tx, mux_rx): (Sender<WorkerReport>, Receiver<WorkerReport>) = mpsc::channel();

        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, mux_tx.clone(), job_runner_arc.clone()));
        }

        let thread = thread::spawn(move || {
            let mut idle_workers: LinkedList<WorkerId> = LinkedList::new();

            loop {
                while idle_workers.is_empty() {
                    match mux_rx.recv() {
                        Err(err) => println!("MUX_RX ERROR ON WORKER MUX: {}", err),
                        Ok(report) => match report.status {
                            WorkerStatus::IDLE => idle_workers.push_back(report.id),
                            WorkerStatus::TERM => {}
                        }
                    }
                }

                match queue_rx.recv() {
                    Err(err) => println!("QUEUE_RX ERROR ON WORKER MUX: {}", err),
                    Ok(sig) => match sig {
                        Signal::TERM => break,
                        Signal::JOB(payload) => {
                            let worker_id = idle_workers.pop_back().unwrap();
                            workers.get(worker_id).unwrap().run_job(payload);
                        }
                    }
                }
            }

            for worker in &workers {
                worker.terminate();
            }

            let mut terminated_workers = 0;
            while terminated_workers < size {
                match mux_rx.recv() {
                    Err(err) => println!("MUX_RX ERROR ON WORKER MUX: {}", err),
                    Ok(report) => match report.status {
                        WorkerStatus::IDLE => {},
                        WorkerStatus::TERM => terminated_workers += 1
                    }
                }
            }

            term_tx.send(());
        });

        WorkerMux {}
    }
}

struct Worker<Payload: 'static + Send> {
    id: WorkerId,
    tx: Sender<Signal<Payload>>,
    thread: thread::JoinHandle<()>,
}

impl<Payload: 'static + Send> Worker<Payload> {
    fn new<JobRunner>(id: WorkerId, mux_tx: Sender<WorkerReport>, job_runner: Arc<JobRunner>) -> Worker<Payload>
        where JobRunner: Fn(Payload) + 'static + Send + Sync
    {
        let (tx, rx): (Sender<Signal<Payload>>, Receiver<Signal<Payload>>) = mpsc::channel();

        let thread = thread::spawn(move || {
            loop {
                mux_tx.send(WorkerReport{
                    id,
                    status: WorkerStatus::IDLE
                });

                match rx.recv() {
                    Err(err) => println!("RX ERROR ON WORKER {}: {}", id, err),
                    Ok(sig) => match sig {
                        Signal::TERM => break,
                        Signal::JOB(payload) => {
                            job_runner(payload);
                        }
                    }
                }
            }

            mux_tx.send(WorkerReport{
                id,
                status: WorkerStatus::TERM
            });
        });

        Worker {
            id,
            tx,
            thread,
        }
    }

    fn run_job(&self, payload: Payload) {
        self.tx.send(Signal::JOB(payload));
    }

    fn terminate(&self) {
        self.tx.send(Signal::TERM);
    }
}

enum Signal<Payload: 'static + Send> {
    TERM,
    JOB(Payload),
}

struct WorkerReport {
    id: WorkerId,
    status: WorkerStatus
}

type WorkerId = usize;

enum WorkerStatus {
    TERM,
    IDLE
}
