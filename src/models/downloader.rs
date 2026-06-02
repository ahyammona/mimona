use crate::config::models_dir;
use crate::models::storage::{register_model, LocalModel};
use crate::models::{ModelEntry, ModelVariant};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use colored::*;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;

const HF_BASE: &str = "https://huggingface.co";
const HF_TOKEN_ENV: &str = "HF_TOKEN";

pub async fn download_model(entry: &ModelEntry, tag: &str) -> Result<PathBuf> {
    let variant = entry
        .variants
        .get(tag)
        .ok_or_else(|| anyhow!("Tag '{}' not found for model '{}'", tag, entry.name))?;

    let dest = models_dir().join(&variant.filename);

    if dest.exists() {
        println!("  {} Model already downloaded.", "✓".green());
        return Ok(dest);
    }

    fs::create_dir_all(models_dir()).await?;

    // Try P2P nodes first (future: query node network)
    // For now, fall straight to HuggingFace
    download_from_hf(variant, &dest).await?;

    // Verify checksum if available
    if let Some(expected) = &variant.checksum {
        println!("  {} Verifying checksum...", "→".cyan());
        verify_checksum(&dest, expected).await?;
        println!("  {} Checksum OK", "✓".green());
    }

    // Register in local manifest
    let meta = fs::metadata(&dest).await?;
    register_model(LocalModel {
        name: entry.name.clone(),
        tag: tag.to_string(),
        filename: variant.filename.clone(),
        size_bytes: meta.len(),
        downloaded_at: Utc::now().to_rfc3339(),
        checksum: variant.checksum.clone(),
    })
    .await?;

    Ok(dest)
}

async fn download_from_hf(variant: &ModelVariant, dest: &PathBuf) -> Result<()> {
    let url = format!(
        "{}/{}/resolve/main/{}",
        HF_BASE, variant.hf_repo, variant.filename
    );

    
    

    let mut req = reqwest::Client::new().get(&url);

    // Add HF token if set
    if let Ok(token) = std::env::var(HF_TOKEN_ENV) {
        req = req.bearer_auth(token);
    }

    let resp = req
        .send()
        .await
        .context("Failed to connect to HuggingFace")?;

    if !resp.status().is_success() {
        if resp.status() == 401 || resp.status() == 403 {
            return Err(anyhow!(
                "Access denied. Set HF_TOKEN env var for gated models."
            ));
        }
        return Err(anyhow!("HTTP {} from HuggingFace", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let gb = total as f64 / 1_073_741_824.0;

    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("  [{bar:45.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
            .unwrap()
            .progress_chars("█▉▊▋▌▍▎▏ "),
    );

    // Temp file while downloading
    let tmp = dest.with_extension("part");
    let mut file = File::create(&tmp).await?;
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Stream error")?;
        pb.inc(chunk.len() as u64);
        file.write_all(&chunk).await?;
    }

    pb.finish_and_clear();
    file.flush().await?;
    drop(file);

    // Rename temp to final
    fs::rename(&tmp, dest).await?;

    println!("  {} Downloaded {:.2} GB", "✓".green(), gb);
    Ok(())
}

async fn verify_checksum(path: &PathBuf, expected: &str) -> Result<()> {
    let data = fs::read(path).await?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let actual = hex::encode(hasher.finalize());

    let expected_clean = expected.trim_start_matches("sha256:");
    if actual != expected_clean {
        return Err(anyhow!(
            "Checksum mismatch!\n  Expected: {}\n  Got:      {}",
            expected_clean,
            actual
        ));
    }
    Ok(())
}
