use alloy_rpc_client::{ClientBuilder, RpcCall};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HyperliquidMeta {
    universe: Vec<HyperliquidAsset>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HyperliquidAsset {
    name: String,
    sz_decimals: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HyperliquidAssetContext {
    mark_px: String,
}

#[tokio::test]
async fn it_posts_plain_json_to_hyperliquid_info() {
    let client =
        ClientBuilder::default().plain_http("https://api.hyperliquid.xyz/info".parse().unwrap());
    let req: RpcCall<_, _, (HyperliquidMeta, Vec<HyperliquidAssetContext>)> =
        client.request_json(json!({ "type": "metaAndAssetCtxs" }));
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(10), req);
    let response = timeout.await.unwrap().unwrap();

    assert!(response.0.universe.len() > 100);
    assert_eq!(response.0.universe.len(), response.1.len());
    assert!(response.0.universe.iter().any(|asset| asset.name == "BTC"));
    assert!(response.0.universe.iter().all(|asset| asset.sz_decimals <= 8));
    assert!(response.1.iter().all(|ctx| ctx.mark_px.parse::<f64>().unwrap() > 0.0));
}
