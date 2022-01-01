use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    time::Duration,
    {thread, thread::JoinHandle},
};

use crate::{
    config::{ExtraOpts, TaskName},
    utils::calculate_hash,
};

use anyhow::{Context, Result};
use log::error;
use prometheus::core::Atomic;

/// A collection of Workers for managing easily.
pub(crate) struct Crawler<T: Atomic> {
    workers: Vec<Arc<RwLock<Worker<T>>>>,
}

impl<T> Crawler<T>
where
    T: Atomic + 'static,
{
    /// Returns a Crawler instance.
    ///
    /// This new method will not execute anything but only returns a Crawler instance.
    /// Every Target in the config structure will be applied to a Worker structure.
    pub(crate) fn new(
        cfg: &crate::config::Crawler,
        metrics: Arc<crate::metrics::Metrics<T>>,
    ) -> Self {
        let mut workers = vec![];
        for target in &cfg.targets {
            let metric = metrics
                .get_metric(calculate_hash(target))
                .expect("get_metric failed");

            let task = match target.task_name {
                TaskName::ConsensusPower => {
                    Task::new("consensus_power", crate::tasks::consensus_power)
                }
                TaskName::NetworkFunctional => {
                    Task::new("network_functional", crate::tasks::network_functional)
                }
                TaskName::TotalCountOfValidators => Task::new(
                    "total_count_of_validators",
                    crate::tasks::total_count_of_validators,
                ),
                TaskName::TotalBalanceOfRelayers => Task::new(
                    "get_relayer_balance",
                    crate::tasks::total_balance_of_relayers,
                ),
            };

            workers.push(Arc::new(RwLock::new(Worker::new(target, metric, task))));
        }

        Crawler { workers }
    }

    /// Signaling workers to stop working.
    pub(crate) fn close(&self) {
        for worker in &self.workers {
            worker.write().unwrap().close();
        }
    }

    /// Spawned a thread to start running each worker.
    pub(crate) fn run(&self) -> Result<JoinHandle<()>> {
        let workers = self.workers.clone();
        thread::Builder::new()
            .name("crawler_thread".into())
            .spawn(move || {
                for worker in &workers {
                    worker.write().unwrap().run();
                }
            })
            .context("crawler thread run failed")
    }
}

struct Worker<T: Atomic> {
    addr: String,
    extra_opts: Option<ExtraOpts>,
    freq: Duration,
    task: Arc<Task<T>>,
    task_thread: Option<thread::JoinHandle<()>>,
    done: Arc<AtomicBool>,
    metric: Arc<crate::metrics::Metric<T>>,
}

impl<T> Worker<T>
where
    T: Atomic + 'static,
{
    fn new(
        cfg: &crate::config::Target,
        metric: Arc<crate::metrics::Metric<T>>,
        task: Task<T>,
    ) -> Self {
        Worker {
            addr: cfg.host_addr.clone(),
            extra_opts: cfg.extra_opts.clone(),
            freq: Duration::from_millis(cfg.frequency_ms),
            task: Arc::new(task),
            done: Arc::new(AtomicBool::new(false)),
            metric,
            task_thread: None,
        }
    }

    fn close(&mut self) {
        self.done.store(true, Ordering::SeqCst);
        if let Some(t) = self.task_thread.take() {
            let _ = t.join();
        }
    }

    fn run(&mut self) {
        let addr = self.addr.clone();
        let done = self.done.clone();
        let freq = self.freq;
        let metric = self.metric.clone();
        let task = self.task.clone();
        let extra_opts = self.extra_opts.clone();

        self.task_thread = Some(thread::spawn(move || {
            while !done.load(Ordering::SeqCst) {
                match (task.f)(&addr, &extra_opts) {
                    Ok(v) => metric.set(v),
                    Err(e) => error!("{} failed: {:?}", task.name, e),
                }
                thread::sleep(freq);
            }
        }))
    }
}

struct Task<T: Atomic> {
    name: &'static str,
    f: fn(&str, &Option<ExtraOpts>) -> Result<<T as Atomic>::T>,
}

impl<T> Task<T>
where
    T: Atomic,
{
    fn new(
        name: &'static str,
        f: fn(&str, &Option<ExtraOpts>) -> Result<<T as Atomic>::T>,
    ) -> Self {
        Task { name, f }
    }
}
