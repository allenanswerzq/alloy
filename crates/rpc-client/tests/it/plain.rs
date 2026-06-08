use alloy_rpc_client::{ClientBuilder, RpcCall};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
#[serde(try_from = "Value")]
struct HyperliquidMarketContext {
    universe_len: usize,
    context_len: usize,
    has_btc: bool,
    valid_sz_decimals: bool,
    positive_mark_prices: bool,
}

impl TryFrom<Value> for HyperliquidMarketContext {
    type Error = String;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let Value::Array(mut values) = value else {
            return Err("expected [meta, asset_contexts] response".to_owned());
        };
        if values.len() != 2 {
            return Err(format!("expected 2 response entries, got {}", values.len()));
        }

        let contexts = values.pop().unwrap();
        let meta = values.pop().unwrap();
        let universe = meta
            .get("universe")
            .and_then(Value::as_array)
            .ok_or_else(|| "missing meta.universe array".to_owned())?;
        let contexts =
            contexts.as_array().ok_or_else(|| "missing asset contexts array".to_owned())?;

        Ok(Self {
            universe_len: universe.len(),
            context_len: contexts.len(),
            has_btc: universe
                .iter()
                .any(|asset| asset.get("name").and_then(Value::as_str) == Some("BTC")),
            valid_sz_decimals: universe.iter().all(|asset| {
                asset
                    .get("szDecimals")
                    .and_then(Value::as_u64)
                    .is_some_and(|decimals| decimals <= 8)
            }),
            positive_mark_prices: contexts.iter().all(|ctx| {
                ctx.get("markPx")
                    .and_then(Value::as_str)
                    .and_then(|mark_px| mark_px.parse::<f64>().ok())
                    .is_some_and(|mark_px| mark_px > 0.0)
            }),
        })
    }
}

#[tokio::test]
async fn it_posts_plain_json_to_hyperliquid_info() {
    let client =
        ClientBuilder::default().plain_http("https://api.hyperliquid.xyz/info".parse().unwrap());
    let req: RpcCall<_, _, HyperliquidMarketContext> =
        client.request_json(json!({ "type": "metaAndAssetCtxs" }));
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(10), req);
    let response = timeout.await.unwrap().unwrap();

    assert!(response.universe_len > 100);
    assert_eq!(response.universe_len, response.context_len);
    assert!(response.has_btc);
    assert!(response.valid_sz_decimals);
    assert!(response.positive_mark_prices);
}
