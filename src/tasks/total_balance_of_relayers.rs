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
                     // the keccak-256 hashed EVM method
                     "data":"0xca15c873e2b7fb3b832174769106daebcfd6d1970523240dda11281102db9363b83b0dc4",
                     "to":bridge_addr
                 },
                 "latest"
             ],
         })).context("get_relayer_balance ask relayer count ureq call failed")?
         .into_json().context("get_relayer_balance ask relayer count ureq json failed")?;

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

    println!("########: {}", count);

    // asking the bridge the releyer addresses
    let mut reqs = vec![];
    for i in 0..count {
        reqs.push(ureq::json!({
            "method":"eth_call", 
            "jsonrpc":"2.0", 
            "id":i, 
            "params":[
                {
                    "data":format!("0x9010d07ce2b7fb3b832174769106daebcfd6d1970523240dda11281102db9363b83b0dc4{}", format!("{:064}", i)), 
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

    println!("{:?}", relayers);

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

    // let mut balances = 0;
    for d in data {
        let balance = &d["result"];
        if balance.is_null() {
            bail!("get_relayer_balance the balance result is null: {}", d)
        }

        let balance = match balance.as_str() {
            Some(v) => u64::from_str_radix(v.trim_start_matches("0x"), 16)
                .with_context(|| format!("balance parse hex failed: {}", v))?,
            None => bail!("get_relayer_balance the balance result is not a str: {}", d),
        };

        println!("{}", balance);
        // balances += balance;
    }

    Ok(N::from_i64(0))
}
