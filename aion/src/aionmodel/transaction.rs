use std::{
    collections::{HashMap, VecDeque, hash_map::DefaultHasher},
    hash::{Hash, Hasher, BuildHasherDefault, BuildHasher},
};
use iota_lib_rs::iota_conversion;
use iota_lib_rs::iota_conversion::*;
use iota_lib_rs::iota_model::Transaction;

// #[derive(Clone, Debug, Hash)]
// pub struct Transaction {
//     pub id: String,
//     pub timestamp: i64,
//     pub branch:String,
//     pub trunk: String,
//     pub tag: String,
//     pub signature: String,
// }

pub fn parse_zmqtransaction (tx_string:&str) -> iota_lib_rs::iota_model::Transaction {
    let split: Vec<&str> = tx_string.split(" ").collect();
    let trytes = split[1];
    let mut transaction = Transaction::default();
    let transaction_trits = trytes.trits();
        transaction.hash = String::from(split[2]);
        transaction.signature_fragments = trytes[0..2187].into();
        transaction.address = trytes[2187..2268].into();
        //transaction.value = iota_conversion::long_value(&transaction_trits[6804..6837]);
        //transaction.obsolete_tag = trytes[2295..2322].into();
        transaction.timestamp = iota_conversion::long_value(&transaction_trits[6966..6993]);
        // transaction.current_index =
        //     iota_conversion::long_value(&transaction_trits[6993..7020]) as usize;
        // transaction.last_index =
        //     iota_conversion::long_value(&transaction_trits[7020..7047]) as usize;
        // transaction.bundle = trytes[2349..2430].into();
        transaction.trunk_transaction = trytes[2430..2511].into();
        transaction.branch_transaction = trytes[2511..2592].into();

        transaction.tag = trytes[2592..2619].into();
        // transaction.attachment_timestamp =
        //     iota_conversion::long_value(&transaction_trits[7857..7884]);
        // transaction.attachment_timestamp_lower_bound =
        //     iota_conversion::long_value(&transaction_trits[7884..7911]);
        // transaction.attachment_timestamp_upper_bound =
        //     iota_conversion::long_value(&transaction_trits[7911..7938]);
        // transaction.nonce = trytes[2646..2673].into();
    transaction

    // Transaction {
    //     id: split[1].to_string(),
    //     timestamp: split[5].parse::<i64>().unwrap(),
    //     branch: split[10].to_string(),
    //     trunk: split[9].to_string(),
    //     tag: "".to_string(),
    //     signature: "".to_string()
    // }
}


pub fn parse_zmq_confirmation_transaction (tx_string:&str) -> &str {
    let split: Vec<&str> = tx_string.split(" ").collect();    
    split[2]
}