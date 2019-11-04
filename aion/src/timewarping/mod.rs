pub mod zmqlistener;
pub mod timewarpindexing;
pub mod timewarpwalker;
use serde::{Serialize, Deserialize};
use indexstorage;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum Protocol {
    RegisterRoutee,
    Start,
    StartTimewarpWalking(timewarpwalker::StartTimewarpWalking),
    TimewarpWalkingResult(String, String),
    PullTxData(zmqlistener::PullTxData),
    StartListening(zmqlistener::StartListening),
    NewTransaction(zmqlistener::NewTransaction),
    TransactionConfirmed(String),
    RegisterZMQListener(timewarpindexing::RegisterZMQListener),
    AddToIndexPersistence(i64, Vec<(String, String)>),
    GetFromIndexPersistence(i64),
    GetFromIndexPersistenceResponse(i64, HashMap<String, String>),
    RemoveFromIndexPersistence(indexstorage::RangeTxIDLookup)
}

pub struct Timewarp {
    from:String,
    to:String,
    distance:i64, //timestamp difference
    trunk_or_branch:bool
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimewarpDetectionData {
    timewarpid: String,
    source: String,
    target: String,
    distance: i64,
    trunk_or_branch: bool,
    timestamp: i64,
    timestamp_deviation_factor:f64,
    avg_distance: i64,
    index_since_id: i64,
}
