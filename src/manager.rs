use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use litesvm::{
    LiteSVM,
    types::{
        FailedTransactionMetadata, SimulatedTransactionInfo, TransactionMetadata,
    },
};
use solana_client::rpc_client::RpcClient;
use solana_program::example_mocks::solana_sdk::system_program;
use solana_sdk::{
    account::Account, clock::Clock, program_error::ProgramError, pubkey::Pubkey,
    slot_hashes::SlotHashes, transaction::VersionedTransaction,
};
use spl_token::solana_program::program_pack::Pack;
use spl_token::solana_program::pubkey;
use spl_token::{
    ID,
    state::{Account as TokenAccount, AccountState},
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

#[derive(Clone)]
pub struct ForkManager {
    pub forks: HashMap<Uuid, Arc<Fork>>,
}

impl ForkManager {
    pub fn new() -> Self {
        ForkManager {
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
    ) -> anyhow::Result<TransactionMetadata> {
        if let Some(fork) = self.get_fork(fork_id) {
            let mut svm = fork.svm.lock().unwrap();

            self.preload_missing_accounts(&mut svm, &tx);

            match svm.send_transaction(tx) {
                Ok(res) => return Ok(res),
                Err(e) => return Err(anyhow::Error::new(e.err)),
            };
        } else {
            anyhow::bail!("Fork not found");
        }
    }

    pub fn simulate_transaction(
        &self,
        fork_id: &Uuid,
        tx: VersionedTransaction,
    ) -> Result<SimulatedTransactionInfo, FailedTransactionMetadata> {
        if let Some(fork) = self.get_fork(fork_id) {
            let mut svm = fork.svm.lock().unwrap();

            self.preload_missing_accounts(&mut svm, &tx);

            svm.simulate_transaction(tx)
        } else {
            Err(FailedTransactionMetadata::from(ProgramError::Custom(0)))
        }
    }

    fn preload_missing_accounts(&self, svm: &mut LiteSVM, tx: &VersionedTransaction) {
        let client = RpcClient::new(DEFAULT_RPC_CLIENT.to_string());
        let account_keys = tx.message.static_account_keys();

        for key in account_keys {
            if svm.get_account(key).is_none() {
                if let Ok(acc) = client.get_account(key) {
                    let _ = svm.set_account(*key, acc);
                    println!("Loaded mainnet account {} into fork", key);
                } else {
                    println!("Warning: account {} not found on mainnet RPC", key);
                }
            }
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

    pub fn set_token_balance(
        &self,
        fork_id: &Uuid,
        token_account_pubkey: Pubkey,
        mint: Pubkey,
        owner: Pubkey,
        amount: u64,
    ) -> anyhow::Result<()> {
        if let Some(fork) = self.get_fork(fork_id) {
            let mut svm = fork.svm.lock().unwrap();

            let mut account = svm.get_account(&token_account_pubkey).unwrap_or_else(|| {
                Account::new(
                    1_000_000,
                    TokenAccount::LEN,
                    &Pubkey::new_from_array(*ID.as_array()),
                )
            });

            let mut token_acc = TokenAccount::default();
            token_acc.mint = pubkey::Pubkey::new_from_array(*mint.as_array());
            token_acc.owner = pubkey::Pubkey::new_from_array(*owner.as_array());
            token_acc.amount = amount;
            token_acc.state = AccountState::Initialized;

            let mut data = vec![0u8; TokenAccount::LEN];
            token_acc.pack_into_slice(&mut data);

            account.data = data;
            account.owner = Pubkey::new_from_array(*ID.as_array());
            account.executable = false;
            account.rent_epoch = 0;

            svm.set_account(token_account_pubkey, account)?;
            Ok(())
        } else {
            anyhow::bail!("Fork not found");
        }
    }

    pub fn get_account(&self, fork_id: &Uuid, pubkey: Pubkey) -> anyhow::Result<Account> {
        if let Some(fork) = self.get_fork(fork_id) {
            let mut svm = fork.svm.lock().unwrap();

            if let Some(acc) = svm.get_account(&pubkey) {
                println!("Account found locally!");
                return Ok(acc);
            }

            let client = RpcClient::new(DEFAULT_RPC_CLIENT.to_string());
            match client.get_account(&pubkey) {
                Ok(acc) => {
                    svm.set_account(pubkey, acc.clone())?;
                    println!("Account found on mainnet!");
                    Ok(acc)
                }
                Err(_) => anyhow::bail!("Account not found on mainnet or fork"),
            }
        } else {
            anyhow::bail!("Fork not found");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::{Keypair, Signer};
    use std::time::Duration;

    #[test]
    fn test_fork_creation() {
        let mut manager = ForkManager::new();
        let fork_id = manager.create_fork().expect("Failed to create fork");

        assert!(manager.forks.contains_key(&fork_id));
    }

    #[test]
    fn test_get_fork() {
        let mut manager = ForkManager::new();
        let fork_id = manager.create_fork().expect("Failed to create fork");

        let fork = manager.get_fork(&fork_id);
        assert!(fork.is_some());
    }

    #[test]
    fn test_delete_fork() {
        let mut manager = ForkManager::new();
        let fork_id = manager.create_fork().expect("Failed to create fork");

        let deleted = manager.delete_fork(&fork_id);
        assert!(deleted);
        assert!(!manager.forks.contains_key(&fork_id));
    }

    #[test]
    fn test_cleanup_expired() {
        let mut manager = ForkManager::new();
        let fork_id = manager.create_fork().expect("Failed to create fork");

        if let Some(fork) = manager.forks.get_mut(&fork_id) {
            let fork_mut = Arc::get_mut(fork).unwrap();
            fork_mut.expires_at = Instant::now() - Duration::from_secs(1);
        }

        assert_eq!(manager.forks.len(), 1);
        manager.cleanup_expired();
        assert_eq!(manager.forks.len(), 0);
    }

    #[test]
    fn test_set_lamports() {
        let mut manager = ForkManager::new();
        let fork_id = manager.create_fork().expect("Failed to create fork");

        let keypair = Keypair::new();
        let pubkey = keypair.pubkey();
        let lamports = 1_000_000;

        let result = manager.set_lamports(&fork_id, pubkey, lamports);
        assert!(result.is_ok());

        let account = manager
            .get_account(&fork_id, pubkey)
            .expect("Failed to get account");
        assert_eq!(account.lamports, lamports);
    }

    #[test]
    fn test_set_token_balance() {
        let mut manager = ForkManager::new();
        let fork_id = manager.create_fork().expect("Failed to create fork");

        let mint = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();

        manager
            .set_token_balance(&fork_id, token_account, mint, user, 1_000_000)
            .expect("Failed to set token balance");

        let account = manager.get_account(&fork_id, token_account).unwrap();
        let unpacked = TokenAccount::unpack(&account.data).unwrap();

        assert_eq!(
            unpacked.owner,
            pubkey::Pubkey::new_from_array(*user.as_array())
        );
        assert_eq!(
            unpacked.mint,
            pubkey::Pubkey::new_from_array(*mint.as_array())
        );
        assert_eq!(unpacked.amount, 1_000_000);
    }

    #[test]
    fn test_mainnet_fallback() {
        let mut manager = ForkManager::new();
        let fork_id = manager.create_fork().expect("Failed to create fork");

        // A well-known system account (system program)
        let address = Pubkey::from_str_const("7nZrcnwtxqGeSsYgyaTZrwrwDFEe39CVwxcGgZhBjgLa");

        // Should fetch from mainnet and cache
        let acc = manager.get_account(&fork_id, address).unwrap();
        assert!(acc.owner != Pubkey::default());

        // Should now be cached locally
        let acc2 = manager.get_account(&fork_id, address).unwrap();
        assert_eq!(acc.lamports, acc2.lamports);
    }
}
