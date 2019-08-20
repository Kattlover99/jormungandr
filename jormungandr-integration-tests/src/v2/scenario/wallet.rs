use crate::v2::scenario::NodeAlias;
use chain_impl_mockchain::value::Value;

pub type WalletAlias = String;

#[derive(Debug)]
pub enum WalletType {
    Account,
    UTxO,
}

#[derive(Debug)]
pub struct Wallet {
    alias: WalletAlias,
    value: Value,
    wallet_type: WalletType,
    delegate: Option<NodeAlias>,
}

impl Wallet {
    pub fn new_account<S: Into<WalletAlias>>(alias: S, value: Value) -> Self {
        Self::new(alias, value, WalletType::Account)
    }
    pub fn new_utxo<S: Into<WalletAlias>>(alias: S, value: Value) -> Self {
        Self::new(alias, value, WalletType::UTxO)
    }

    #[inline]
    fn new<S: Into<WalletAlias>>(alias: S, value: Value, wallet_type: WalletType) -> Self {
        Wallet {
            alias: alias.into(),
            value,
            wallet_type,
            delegate: None,
        }
    }

    pub fn alias(&self) -> &WalletAlias {
        &self.alias
    }

    pub fn wallet_type(&self) -> &WalletType {
        &self.wallet_type
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn delegate(&self) -> &Option<NodeAlias> {
        &self.delegate
    }

    pub fn delegate_mut(&mut self) -> &mut Option<NodeAlias> {
        &mut self.delegate
    }
}
