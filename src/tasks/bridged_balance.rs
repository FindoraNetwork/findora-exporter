use crate::config::ExtraOpts;

use anyhow::{bail, Context, Result};

use prometheus::core::Number;
use serde_json::Value;

pub(crate) fn bridged_balance<N: Number>(addr: &str, opts: &Option<ExtraOpts>) -> Result<N> {
    let (handler_addr, token_addr) = match opts {
        Some(ExtraOpts::BridgedBalance {
            erc20handler_address,
            token_address,
        }) => (erc20handler_address, token_address),
        _ => {
            bail!("bridged_balance expecting extra_opts: erc20handler_address and token_address")
        }
    };

    let data: Value = ureq::post(addr)
        .send_json(ureq::json!({
            "method":"eth_call",
            "jsonrpc":"2.0",
            "id":0,
            "params":[
                {
                    // keccak256("balanceOf(address)")[:8] = "70a08231"
                    // https://eips.ethereum.org/EIPS/eip-20#balanceOf
                    //
                    // 0x + function signature(8) + padding(erc20Handler)(64)
                    "data":format!("0x70a08231{:0>64}", handler_addr.trim_start_matches("0x")),
                    "to":token_addr
                },
                "latest"
            ],
        }))
        .context("bridged_balance requesting balanceOf ureq call failed")?
        .into_json()
        .context("bridged_balance requesting balanceOf ureq json failed")?;

    let balance = &data["result"];
    if balance.is_null() {
        bail!("bridged_balance the balance is null")
    }

    let balance = match balance.as_str() {
        Some(v) => {
            let mut b = v.trim_start_matches("0x");
            if b.is_empty() {
                b = "0"
            };
            i64::from_str_radix(b, 16)
                .with_context(|| format!("balance parse hex failed: {}", v))?
        }
        None => bail!(
            "bridged_balance the balance result is not a str: {}",
            balance
        ),
    };

    Ok(N::from_i64(balance))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridged_balance() {
        assert!(bridged_balance::<u64>(
            "https://prod-testnet.prod.findora.org:8545",
            &Some(ExtraOpts::BridgedBalance {
                erc20handler_address: "0xe2b65e624bBb5513fF805d225258D7A92b0f62C4".to_string(),
                token_address: "0x6ce8da28e2f864420840cf74474eff5fd80e65b8".to_string(),
            }),
        )
        .is_ok())
    }
}
