use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    time::Duration,
    {thread, thread::JoinHandle},
};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use log::error;
use serde_json::Value;

pub(crate) struct Crawler {
    workers: Vec<Arc<RwLock<Worker>>>,
}

impl Crawler {
    pub(crate) fn new(cfg: &crate::config::Crawler, metrics: Arc<crate::metrics::Metrics>) -> Self {
        let mut workers = vec![];
        for i in 0..cfg.targets.len() {
            workers.push(Arc::new(RwLock::new(Worker::new(
                &cfg.targets[i],
                metrics
                    .get_metric(i)
                    .expect("workers and metrics length should be equal"),
            ))));
        }
        Crawler { workers }
    }

    pub(crate) fn close(&self) {
        for worker in &self.workers {
            worker.write().unwrap().close();
        }
    }

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

struct Worker {
    addr: String,
    freq: Duration,
    jobs: Vec<Option<thread::JoinHandle<()>>>,
    done: Arc<AtomicBool>,
    metric: Arc<crate::metrics::Metric>,
}

impl Worker {
    fn new(cfg: &crate::config::Target, metric: Arc<crate::metrics::Metric>) -> Self {
        Worker {
            addr: cfg.host_addr.clone(),
            freq: Duration::from_millis(cfg.frequency_ms),
            jobs: Vec::with_capacity(3),
            done: Arc::new(AtomicBool::new(false)),
            metric,
        }
    }

    fn close(&mut self) {
        self.done.store(true, Ordering::SeqCst);
        for job in &mut self.jobs {
            if let Some(t) = job.take() {
                let _ = t.join();
            }
        }
    }

    fn run(&mut self) {
        let addr = self.addr.clone();
        let done = self.done.clone();
        let freq = self.freq;
        let metric = self.metric.clone();

        self.jobs.push(Some(thread::spawn(move || {
            let tasks = vec![
                Task::new("get_consensus_power", get_consensus_power),
                Task::new("get_network_functional", get_network_functional),
            ];

            while !done.load(Ordering::SeqCst) {
                thread::sleep(freq);
                for task in &tasks {
                    if let Err(e) = (task.f)(&addr, metric.clone()) {
                        error!("{} failed: {:?}", task.name, e);
                    }
                }
            }
        })));
    }
}

struct Task {
    name: &'static str,
    f: fn(&str, Arc<crate::metrics::Metric>) -> Result<()>,
}

impl Task {
    fn new(name: &'static str, f: fn(&str, Arc<crate::metrics::Metric>) -> Result<()>) -> Self {
        Task { name, f }
    }
}

fn get_consensus_power(addr: &str, metric: Arc<crate::metrics::Metric>) -> Result<()> {
    let data: Value = ureq::get(&format!("{}/dump_consensus_state", addr))
        .call()
        .context("get_consensus_power ureq call failed")?
        .into_json()
        .context("get_consensus_power ureq json failed")?;

    let power = &data["result"]["round_state"]["last_commit"]["votes_bit_array"];
    if power.is_null() {
        bail!("power is null")
    }

    let power = match power.as_str() {
        Some(v) => v.to_string(),
        None => bail!("power is not a str"),
    };

    let power = match power.rfind('=') {
        Some(pos) => {
            let n = power.len();
            if pos + 2 >= n - 1 {
                bail!("power cannot be parsed, pos:{}, n:{}", pos, n)
            }
            (&power[pos + 2..n - 1]).to_string()
        }
        None => bail!("power cannot find = symbol"),
    };

    let power: f64 = power
        .parse()
        .with_context(|| format!("power:{} convert to f64 failed", power))?;

    metric.set_consensus_power(power * 100f64);
    Ok(())
}

fn get_network_functional(addr: &str, metric: Arc<crate::metrics::Metric>) -> Result<()> {
    let data: Value = ureq::get(&format!("{}/status", addr))
        .call()
        .context("get_network_functional ureq call failed")?
        .into_json()
        .context("get_network_functional ureq json failed")?;

    let latest_block_time = &data["result"]["sync_info"]["latest_block_time"];
    if latest_block_time.is_null() {
        bail!("latest_block_time is null")
    }

    let latest_block_time = match latest_block_time.as_str() {
        Some(v) => v,
        None => bail!("latest_block_time is not a str"),
    };

    let latest_block_timestamp = DateTime::parse_from_rfc3339(latest_block_time)
        .context("parse latest_block_time failed")?
        .timestamp();
    let cur_timestamp = Utc::now().naive_utc().timestamp();

    metric.set_network_functional((cur_timestamp - latest_block_timestamp).abs());
    Ok(())
}
