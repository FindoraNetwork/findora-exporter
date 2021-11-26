use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use log::error;
use serde_json::Value;

pub(crate) struct Crawler {
    addr: String,
    jobs: Vec<Option<thread::JoinHandle<()>>>,
    done: Arc<AtomicBool>,
}

impl Crawler {
    pub(crate) fn new(addr: &str) -> Self {
        Crawler {
            addr: addr.to_string(),
            jobs: Vec::with_capacity(3),
            done: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn close(&mut self) {
        for job in &mut self.jobs {
            if let Some(t) = job.take() {
                let _ = t.join();
            }
        }
    }

    pub(crate) fn run(&mut self) {
        let addr = self.addr.clone();
        let done = self.done.clone();

        self.jobs.push(Some(thread::spawn(move || {
            while !done.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(500));
                match get_consensus_power(&addr) {
                    Ok(v) => {
                        println!("#############:{}", v);
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
