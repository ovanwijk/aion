pub mod zmqlistener;
pub mod timewarpindexing;
pub mod timewarpwalker;
pub mod signing;
pub mod timewarpissuing;
use serde::{Serialize, Deserialize};
use indexstorage;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum Protocol {
    RegisterRoutee,
    Start,
    Timer,
    StartTimewarpWalking(timewarpwalker::StartTimewarpWalking),
    TimewarpWalkingResult(Vec<indexstorage::WarpWalk>),
    PullTxData(zmqlistener::PullTxData),
    StartListening(zmqlistener::StartListening),
    NewTransaction(zmqlistener::NewTransaction),
    TransactionConfirmed(String),
    RegisterZMQListener(zmqlistener::RegisterZMQListener)
}

pub struct Timewarp {
    from:String,
    to:String,
    distance:i64, //timestamp difference
    trunk_or_branch:bool
}

