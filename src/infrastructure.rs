use std::collections::HashMap;

use crate::domain::{Account, ClientId};

#[derive(Debug)]
pub struct AccountRepository {
    accounts: HashMap<ClientId, Account>,
}

#[allow(clippy::new_without_default)]
impl AccountRepository {
    pub fn new() -> Self {
        unimplemented!()
    }

    pub fn get(&self, _client_id: ClientId) -> Option<&Account> {
        unimplemented!()
    }

    pub fn get_mut(&mut self, _client_id: ClientId) -> Option<&mut Account> {
        unimplemented!()
    }

    pub fn save(&mut self, _account: Account) {
        unimplemented!()
    }

    pub fn accounts(&self) -> Vec<&Account> {
        unimplemented!()
    }
}
