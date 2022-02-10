use crate::config::ExtraOpts;

use anyhow::{bail, Context, Result};

use prometheus::core::Number;
use serde_json::Value;

pub(crate) fn total_count_of_validators<N: Number>(
    addr: &str,
    _opts: &Option<ExtraOpts>,
) -> Result<N> {
    let data: Value = ureq::get(&format!("{}/validators", addr))
        .call()
        .with_context(|| format!("ureq call failed, addr:{:?}", addr))?
        .into_json()
        .with_context(|| format!("ureq json failed, addr:{:?}", addr))?;

    let total_validators = &data["result"]["total"];
    if total_validators.is_null() {
        bail!("total_validators is null, addr:{:?}", addr)
    }

    let total_validators = match total_validators.as_str() {
        Some(v) => v.to_string(),
        None => bail!("total_validators is not a str, addr:{:?}", addr),
    };

    let total_validators: i64 = total_validators.parse().with_context(|| {
        format!(
            "total_validators:{} convert to i64 failed, addr:{:?}",
            total_validators, addr,
        )
    })?;

    Ok(N::from_i64(total_validators))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_total_count_of_validators() {
        assert!(total_count_of_validators::<u64>(
            "https://prod-mainnet.prod.findora.org:26657",
            &None
        )
        .is_ok())
    }
}
