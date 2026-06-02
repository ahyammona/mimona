/// Mimona node network registry client.
/// Nodes register here so other clients can route requests to them.

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RemoteNode {
    pub id: String,
    pub wallet: String,
    pub endpoint: String,
    pub models: Vec<String>,
    pub last_seen: String,
    pub price_sol: f64,
}

/// Fetch nodes that have a specific model
pub async fn find_nodes_for_model(model_name: &str) -> Result<Vec<RemoteNode>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let resp = client
        .get(format!(
            "https://registry.mimona.io/nodes?model={}",
            urlencoding::encode(model_name)
        ))
        .send()
        .await?;

    let nodes: Vec<RemoteNode> = resp.json().await?;
    Ok(nodes)
}

/// Pick the best (cheapest, most recent) node
pub fn pick_best_node(nodes: Vec<RemoteNode>) -> Option<RemoteNode> {
    nodes.into_iter().min_by(|a, b| {
        a.price_sol
            .partial_cmp(&b.price_sol)
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}
