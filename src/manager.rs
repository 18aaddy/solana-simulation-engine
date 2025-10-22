use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use litesvm::LiteSVM;
use solana_client::rpc_client::RpcClient;
use solana_program::example_mocks::solana_sdk::system_program;
use solana_sdk::{
    account::Account, clock::Clock, pubkey::Pubkey, slot_hashes::SlotHashes,
    transaction::VersionedTransaction,
};
use uuid::Uuid;

const DEFAULT_RPC_CLIENT: &str = "https://api.mainnet-beta.solana.com";

pub struct Fork {
    pub id: Uuid,
    // Expires 15 minutes after creation
    pub expires_at: Instant,
    pub svm: Arc<Mutex<LiteSVM>>,
}

impl Fork {
    pub fn new(id: Uuid, svm: Arc<Mutex<LiteSVM>>) -> Self {
        Fork {
            id,
            expires_at: Instant::now() + Duration::from_secs(15 * 60),
            svm,
        }
    }
}

pub struct SvmManager {
    pub forks: HashMap<Uuid, Arc<Fork>>,
}

impl SvmManager {
    pub fn new() -> Self {
        SvmManager {
            forks: HashMap::new(),
        }
    }

    pub fn create_fork(&mut self) -> anyhow::Result<Uuid> {
        let client = RpcClient::new(DEFAULT_RPC_CLIENT.to_string());
        let latest_blockhash = client.get_latest_blockhash()?;
        let slot = client.get_slot()?;

        let mut svm = LiteSVM::new().with_sysvars();

        let mut hash = svm.get_sysvar::<SlotHashes>().clone();
        hash.push((slot, latest_blockhash));
        svm.set_sysvar(&SlotHashes::new(&hash));

        let mut clock: Clock = svm.get_sysvar();
        clock.slot = slot;
        clock.unix_timestamp = chrono::Utc::now().timestamp();
        svm.set_sysvar(&clock);
        svm.set_sysvar(&SlotHashes::new(&hash));

        let fork_id = Uuid::new_v4();
        let fork = Fork::new(fork_id, Arc::new(Mutex::new(svm)));

        self.forks.insert(fork_id, Arc::new(fork));

        Ok(fork_id)
    }

    pub fn get_fork(&self, id: &Uuid) -> Option<Arc<Fork>> {
        self.forks.get(id).map(|entry| Arc::clone(entry))
    }

    pub fn delete_fork(&mut self, id: &Uuid) -> bool {
        self.forks.remove(id).is_some()
    }

    pub fn cleanup_expired(&mut self) {
        let now = Instant::now();
        let expired: Vec<Uuid> = self
            .forks
            .iter()
            .filter(|(_id, fork)| fork.expires_at <= now)
            .map(|(id, _fork)| *id)
            .collect();

        for id in expired {
            self.forks.remove(&id);
            println!("Cleaned up expired fork {}", id);
        }
    }

    pub fn execute_transaction(
        &self,
        fork_id: &Uuid,
        tx: VersionedTransaction,
    ) -> anyhow::Result<()> {
        if let Some(fork) = self.get_fork(fork_id) {
            let mut svm = fork.svm.lock().unwrap();
            match svm.send_transaction(tx) {
                Ok(_) => return Ok(()),
                Err(e) => return Err(anyhow::Error::new(e.err)),
            };
        } else {
            anyhow::bail!("Fork not found");
        }
    }

    pub fn set_lamports(
        &self,
        fork_id: &Uuid,
        pubkey: Pubkey,
        lamports: u64,
    ) -> anyhow::Result<()> {
        if let Some(fork) = self.get_fork(fork_id) {
            let mut svm = fork.svm.lock().unwrap();
            let mut account = match svm.get_account(&pubkey) {
                Some(acc) => acc,
                None => Account::new(0, 0, &system_program::ID),
            };
            account.lamports = lamports;
            svm.set_account(pubkey, account)?;
            Ok(())
        } else {
            anyhow::bail!("Fork not found");
        }
    }

    // TODO
    pub fn set_token_balances(
        &self,
        fork_id: &Uuid,
        pubkey: Pubkey,
        lamports: u64,
    ) -> anyhow::Result<()> {
        if let Some(fork) = self.get_fork(fork_id) {
            let mut svm = fork.svm.lock().unwrap();
            let mut account = match svm.get_account(&pubkey) {
                Some(acc) => acc,
                None => Account::new(0, 0, &system_program::ID),
            };
            account.lamports = lamports;
            svm.set_account(pubkey, account)?;
            Ok(())
        } else {
            anyhow::bail!("Fork not found");
        }
    }

    pub fn get_account(&self, fork_id: &Uuid, pubkey: Pubkey) -> anyhow::Result<Account> {
        if let Some(fork) = self.get_fork(fork_id) {
            let svm = fork.svm.lock().unwrap();
            match svm.get_account(&pubkey) {
                Some(acc) => return Ok(acc),
                None => return Err(anyhow::Error::msg("account not found")),
            };
        } else {
            anyhow::bail!("Fork not found");
        }
    }
}
