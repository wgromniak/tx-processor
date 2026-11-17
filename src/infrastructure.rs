//! Concrete in-memory storage for account aggregates.

use std::collections::HashMap;

use crate::domain::{Account, ClientId};

#[derive(Debug)]
pub struct AccountRepository {
    accounts: HashMap<ClientId, Account>,
}

#[allow(
    clippy::new_without_default,
    reason = "Repository construction is a placeholder until storage is implemented"
)]
impl AccountRepository {
    #[must_use]
    pub fn new() -> Self {
        unimplemented!()
    }

    #[must_use]
    pub fn get(&self, _client_id: ClientId) -> Option<&Account> {
        unimplemented!()
    }

    pub fn get_mut(&mut self, _client_id: ClientId) -> Option<&mut Account> {
        unimplemented!()
    }

    pub fn save(&mut self, _account: Account) {
        unimplemented!()
    }

    #[must_use]
    pub fn accounts(&self) -> Vec<&Account> {
        unimplemented!()
    }
}
