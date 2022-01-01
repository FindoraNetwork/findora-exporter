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
