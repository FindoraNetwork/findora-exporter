use crate::config::ExtraOpts;

use anyhow::{bail, Context, Result};
use prometheus::core::Number;
use serde_json::Value;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub(crate) fn network_functional<N: Number>(addr: &str, _opts: &Option<ExtraOpts>) -> Result<N> {
    let data: Value = ureq::get(&format!("{}/status", addr))
        .call()
        .with_context(|| format!("ureq call failed, addr:{:?}", addr))?
        .into_json()
        .with_context(|| format!("ureq json failed, addr:{:?}", addr))?;

    let latest_block_time = &data["result"]["sync_info"]["latest_block_time"];
    if latest_block_time.is_null() {
        bail!("latest_block_time is null, addr:{:?}", addr)
    }

    // format is like 2022-01-14T13:44:55.889015796Z UTC time
    let latest_block_time = match latest_block_time.as_str() {
        Some(v) => v,
        None => bail!("latest_block_time is not a str, addr:{:?}", addr),
    };

    let latest_block_timestamp = OffsetDateTime::parse(latest_block_time, &Rfc3339)
        .with_context(|| format!("parse latest_block_time failed, addr:{:?}", addr))?
        .unix_timestamp();
    let cur_timestamp = OffsetDateTime::now_utc().unix_timestamp();

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
