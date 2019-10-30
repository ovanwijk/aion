use std::{
    collections::{HashMap, VecDeque, hash_map::DefaultHasher},
    hash::{Hash, Hasher, BuildHasherDefault, BuildHasher},
};



#[derive(Clone, Debug, Hash)]
pub struct Transaction {
    pub id: String,
    pub timestamp: i64,
    pub branch:String,
    pub trunk: String
}

pub fn parse_zmqtransaction (tx_string:&str) -> Transaction {
    let split: Vec<&str> = tx_string.split(" ").collect();
    Transaction {
        id: split[1].to_string(),
        timestamp: split[5].parse::<i64>().unwrap(),
        branch: split[10].to_string(),
        trunk: split[9].to_string()
    }
}


pub fn parse_zmq_confirmation_transaction (tx_string:&str) -> &str {
    let split: Vec<&str> = tx_string.split(" ").collect();    
    split[2]
}