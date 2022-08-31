// curl -H 'Accept: application/json' -X GET https://api.gateio.ws/api/v4/spot/candlesticks\?currency_pair\=FRA_USDT\&interval\=15m\&limit\=1
// [[unix_timestamp, trading_volume, close_price, highest_price, lowest_price, open_price]]
// [["1645749900","2839.79160470986265","0.01815","0.01897","0.01793","0.01889"]]
use anyhow::{bail, Context, Result};
use prometheus::core::Number;
use serde_json::Value;

pub(crate) fn get_price<N: Number>(pair: &str) -> Result<N> {
    // https://api.gateio.ws/api/v4/spot/candlesticks\?currency_pair\=FRA_USDT\&interval\=15m\&limit\=1
    let path = format!(
        "https://api.gateio.ws/api/v4/spot/candlesticks?interval=15m&limit=1&currency_pair={}",
        pair
    );

    let data: Value = ureq::get(&path)
        .set("Accept", "application/json")
        .call()
        .with_context(|| format!("requesting gate.io call failed, pair:{:?}", pair))?
        .into_json()
        .with_context(|| format!("requesting gate.io json failed, pair:{:?}", pair))?;

    if !data.is_array() || !data[0].is_array() {
        bail!(
            "the data is not an array which is:{:?}, pair:{:?}",
            data,
            pair
        )
    }

    let d = data
        .as_array()
        .with_context(|| format!("data as_array failed, data:{:?}, pair:{:?}", data, pair))?;

    if d.len() != 1 {
        bail!(
            "first level of data length is not 1: data:{:?}, pair:{:?}",
            data,
            pair
        );
    }

    let info = d[0]
        .as_array()
        .with_context(|| format!("info as_array failed, data:{:?}, pair:{:?}", data, pair))?;

    if info.len() < 3 {
        bail!(
            "second level of data length is smaller than 3: data:{:?}, pair:{:?}",
            data,
            pair
        );
    }

    let price = info[2].as_str().with_context(|| {
        format!(
            "close price as_str failed, price:{:?}, pair:{:?}",
            info[2], pair
        )
    })?;

    let price = price.parse::<f64>().with_context(|| {
        format!(
            "close price parse into f64 failed, price:{:?}, pair:{:?}",
            price, pair,
        )
    })?;

    Ok(N::from_i64((price * 1000000.0).round() as i64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_price() {
        assert!(get_price::<u64>("FRA_USDT",).is_ok())
    }
}
