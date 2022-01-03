use chrono::{DateTime, Utc};

use crate::config::ExtraOpts;

use anyhow::{bail, Context, Result};
use prometheus::core::Number;
use serde_json::Value;

pub(crate) fn network_functional<N: Number>(addr: &str, _opts: &Option<ExtraOpts>) -> Result<N> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_network_functional() {
        assert!(
            network_functional::<u64>("https://prod-mainnet.prod.findora.org:26657", &None).is_ok()
        )
    }
}
