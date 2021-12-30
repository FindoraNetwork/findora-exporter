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

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use log::error;
use prometheus::core::{Atomic, Number};
use serde_json::Value;

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
                TaskName::ConsensusPower => Task::new("get_consensus_power", get_consensus_power),
                TaskName::NetworkFunctional => {
                    Task::new("get_network_functional", get_network_functional)
                }
                TaskName::TotalCountOfValidators => {
                    Task::new("get_total_validators", get_total_validators)
                }
                TaskName::TotalBalanceOfRelayers => {
                    Task::new("get_relayer_balance", get_relayer_balance)
                }
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

fn get_consensus_power<N: Number>(addr: &str, _opts: &Option<ExtraOpts>) -> Result<N> {
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

    let power: i64 = power
        .parse()
        .with_context(|| format!("power:{} convert to i64 failed", power))?;

    Ok(N::from_i64(power * 100))
}

fn get_network_functional<N: Number>(addr: &str, _opts: &Option<ExtraOpts>) -> Result<N> {
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

    Ok(N::from_i64((cur_timestamp - latest_block_timestamp).abs()))
}

fn get_total_validators<N: Number>(addr: &str, _opts: &Option<ExtraOpts>) -> Result<N> {
    let data: Value = ureq::get(&format!("{}/validators", addr))
        .call()
        .context("get_total_validators ureq call failed")?
        .into_json()
        .context("get_total_validators ureq json failed")?;

    let total_validators = &data["result"]["total"];
    if total_validators.is_null() {
        bail!("total_validators is null")
    }

    let total_validators = match total_validators.as_str() {
        Some(v) => v.to_string(),
        None => bail!("total_validators is not a str"),
    };

    let total_validators: i64 = total_validators.parse().with_context(|| {
        format!(
            "total_validators:{} convert to i64 failed",
            total_validators
        )
    })?;

    Ok(N::from_i64(total_validators))
}

fn get_relayer_balance<N: Number>(addr: &str, opts: &Option<ExtraOpts>) -> Result<N> {
    // asking the bridge for the count of relayers
    let bridge_addr = match opts {
        Some(o) => {
            let ExtraOpts::BridgeAddress(b) = o;
            b
        }
        None => bail!("get_relayer_balance cannot get bridge address"),
    };
    let count: Value = ureq::post(addr)
         .send_json(ureq::json!({
             "method":"eth_call",
             "jsonrpc":"2.0",
             "id":0,
             "params":[
                 {
                     "data":"0xca15c873e2b7fb3b832174769106daebcfd6d1970523240dda11281102db9363b83b0dc4",
                     "to":bridge_addr
                 },
                 "latest"
             ],
         }))?
         .into_json()?;

    println!("########: {}", count);
    Ok(N::from_i64(0))
}

// As bridge for releyer count
// curl -X POST "https://dev-qa01.dev.findora.org:8545" -H "Content-Type: application/json" --data '[
// {"method":"eth_call", "jsonrpc":"2.0", "id": 2, "params":[{"data":"0xca15c873e2b7fb3b832174769106daebcfd6d1970523240dda11281102db9363b83b0dc4", "to":"0x26925046a09d9AEfe6903eae0aD090be06186Bd9"},"latest"]}
// ]'
// [{"jsonrpc":"2.0","result":"0x0000000000000000000000000000000000000000000000000000000000000003","id":2}]
//
// Ask Bridge for releyers address
// curl -X POST "https://dev-qa01.dev.findora.org:8545" -H "Content-Type: application/json" --data
// '[
//      {"method":"eth_call", "jsonrpc":"2.0", "id": 1, "params":[{"data":"0x9010d07ce2b7fb3b832174769106daebcfd6d1970523240dda11281102db9363b83b0dc40000000000000000000000000000000000000000000000000000000000000000", "to":"0x26925046a09d9AEfe6903eae0aD090be06186Bd9"}, "latest"]},
//      {"method":"eth_call", "jsonrpc":"2.0", "id": 2, "params":[{"data":"0x9010d07ce2b7fb3b832174769106daebcfd6d1970523240dda11281102db9363b83b0dc40000000000000000000000000000000000000000000000000000000000000001", "to":"0x26925046a09d9AEfe6903eae0aD090be06186Bd9"}, "latest"]},
//      {"method":"eth_call", "jsonrpc":"2.0", "id": 3, "params":[{"data":"0x9010d07ce2b7fb3b832174769106daebcfd6d1970523240dda11281102db9363b83b0dc40000000000000000000000000000000000000000000000000000000000000002", "to":"0x26925046a09d9AEfe6903eae0aD090be06186Bd9"}, "latest"]}
//  ]'
//
// [{"jsonrpc":"2.0","result":"0x0000000000000000000000002bae5160a67ffe0d2dd9114c521dd51689fdb549","id":2}]
//
// Ask for the balances
// curl -X POST "https://dev-qa01.dev.findora.org:8545" -H "Content-Type: application/json" --data '[{"method":"eth_getBalance", "jsonrpc":"2.0", "id": 1, "params":["0x2bae5160a67ffe0d2dd9114c521dd51689fdb549", "latest"]}]'
// [{"jsonrpc":"2.0","result":"0x8aa21e312be77000","id":1}]
