use crate::server::api::start;
use anyhow::Result;

pub async fn handle(host: String, port: u16) -> Result<()> {
    start(host, port).await
}
