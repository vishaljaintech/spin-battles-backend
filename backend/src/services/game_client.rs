use crate::{errors::AppError, models::PendingBattle};
use serde::Deserialize;
use std::sync::OnceLock;
use std::time::Duration;

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("failed to build shared reqwest client")
    })
}

/// Response shape from GET /battles/:address
#[derive file is uploaded successfully, add more      (Deserialize)]
struct BattlesResponse {
    battles: Vec<GameBattle>,
}

#[derive file is uploaded successfully, add more     (Deserialize)]
struct GameBattle {
    battle_id: String,
    battle_name: String,
    reward_sbr: String,
    reward_lamports: String,
    played_at: i64,
}

/// Response shape from GET /battles/:battle_id/verify
#[derive file is uploaded successfully, add more     (Deserialize)]
pub struct VerifyResponse {
    pub eligible: bool,
    #[allow(dead_code)]
    pub reward_lamports: String,
}

fn game_server_url() -> String {
    std::env::var("GAME_SERVER_URL").unwrap_or_else(|_| "http://localhost:8081".to_string())
}

/// Fetch all pending won battles for a player from the game server.
///
/// Returns an error if the game server is unreachable — the backend cannot
/// authorise claims without verified battle data.
pub async fn get_pending_battles(address: &str) -> Result<Vec<PendingBattle>, AppError> {
    let url = format!("{}/battles/{}", game_server_url(), address);

    let response = http_client()
        .get(&url)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Game server unreachable at {}: {}", url, e);
            AppError::GameServerUnavailable
        })?;

    if !response.status().is_success() {
        tracing::error!("Game server returned {} for {}", response.status(), url);
        return Err(AppError::GameServerUnavailable);
    }

    let body: BattlesResponse = response.json().await.map_err(|e| {
        tracing::error!("Failed to parse game server response: {}", e);
        AppError::Internal(anyhow::anyhow!("Invalid game server response"))
    })?;

    let battles = body
        .battles
        .into_iter()
        .map(|b| PendingBattle {
            battle_id: b.battle_id,
            battle_name: b.battle_name,
            reward: b.reward_sbr,
            reward_lamports: b.reward_lamports,
            timestamp: b.played_at,
            status: "unclaimed",
        })
        .collect();

    Ok(battles)
}

/// Verify a specific battle result with the game server.
///
/// Returns `(eligible, reward_lamports)` or an error if the game server is down.
pub async fn verify_battle(battle_id: &str) -> Result<VerifyResponse, AppError> {
    let url = format!("{}/battles/{}/verify", game_server_url(), battle_id);

    let response = http_client()
        .get(&url)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Game server unreachable at {}: {}", url, e);
            AppError::GameServerUnavailable
        })?;

    let body: VerifyResponse = response.json().await.map_err(|e| {
        tracing::error!("Failed to parse game server verify response: {}", e);
        AppError::Internal(anyhow::anyhow!("Invalid game server response"))
    })?;

    Ok(body)
}
