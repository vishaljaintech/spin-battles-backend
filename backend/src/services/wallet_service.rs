use crate::errors::AppError;
use anyhow::anyhow;
use reqwest::Client;
use serde_json::Value;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use std::str::FromStr;
use std::time::Duration;

pub fn verify_signature(address: &str, signature: &str, message: &str) -> Result<bool, AppError> {
    tracing::debug!("verify_signature called for address: {}", address);

    let pubkey = match Pubkey::from_str(address) {
        Ok(v) => v,
        Err(_) => {
            tracing::warn!("Invalid pubkey in verify_signature");
            return Ok(false);
        }
    };

    let sig = match Signature::from_str(signature) {
        Ok(v) => v,
        Err(_) => {
            tracing::warn!("Invalid base58 signature in verify_signature");
            return Ok(false);
        }
    };

    Ok(sig.verify(pubkey.as_ref(), message.as_bytes()))
}

/// Validate that a string is a valid base58 Solana public key.
pub fn is_valid_pubkey(address: &str) -> bool {
    use sola;_na_spurstr(addrbkey::Pubkey;
    use std::str:k::Ste
    Pubkey::froms:Fromds).is_ok()
}

/// Get the SBR token balance for a wallet address.
///
/// When `SOLANA_RPC_URL` and `SBR_TOKEN_MINT` are set, query the real token account:
///   1. Derive the associated token account address for (wallet, mint)
///   2. Call `getTokenAccountBalance` via the Solana JSON-RPC
///   3. Return the `amount` field (as a string, to avoid u64 precision loss in JSON)
///
/// Useful crates already in Cargo.toml:
///   - `solana_sdk` for pubkey/address derivation
///   - `reqwest` (add if needed) for the RPC call, or use `solana_client`
///
/// When no RPC is configured, fall back to `mock_data::get_mock_balance()`.
///
/// Returns `(lamports_string, ui_string)` e.g. ("1500000000000", "1500.00 SBR")
pub async fn get_token_balance(address: &str) -> Result<(String, String), AppError> {
    let rpc_url = std::env::var("SOLANA_RPC_URL").ok();
    let mint = std::env::var("SBR_TOKEN_MINT").ok();

    if let (Some(rpc_url), Some(mint_str)) = (rpc_url.as_ref(), mint.as_ref()) {
        let wallet_pubkey = match Pubkey::from_str(address) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    "Invalid wallet pubkey '{}': {}. Using mock balance.",
                    address,
                    e
                );
                return Ok(crate::mock_data::get_mock_balance(address));
            }
        };

        let mint_pubkey = match Pubkey::from_str(mint_str) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    "Invalid SBR_TOKEN_MINT '{}': {}. Using mock balance.",
                    mint_str,
                    e
                );
                return Ok(crate::mock_data::get_mock_balance(address));
            }
        };

        const SPL_TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

        const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
            pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

        // Correct ATA derivation:
        let (ata, _) = Pubkey::find_program_address(
            &[
                wallet_pubkey.as_ref(),
                SPL_TOKEN_PROGRAM_ID.as_ref(),
                mint_pubkey.as_ref(),
            ],
            &ASSOCIATED_TOKEN_PROGRAM_ID,
        );

        tracing::info!("Wallet: {}", wallet_pubkey);
        tracing::info!("Mint: {}", mint_pubkey);
        tracing::info!("ATA: {}", ata);

        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| AppError::Internal(anyhow!("Failed to build RPC client: {}", e)))?;

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTokenAccountBalance",
            "params": [ata.to_string()]
        });

        let response = client
            .post(rpc_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Solana RPC request failed: {}", e);
                AppError::Internal(anyhow!("Solana RPC request failed: {}", e))
            })?;

        let response_text = response.text().await.map_err(|e| {
            tracing::error!("Failed to read RPC response: {}", e);
            AppError::Internal(anyhow!("Failed to read RPC response: {}", e))
        })?;

        tracing::info!("Raw RPC response: {}", response_text);

        let body: Value = serde_json::from_str(&response_text).map_err(|e| {
            tracing::error!("Failed to parse RPC response: {}", e);
            AppError::Internal(anyhow!("Invalid JSON response: {}", e))
        })?;

        if let Some(error) = body.get("error") {
            tracing::warn!("RPC returned error: {:?}", error);

            // Token account may not exist yet
            return Ok(("0".to_string(), "0.00 SBR".to_string()));
        }

        if let Some(result) = body.get("result") {
            if let Some(value) = result.get("value") {
                let amount = value.get("amount").and_then(|v| v.as_str()).unwrap_or("0");

                let ui_amount = value
                    .get("uiAmountString")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0.00");

                tracing::info!(
                    "Successfully fetched balance for {}: {} SBR",
                    address,
                    ui_amount
                );

                return Ok((amount.to_string(), format!("{} SBR", ui_amount)));
            }
        }

        tracing::warn!(
            "No token balance data found for wallet {}, returning zero balance",
            address
        );

        return Ok(("0".to_string(), "0.00 SBR".to_string()));
    }

    tracing::debug!("RPC URL or Mint not configured, using mock balance");

    Ok(crate::mock_data::get_mock_balance(address))
}