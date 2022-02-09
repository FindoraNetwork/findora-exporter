use crate::config::ExtraOpts;

use anyhow::{bail, Context, Result};

use prometheus::core::Number;
use serde_json::Value;

pub(crate) fn total_balance_of_relayers<N: Number>(
    addr: &str,
    opts: &Option<ExtraOpts>,
) -> Result<N> {
    // asking the bridge for the count of relayers
    let bridge_addr = match opts {
        Some(o) => {
            let ExtraOpts::BridgeAddress(b) = o;
            b
        }
        None => bail!("get_relayer_balance cannot get bridge address"),
    };
    let data: Value = ureq::post(addr)
        .send_json(ureq::json!({
            "method":"eth_call",
            "jsonrpc":"2.0",
            "id":0,
            "params":[
                {
                    // keccak256("_totalRelayers()")[:8] = "0x802aabe8"
                    // https://github.com/ChainSafe/chainbridge-solidity/blob/master/contracts/Bridge.sol#L314
                    "data":"0x802aabe8",
                    "to":bridge_addr
                },
                "latest"
            ],
        }))
        .context("get_relayer_balance ask relayer count ureq call failed")?
        .into_json()
        .context("get_relayer_balance ask relayer count ureq json failed")?;

    let count = &data["result"];
    if count.is_null() {
        bail!("get_relayer_balance the relayer count is null")
    }

    let count = match count.as_str() {
        Some(v) => usize::from_str_radix(v.trim_start_matches("0x"), 16)
            .with_context(|| format!("count parse hex failed: {}", v))?,
        None => bail!(
            "get_relayer_balance the relayer count is not a str: {}",
            count
        ),
    };

    // asking the bridge the releyer addresses
    let mut reqs = vec![];
    for i in 0..count {
        reqs.push(ureq::json!({
            "method":"eth_call", 
            "jsonrpc":"2.0", 
            "id":i, 
            "params":[
                {
                    // keccak256("getRoleMember(bytes32,uint256)")[:8] = "9010d07c"
                    // https://github.com/ChainSafe/chainbridge-solidity/blob/master/contracts/utils/AccessControl.sol#L104
                    // keccak256("RELAYER_ROLE") = "e2b7fb3b832174769106daebcfd6d1970523240dda11281102db9363b83b0dc4"
                    // https://github.com/ChainSafe/chainbridge-solidity/blob/master/contracts/Bridge.sol#L73
                    "data":format!(
                        "0x9010d07ce2b7fb3b832174769106daebcfd6d1970523240dda11281102db9363b83b0dc4{:064x}", i), 
                    "to":bridge_addr
                },
                "latest"
            ]
        }))
    }

    let data: Value = ureq::post(addr)
        .send_json(serde_json::Value::Array(reqs))
        .context("get_relayer_balance ask relayer addresses ureq call failed")?
        .into_json()
        .context("get_relayer_balance ask relayer addresses ureq json failed")?;

    let data = data
        .as_array()
        .context("get_relayer_balance ask relayer addresses as_array failed")?;

    let mut relayers = vec![];
    for d in data {
        let relayer = &d["result"];
        if relayer.is_null() {
            bail!("get_relayer_balance the relayer result is null: {}", d)
        }

        let relayer = match relayer.as_str() {
            Some(v) => format!("0x{}", v.trim_start_matches("0x").trim_start_matches('0')),
            None => bail!("get_relayer_balance the relayer result is not a str: {}", d),
        };

        relayers.push(relayer);
    }

    // asking the releyer balances
    let mut reqs = vec![];
    for (i, relayer) in relayers.iter().enumerate() {
        reqs.push(ureq::json!({
            "method":"eth_getBalance",
            "jsonrpc":"2.0",
            "id":i,
            "params":[
                relayer,
                "latest"
            ]
        }));
    }

    let data: Value = ureq::post(addr)
        .send_json(serde_json::Value::Array(reqs))
        .context("get_relayer_balance ask relayer balances ureq call failed")?
        .into_json()
        .context("get_relayer_balance ask relayer balances ureq json failed")?;

    let data = data
        .as_array()
        .context("get_relayer_balance ask relayer balances as_array failed")?;

    let mut balances: i64 = 0;
    for d in data {
        let balance = &d["result"];
        if balance.is_null() {
            bail!("get_relayer_balance the balance result is null: {}", d)
        }

        let balance = match balance.as_str() {
            Some(v) => u128::from_str_radix(v.trim_start_matches("0x"), 16)
                .with_context(|| format!("balance parse hex failed: {}", v))?,
            None => bail!("get_relayer_balance the balance result is not a str: {}", d),
        };

        // the balance came from relayer is like below:
        // 9989580120000000000
        // and it is using the 18th number as it's decimal point
        // 9989580120000000000 = 9.989580120000000000
        // and the max i64 is 9989580120000000000 a 19th number
        // so for filling this huge number into i64 we div by 10.
        balances += (balance.wrapping_div(10u128.pow(10))) as i64;
    }

    // the real balances needs to div by 8 again
    Ok(N::from_i64(balances))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_total_balance_of_relayers() {
        assert!(total_balance_of_relayers::<u64>(
            "https://data-seed-prebsc-1-s1.binance.org:8545",
            &Some(ExtraOpts::BridgeAddress(
                "0xD609931ec1c7a7F6ad59A69fede03fB067Af997c".to_string()
            ))
        )
        .is_ok())
    }
}
