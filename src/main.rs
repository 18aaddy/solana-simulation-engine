use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, post},
};
use base64::{Engine, engine};
use bincode;
use litesvm::types::TransactionMetadata;
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use uuid::Uuid;
mod manager;
use manager::ForkManager;

use solana_sdk::transaction::VersionedTransaction;

#[derive(Deserialize)]
struct ExecuteRequest {
    tx_base64: String,
}

#[derive(Deserialize)]
struct SetLamportsRequest {
    pubkey: String,
    lamports: u64,
}

#[derive(Deserialize)]
struct SetTokenBalanceRequest {
    token_account: String,
    mint: String,
    owner: String,
    amount: u64,
}

#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let manager = Arc::new(Mutex::new(ForkManager::new()));

    let app = Router::new()
        .route("/forks", post(create_fork))
        .route("/forks/{id}", delete(delete_fork))
        .route("/forks/{id}/execute", post(execute_transaction))
        .route("/forks/{id}/simulate", post(simulate_transaction))
        .route("/forks/{id}/set_lamports", post(set_lamports))
        .route("/forks/{id}/set_token_balance", post(set_token_balance))
        .with_state(manager);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("server running at {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}

#[axum::debug_handler]
async fn create_fork(State(manager): State<Arc<Mutex<ForkManager>>>) -> Json<ApiResponse<Uuid>> {
    match manager.lock().unwrap().create_fork() {
        Ok(fork_id) => Json(ApiResponse {
            success: true,
            data: Some(fork_id),
            error: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("{:?}", e)),
        }),
    }
}

#[axum::debug_handler]
async fn delete_fork(
    State(manager): State<Arc<Mutex<ForkManager>>>,
    Path(fork_id): Path<Uuid>,
) -> Json<ApiResponse<String>> {
    if manager.lock().unwrap().delete_fork(&fork_id) {
        Json(ApiResponse {
            success: true,
            data: Some(format!("Deleted fork {}", fork_id)),
            error: None,
        })
    } else {
        Json(ApiResponse {
            success: false,
            data: None,
            error: Some("Fork not found".into()),
        })
    }
}

#[axum::debug_handler]
async fn execute_transaction(
    State(manager): State<Arc<Mutex<ForkManager>>>,
    Path(fork_id): Path<Uuid>,
    Json(req): Json<ExecuteRequest>,
) -> Json<ApiResponse<TransactionMetadata>> {
    let tx_bytes = engine::general_purpose::STANDARD
        .decode(&req.tx_base64)
        .unwrap();
    let tx: VersionedTransaction = bincode::deserialize(&tx_bytes).unwrap();

    match manager.lock().unwrap().execute_transaction(&fork_id, tx) {
        Ok(result) => Json(ApiResponse {
            success: true,
            data: Some(result),
            error: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("{:?}", e)),
        }),
    }
}

#[axum::debug_handler]
async fn simulate_transaction(
    State(manager): State<Arc<Mutex<ForkManager>>>,
    Path(fork_id): Path<Uuid>,
    Json(req): Json<ExecuteRequest>,
) -> Json<ApiResponse<TransactionMetadata>> {
    let tx_bytes = engine::general_purpose::STANDARD
        .decode(&req.tx_base64)
        .unwrap();
    let tx: VersionedTransaction = bincode::deserialize(&tx_bytes).unwrap();

    match manager.lock().unwrap().simulate_transaction(&fork_id, tx) {
        Ok(info) => Json(ApiResponse {
            success: true,
            data: Some(info.meta),
            error: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("{:?}", e)),
        }),
    }
}

#[axum::debug_handler]
async fn set_lamports(
    State(manager): State<Arc<Mutex<ForkManager>>>,
    Path(fork_id): Path<Uuid>,
    Json(req): Json<SetLamportsRequest>,
) -> Json<ApiResponse<String>> {
    use solana_sdk::pubkey::Pubkey;
    let pubkey = req.pubkey.parse::<Pubkey>().unwrap();

    match manager
        .lock()
        .unwrap()
        .set_lamports(&fork_id, pubkey, req.lamports)
    {
        Ok(_) => Json(ApiResponse {
            success: true,
            data: Some(format!("Set lamports for {}", pubkey)),
            error: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("{:?}", e)),
        }),
    }
}

#[axum::debug_handler]
async fn set_token_balance(
    State(manager): State<Arc<Mutex<ForkManager>>>,
    Path(fork_id): Path<Uuid>,
    Json(req): Json<SetTokenBalanceRequest>,
) -> Json<ApiResponse<String>> {
    use solana_sdk::pubkey::Pubkey;
    let token_account = req.token_account.parse::<Pubkey>().unwrap();
    let mint = req.mint.parse::<Pubkey>().unwrap();
    let owner = req.owner.parse::<Pubkey>().unwrap();

    match manager.lock().unwrap().set_token_balance(
        &fork_id,
        token_account,
        mint,
        owner,
        req.amount,
    ) {
        Ok(_) => Json(ApiResponse {
            success: true,
            data: Some(format!("Set token balance for {}", token_account)),
            error: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("{:?}", e)),
        }),
    }
}
