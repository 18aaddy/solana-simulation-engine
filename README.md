# Solana Fork Simulation Engine

A **Solana mainnet fork simulation engine** written in Rust, built using [`LiteSVM`](https://github.com/LiteSVM/litesvm).  
It allows developers to **create isolated, in-memory forks** of the Solana blockchain and **interact with dApps** (e.g., deposits, swaps) inside them — similar to how **Tenderly** provides mainnet forks for Ethereum.

---

## 🚀 Overview

This engine lets each user spin up a **dedicated Solana fork** starting from the latest mainnet block.  
Inside this fork, users can:

- Execute or simulate **real Solana transactions**
- Interact with mainnet **programs and token accounts**
- Modify balances and SPL token states
- Query updated balances via HTTP APIs
- Observe isolated, time-limited fork environments (15 minutes per fork)

All forks share no state and are ephemeral, running completely in-memory.

---

## 🧠 Architecture

```
            ┌────────────────────────────┐
            │        RPC Client          │
            │ (Fetches mainnet state)    │
            └─────────────┬──────────────┘
                          │
    ┌─────────────────────┴─────────────────────┐
    │               ForkManager                 │
    │-------------------------------------------│
    │ - create_fork()                           │
    │ - execute_transaction()                   │
    │ - simulate_transaction()                  │
    │ - get_account() / set_balance()           │
    │ - cleanup_expired()                       │
    └─────────────┬──────────────┬──────────────┘
                  │              │
         ┌────────┴──────┐ ┌─────┴─────────┐
         │   Fork A      │ │   Fork B      │
         │  (15 min TTL) │ │  (15 min TTL) │
         └───────────────┘ └───────────────┘
                │
      ┌────────────────────┐
      │   LiteSVM Runtime  │
      │  (Executes txs,    │
      │   stores accounts) │
      └────────────────────┘
```

Each fork maintains its own `LiteSVM` instance, sysvars, and account state.  
RPC is used only when fetching missing accounts or updating sysvars.

---

## ⚙️ Features

✅ **Mainnet Fork Creation**
- Fork created from the latest Solana header.

✅ **Isolated Environments**
- Each user has an independent, in-memory Solana runtime (via `LiteSVM`).
- Forks expire after 15 minutes.

✅ **Transaction Simulation & Execution**
- `simulate_transaction()` → read-only dry-run (no state change).
- `execute_transaction()` → full execution with state updates.

✅ **Automatic Account Fetching**
- Missing mainnet accounts auto-fetched via RPC.
- Ensures transactions referencing unknown programs/accounts run smoothly.
- This is mainly to avoid fetching the whole state at the start. Fetch a given account's state only when needed, instead of pre-loading the whole mainnet state, which could take a lot of time.

✅ **Balance Modification**
- `set_lamports()` and `set_token_balance()` to manually fund accounts.

✅ **Transaction Recording**
- Each fork logs all executed transactions (signature, slot, logs, success).

✅ **HTTP API Interface**
- Fully RESTful API using [Axum](https://docs.rs/axum/latest/axum/).

---

## 🧩 API Endpoints

| Method | Endpoint | Description |
|---------|-----------|-------------|
| `POST /forks` | Create a new fork | Returns a `fork_id` |
| `DELETE /forks/{id}` | Delete fork | |
| `POST /forks/{id}/execute` | Execute a transaction inside fork | Mutates fork state |
| `POST /forks/{id}/simulate` | Simulate transaction | Read-only |
| `POST /forks/{id}/set_lamports` | Manually set SOL balance | |
| `POST /forks/{id}/set_token_balance` | Manually set SPL token balance | |
| `POST /forks/{id}/get_account` | Fetch current account state | Returns updated balances |
| `POST /forks/{id}/get_executed_transactions` | List executed transactions |
| `POST /forks/{id}/get_simulated_transactions` | List simulated transactions |
---

## 🧪 Example Usage

### 1️⃣ Create a fork
```bash
curl -X POST http://localhost:8080/forks
````

Response:

```json
{ "success": true, "data": "b6f98e3b-75e9-4dc8-a52e-bf1ad9c4e1e7" }
```

### 2️⃣ Simulate a mainnet transaction

```bash
curl -X POST http://localhost:8080/forks/b6f98e3b.../simulate \
  -H "Content-Type: application/json" \
  -d '{"tx_base64": "AgAAABF0L2eYv..."}'
```

### 3️⃣ Execute the same transaction (state-changing)

```bash
curl -X POST http://localhost:8080/forks/b6f98e3b.../execute \
  -H "Content-Type: application/json" \
  -d '{"tx_base64": "AgAAABF0L2eYv..."}'
```

### 4️⃣ Query balances

```bash
curl -X POST http://localhost:8080/forks/b6f98e3b.../get_account \
  -H "Content-Type: application/json" \
  -d '{"pubkey": "AgAAABF0L2eYv..."}'
```

---

## 🔧 Local Setup

### Build & Run

```bash
git clone https://github.com/18aaddy/solana-simulation-engine
cd solana-simulation-engine
cargo run
```

### Use the JavaScript scripts to test the Simulation Engine:
```bash
cd tests
npm install
# Test the API endpoints
node api_test.js
# Test transactions on the engine
node test_simulation_engine.js
```

### Default RPC:

```
http://127.0.0.1:8080
```

---
## 🧰 Tech Stack

| Component | Description |
|------------|-------------|
| **LiteSVM** | Solana Virtual Machine for local transaction execution |
| **Axum** | HTTP server and routing |
| **Solana SDK / Client** | Interfacing with Solana RPC |
| **UUID** | Fork ID generation |
| **Anyhow** | Error handling |
| **Serde / JSON** | Request/response serialization |

