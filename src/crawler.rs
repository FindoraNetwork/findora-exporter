use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::channel,
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use crate::{
    config::{ExtraOpts, TaskName},
    metrics::Metric,
    utils::calculate_hash,
};

use anyhow::{Context, Result};
use log::error;
use prometheus::core::Atomic;

/// A collection of Workers for managing easily.
pub(crate) struct Crawler {
    workers: Vec<Option<thread::JoinHandle<()>>>,
    done: Arc<AtomicBool>,
}

impl Crawler {
    /// Returns a Crawler instance and
    /// Spawned
    /// 1. a thread to push tasks in the mpsc queue for workers to consume.
    /// 2. N threads of worker to consume tasks.
    pub(crate) fn new<T>(
        cfg: &crate::config::Crawler,
        metrics: Arc<crate::metrics::Metrics<T>>,
    ) -> Result<Self>
    where
        T: Atomic + 'static,
    {
        let mut workers = Vec::with_capacity(cfg.worker_n + 1);
        let mut tasks = Vec::with_capacity(cfg.targets.len());
        let done = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();
        let rx = Arc::new(Mutex::new(rx));

        for target in &cfg.targets {
            let metric = metrics
                .get_metric(calculate_hash(&target))
                .expect("get_metric failed");

            let task = match target.task_name {
                TaskName::ConsensusPower => Task::new(
                    "consensus_power".to_string(),
                    target.host_addr.clone(),
                    metric,
                    target.extra_opts.clone(),
                    crate::tasks::consensus_power,
                ),
                TaskName::NetworkFunctional => Task::new(
                    "network_functional".to_string(),
                    target.host_addr.clone(),
                    metric,
                    target.extra_opts.clone(),
                    crate::tasks::network_functional,
                ),
                TaskName::TotalCountOfValidators => Task::new(
                    "total_count_of_validators".to_string(),
                    target.host_addr.clone(),
                    metric,
                    target.extra_opts.clone(),
                    crate::tasks::total_count_of_validators,
                ),
                TaskName::TotalBalanceOfRelayers => Task::new(
                    "total_balance_of_relayers".to_string(),
                    target.host_addr.clone(),
                    metric,
                    target.extra_opts.clone(),
                    crate::tasks::total_balance_of_relayers,
                ),
                TaskName::BridgedBalance => Task::new(
                    "bridged_balance".to_string(),
                    target.host_addr.clone(),
                    metric,
                    target.extra_opts.clone(),
                    crate::tasks::bridged_balance,
                ),
                TaskName::BridgedSupply => Task::new(
                    "bridged_supply".to_string(),
                    target.host_addr.clone(),
                    metric,
                    target.extra_opts.clone(),
                    crate::tasks::bridged_supply,
                ),
            };

            tasks.push(Arc::new(task));
        }

        let freq = Duration::from_millis(cfg.frequency_ms);
        let pusher_done = done.clone();
        workers.push(Some(
            thread::Builder::new()
                .name("task pusher".to_string())
                .spawn(move || {
                    while !pusher_done.load(Ordering::SeqCst) {
                        for task in tasks.clone() {
                            tx.send(task).unwrap();
                        }
                        thread::sleep(freq);
                    }
                })
                .context("spawning task pusher thread failed")?,
        ));

        for id in 0..cfg.worker_n {
            let rx = rx.clone();
            workers.push(Some(
                thread::Builder::new()
                    .name(format!("worker{}", id))
                    .spawn(move || {
                        rx.lock().unwrap().recv().unwrap().execute();
                    })
                    .context("spawning worker thread failed")?,
            ));
        }

        Ok(Crawler { workers, done })
    }

    /// Signaling workers to stop working.
    pub(crate) fn close(&mut self) {
        self.done.store(true, Ordering::SeqCst);

        for worker in self.workers.iter_mut() {
            if let Some(w) = worker.take() {
                let _ = w.join();
            }
        }
    }
}

#[derive(Clone)]
struct Task<T: Atomic> {
    name: String,
    addr: String,
    metric: Arc<Metric<T>>,
    option: Option<ExtraOpts>,
    f: fn(&str, &Option<ExtraOpts>) -> Result<<T as Atomic>::T>,
}

impl<T> Task<T>
where
    T: Atomic + 'static,
{
    fn new(
        name: String,
        addr: String,
        metric: Arc<Metric<T>>,
        option: Option<ExtraOpts>,
        f: fn(&str, &Option<ExtraOpts>) -> Result<<T as Atomic>::T>,
    ) -> Self {
        Task {
            name,
            addr,
            metric,
            option,
            f,
        }
    }

    fn execute(&self) {
        match (self.f)(&self.addr, &self.option) {
            Ok(v) => self.metric.set(v),
            Err(e) => error!(
                "task:{}, addr:{}, option:{:?}, err:{}",
                self.name, self.addr, self.option, e
            ),
        }
    }
}
