use crate::config::ExtraOpts;
use crate::utils::{diff_of_decimal_18, toi64_div_10pow12};

use anyhow::{bail, Context, Result};
use prometheus::core::Number;
use serde_json::Value;

pub(crate) fn total_balance_of_relayers<N: Number>(
    addr: &str,
    opts: &Option<ExtraOpts>,
) -> Result<N> {
    let (bridge_addr, decimal) = match opts {
        Some(ExtraOpts::TotalBalanceOfRelayers {
            bridge_address,
            decimal,
        }) => (bridge_address, decimal),
        _ => bail!("expecting extra_opts: bridge_address, addr:{:?}", addr),
    };

    // asking the bridge for the count of relayers
    let data: Value = ureq::post(addr)
        .send_json(ureq::json!({
            "method":"eth_call",
            "jsonrpc":"2.0",
            "id":0,
            "params":[
                {
                    // keccak256("_totalRelayers()")[:8] = "0x802aabe8"
                    // https://github.com/ChainSafe/chainbridge-solidity/blob/master/contracts/Bridge.sol#L314
                    //
                    // no params so only function signature
                    "data":"0x802aabe8",
                    "to":bridge_addr
                },
                "latest"
            ],
        }))
        .with_context(|| {
            format!(
                "ask relayer count ureq call failed, addr:{:?}, opts:{:?}",
                addr, opts
            )
        })?
        .into_json()
        .with_context(|| {
            format!(
                "ask relayer count ureq json failed, addr:{:?}, opts:{:?}",
                addr, opts
            )
        })?;

    let count = &data["result"];
    if count.is_null() {
        bail!(
            "the relayer count is null, addr:{:?}, opts:{:?}",
            addr,
            opts
        )
    }

    let count = match count.as_str() {
        Some(v) => usize::from_str_radix(v.trim_start_matches("0x"), 16).with_context(|| {
            format!(
                "count parse hex failed: {}, addr:{:?}, opts:{:?}",
                v, addr, opts
            )
        })?,
        None => bail!(
            "the relayer count is not a str: {}, addr:{:?}, opts:{:?}",
            count,
            addr,
            opts
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
                    //
                    // keccak256("RELAYER_ROLE") = "e2b7fb3b832174769106daebcfd6d1970523240dda11281102db9363b83b0dc4"
                    // https://github.com/ChainSafe/chainbridge-solidity/blob/master/contracts/Bridge.sol#L73
                    //
                    // function signature(8) + padding(RELAYER_ROLE)(64) + padding(index)(64)
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
        .with_context(|| {
            format!(
                "ask relayer addresses ureq call failed, addr:{:?}, opts:{:?}",
                addr, opts
            )
        })?
        .into_json()
        .with_context(|| {
            format!(
                "ask relayer addresses ureq json failed, addr:{:?}, opts:{:?}",
                addr, opts
            )
        })?;

    let data = data.as_array().with_context(|| {
        format!(
            "ask relayer addresses as_array failed, addr:{:?}, opts:{:?}",
            addr, opts
        )
    })?;

    let mut relayers = vec![];
    for d in data {
        let relayer = &d["result"];
        if relayer.is_null() {
            bail!(
                "the relayer result is null: {}, addr:{:?}, opts:{:?}",
                d,
                addr,
                opts
            )
        }

        let relayer = match relayer.as_str() {
            Some(v) => {
                let mut addr = v
                    .trim_start_matches("0x")
                    .trim_start_matches('0')
                    .to_string();
                if addr.len() < 40 {
                    for _ in 0..40 - addr.len() {
                        addr = "0".to_string() + &addr;
                    }
                }
                format!("0x{}", addr)
            }
            None => bail!(
                "the relayer result is not a str: {}, addr:{:?}, opts:{:?}",
                d,
                addr,
                opts
            ),
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
        .with_context(|| {
            format!(
                "ask relayer balances ureq call failed, addr:{:?}, opts:{:?}",
                addr, opts
            )
        })?
        .into_json()
        .with_context(|| {
            format!(
                "ask relayer balances ureq json failed, addr:{:?}, opts:{:?}",
                addr, opts
            )
        })?;

    let data = data.as_array().with_context(|| {
        format!(
            "ask relayer balances as_array failed, addr:{:?}, opts:{:?}",
            addr, opts
        )
    })?;

    let mut balances: i64 = 0;
    for d in data {
        let balance = &d["result"];
        if balance.is_null() {
            bail!(
                "the balance result is null: {}, addr:{:?}, opts:{:?}",
                d,
                addr,
                opts
            )
        }

        let balance = match balance.as_str() {
            Some(v) => u128::from_str_radix(v.trim_start_matches("0x"), 16).with_context(|| {
                format!(
                    "balance parse hex failed: {}, addr:{:?}, opts:{:?}",
                    v, addr, opts
                )
            })?,
            None => bail!(
                "the balance result is not a str: {}, addr:{:?}, opts:{:?}",
                d,
                addr,
                opts
            ),
        };

        balances += toi64_div_10pow12(balance, diff_of_decimal_18(decimal));
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
            &Some(ExtraOpts::TotalBalanceOfRelayers {
                bridge_address: "0xD609931ec1c7a7F6ad59A69fede03fB067Af997c".to_string(),
                decimal: 18,
            }),
        )
        .is_ok());
        assert!(total_balance_of_relayers::<u64>(
            "https://prod-forge.prod.findora.org:8545/",
            &Some(ExtraOpts::TotalBalanceOfRelayers {
                bridge_address: "0xe58C2e75147c462F63cB310462fFC412f5875852".to_string(),
                decimal: 18,
            }),
        )
        .is_ok());
    }
}
