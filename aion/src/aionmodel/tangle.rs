use aionmodel::transaction::*;
use crate::SETTINGS;
use std::{
    collections::{HashMap, HashSet, VecDeque, hash_map::DefaultHasher},
    hash::{Hash, Hasher, BuildHasherDefault, BuildHasher},
};


pub struct Tangle {
    pub txs: HashMap<String, Box<Transaction>>,
    pub txs_ages: VecDeque<String>,
    pub max_txs: usize,
    pub confirmed_txs: HashSet<String>
}


impl Tangle {  

    pub fn maintain(&mut self) {
        while self.txs_ages.len() > self.max_txs {
            let popping = &self.txs_ages.pop_front().unwrap();
            self.txs.remove(popping);
            self.confirmed_txs.remove(popping);
        }
    }

    pub fn insert(&mut self, tx: Transaction) {        
         if !self.contains(tx.id.to_string()) {
            self.txs_ages.push_back(tx.id.to_string());
            self.txs.insert(tx.id.to_string(), Box::new(tx));
            println!("Got transactions {}", self.txs.len());    
         } 
    }

    pub fn contains(&self, txid: String) -> bool {
        self.txs.contains_key(&txid)
    }
    
    pub fn is_certainly_confirmed(&self, txid:String) -> bool {
        self.confirmed_txs.get(&txid).is_some()
    }

    pub fn get(&self, txid: &String) -> Option<&Box<Transaction>> {
        let result = self.txs.get(txid);
        result        
    }
}

impl Default for Tangle {
    fn default() -> Self {
        Self {
            txs: HashMap::default(),
            txs_ages: VecDeque::default(),
            max_txs: SETTINGS.cache_settings.local_tangle_max_transactions,
            confirmed_txs: HashSet::default()
        }
    }
}