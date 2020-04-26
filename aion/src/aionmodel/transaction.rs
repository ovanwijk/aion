
use iota_lib_rs::iota_conversion;
use iota_lib_rs::iota_conversion::*;

#[derive(Clone, Debug, Hash)]
pub struct Transaction {
    pub id: String,
    pub timestamp: i64,
    pub trunk_transaction:String,
    pub branch_transaction: String,
    pub tag: String,
    pub signature_fragments: String,
}
// pub fn parse_zmqtransaction_ (tx_string:&str) -> Transaction {
//     let split: Vec<&str> = tx_string.split(" ").collect();
//     Transaction {
//         id: split[1].to_string(),
//         timestamp: split[5].parse::<i64>().unwrap(),
//         branch_transaction: split[10].to_string(),
//         trunk_transaction: split[9].to_string(),
//         tag: "".to_string(),
//         signature_fragments: "".to_string()
//     }
// }

pub trait TimewarpTX {
    fn timewarp_id(&self) -> &str;
    fn timewarp_index(&self) -> u64;
}

impl TimewarpTX for iota_lib_rs::iota_model::Transaction {
    fn timewarp_id(&self) -> &str {
        &self.tag[16..25]
    }
    fn timewarp_index(&self) -> u64 {
        iota_conversion::long_value(&self.tag[11..16].to_owned().trits()) as u64
    }
}

pub fn get_trunk_branch_ts_tag(trytes:&str) -> (String, String, i64, String) {
    let transaction_trits = trytes.trits();
    let attachment = iota_conversion::long_value(&transaction_trits[7857..7884]) / 1000;
    (trytes[2430..2511].into(), 
        trytes[2511..2592].into(),
        if attachment == 0 {iota_conversion::long_value(&transaction_trits[6966..6993])}else{
            attachment},
        trytes[2592..2619].into())
}

pub fn parse_tx_trytes(trytes:&str, hash:&str) -> iota_lib_rs::iota_model::Transaction {
    let mut transaction = iota_lib_rs::iota_model::Transaction::default();
    let tag:String =  trytes[2592..2619].into();
    let now = crate::now();
    let transaction_trits = trytes.trits();
    let attachment = iota_conversion::long_value(&transaction_trits[7857..7884]) / 1000;
    transaction.hash = String::from(hash);
    transaction.signature_fragments = if tag.ends_with("TW") { trytes[0..2187].into() } else { "".to_owned() };
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
    transaction.tag = tag;
    transaction.attachment_timestamp = if attachment == 0 {transaction.timestamp}else{
        attachment
    };
       //  iota_conversion::long_value(&transaction_trits[7857..7884]) / 1000;
    // transaction.attachment_timestamp_lower_bound =
    //     iota_conversion::long_value(&transaction_trits[7884..7911]);
    // transaction.attachment_timestamp_upper_bound =
    //     iota_conversion::long_value(&transaction_trits[7911..7938]);
    transaction.nonce = if transaction.timestamp < now - 120 || transaction.timestamp > now + 120 {
        // warn!("Transaction timestamp out of bounds");
        String::from("NO")
    }else{
        String::from("")
    };
    transaction
    
}

pub fn parse_zmqtransaction (tx_string:&str) -> iota_lib_rs::iota_model::Transaction {
    let split: Vec<&str> = tx_string.split(" ").collect();    
    parse_tx_trytes(split[1], split[2])

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