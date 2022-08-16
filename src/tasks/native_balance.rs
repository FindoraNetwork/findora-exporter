use crate::config::ExtraOpts;
use crate::utils::{diff_of_decimal_18, toi64_div_10pow12};

use anyhow::{bail, Context, Result};
use prometheus::core::Number;
use serde_json::Value;

pub(crate) fn native_balance<N: Number>(addr: &str, opts: &Option<ExtraOpts>) -> Result<N> {
    let (native_addr, decimal) = match opts {
        Some(ExtraOpts::NativeBalance {
            native_address,
            decimal,
        }) => (native_address, decimal),
        _ => {
            bail!(
                "expecting extra_opts: native_address and decimal, addr:{:?}",
                addr
            )
        }
    };

    let data: Value = ureq::post(addr)
        .send_json(ureq::json!({
            "method":"eth_getBalance",
            "jsonrpc":"2.0",
            "id":0,
            "params":[
                native_addr,
                "latest"
            ],
        }))
        .with_context(|| {
            format!(
                "requesting eth_getBalance ureq call failed, addr:{:?}, opts:{:?}",
                addr, opts
            )
        })?
        .into_json()
        .with_context(|| {
            format!(
                "requesting eth_getBalance ureq json failed, addr:{:?}, opts:{:?}",
                addr, opts
            )
        })?;

    let balance = &data["result"];
    if balance.is_null() {
        bail!("the balance is null, addr:{:?}, opts:{:?}", addr, opts)
    }

    let balance = match balance.as_str() {
        Some(v) => {
            let mut b = v.trim_start_matches("0x");
            if b.is_empty() {
                b = "0"
            };
            u128::from_str_radix(b, 16).with_context(|| {
                format!(
                    "balance parse hex failed: {}, addr:{:?}, opts:{:?}",
                    v, addr, opts
                )
            })?
        }
        None => bail!(
            "the balance result is not a str: {}, addr:{:?}, opts:{:?}",
            balance,
            addr,
            opts
        ),
    };

    Ok(N::from_i64(toi64_div_10pow12(
        balance,
        diff_of_decimal_18(decimal),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_balance() {
        println!(
            "{:?}",
            native_balance::<u64>(
                "https://data-seed-prebsc-1-s1.binance.org:8545",
                &Some(ExtraOpts::NativeBalance {
                    native_address: "0xae13d989dac2f0debff460ac112a837c89baa7cd".to_string(),
                    decimal: 18,
                }),
            )
        );
        assert!(native_balance::<u64>(
            "https://data-seed-prebsc-1-s1.binance.org:8545",
            &Some(ExtraOpts::NativeBalance {
                native_address: "0xae13d989dac2f0debff460ac112a837c89baa7cd".to_string(),
                decimal: 18,
            }),
        )
        .is_ok())
    }
}
