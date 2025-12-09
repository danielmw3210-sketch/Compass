// src/coinmint.rs

use crate::wallet::Wallet;

pub fn mint_one_coin(wallet: &mut Wallet) {
    wallet.deposit(1);
}