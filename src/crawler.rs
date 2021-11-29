use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    time::Duration,
    {thread, thread::JoinHandle},
};

use anyhow::{bail, Context, Result};
use log::error;
use serde_json::Value;

pub(crate) struct Crawler {
    workers: Vec<Arc<RwLock<Worker>>>,
}

impl Crawler {
    pub(crate) fn new(cfg: &crate::config::Crawler) -> Self {
        let mut workers = vec![];
        for target in &cfg.targets {
            workers.push(Arc::new(RwLock::new(Worker::new(target))));
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
}

impl Worker {
    fn new(cfg: &crate::config::CrawlingTarget) -> Self {
        Worker {
            addr: cfg.host_addr.clone(),
            freq: Duration::from_millis(cfg.frequency_ms),
            jobs: Vec::with_capacity(3),
            done: Arc::new(AtomicBool::new(false)),
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

        self.jobs.push(Some(thread::spawn(move || {
            while !done.load(Ordering::SeqCst) {
                thread::sleep(freq);
                match get_consensus_power(&addr) {
                    Ok(v) => {
                        crate::CONSENSUS_POWER.set(v);
                    }
                    Err(e) => {
                        error!("get_consensus_power failed: {:?}", e)
                    }
                }
            }
        })));
    }
}

fn get_consensus_power(addr: &str) -> Result<f64> {
    let data: Value = ureq::get(&format!("{}/dump_consensus_state", addr))
        .call()
        .context("ureq call failed")?
        .into_json()
        .context("ureq json failed")?;

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

    Ok(power * 100f64)
}
