use crate::node::provider::{node_status, start_node};
use anyhow::Result;
use colored::*;

pub async fn start() -> Result<()> {
    start_node(11435).await
}

pub async fn stop() -> Result<()> {
    println!("\n  {} Node stopped.\n", "✓".green());
    Ok(())
}

pub async fn status() -> Result<()> {
    node_status().await
}

pub async fn earnings() -> Result<()> {
    println!();
    println!("  {}", "Node Earnings".bold());
    println!("  {}", "─".repeat(40).dimmed());
    println!("  {} 0.0000 SOL", "Today:".dimmed());
    println!("  {} 0.0000 SOL", "This week:".dimmed());
    println!("  {} 0.0000 SOL", "All time:".dimmed());
    println!("  {} 0 queries served", "Queries:".dimmed());
    println!();
    println!("  Start earning:  {} node start\n", "mimona".cyan());
    Ok(())
}
