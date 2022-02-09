use crate::config::ExtraOpts;

use anyhow::{bail, Context, Result};
use prometheus::core::Number;
use serde_json::Value;

pub(crate) fn consensus_power<N: Number>(addr: &str, _opts: &Option<ExtraOpts>) -> Result<N> {
    let data: Value = ureq::get(&format!("{}/dump_consensus_state", addr))
        .call()
        .with_context(|| format!("ureq call failed, addr:{:?}", addr))?
        .into_json()
        .with_context(|| format!("ureq json failed, addr:{:?}", addr))?;

    let power = &data["result"]["round_state"]["last_commit"]["votes_bit_array"];
    if power.is_null() {
        bail!("power is null, addr:{:?}", addr)
    }

    let power = match power.as_str() {
        Some(v) => v.to_string(),
        None => bail!("power is not a str, addr:{:?}", addr),
    };

    let power = match power.rfind('=') {
        Some(pos) => {
            let n = power.len();
            if pos + 2 >= n - 1 {
                bail!(
                    "power cannot be parsed, pos:{}, n:{}, addr:{:?}",
                    pos,
                    n,
                    addr
                )
            }
            (&power[pos + 2..n - 1]).to_string()
        }
        None => bail!("power cannot find = symbol, addr:{:?}", addr),
    };

    let power: f64 = power
        .parse()
        .with_context(|| format!("power:{} convert to f64 failed, addr:{:?}", power, addr))?;

    Ok(N::from_i64((power * 100.0f64) as i64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_consensus_power() {
        assert!(
            consensus_power::<u64>("https://prod-mainnet.prod.findora.org:26657", &None).is_ok()
        )
    }
}
