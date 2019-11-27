pub mod zmqlistener;
pub mod timewarpindexing;
pub mod timewarpwalker;
pub mod signing;
pub mod timewarpissuing;
use serde::{Serialize, Deserialize};



#[derive(Clone, Debug)]
pub enum Protocol {
    RegisterRoutee,
    Start,
    Timer,
    StartTimewarpWalking(timewarpwalker::StartTimewarpWalking),
    TimewarpWalkingResult(String, Vec<Timewarp>),
    PullTxData(zmqlistener::PullTxData),
    StartListening(zmqlistener::StartListening),
    NewTransaction(zmqlistener::NewTransaction),
    TransactionConfirmed(String),
    RegisterZMQListener(zmqlistener::RegisterZMQListener)
}

#[derive(Clone, Debug)]
pub struct Timewarp {
    pub source_hash: String,
    pub source_branch: String,
    pub source_trunk: String,
    pub source_timestamp:i64,

    pub distance:i64, //timestamp difference
    pub trunk_or_branch:bool
}


impl Timewarp {
    pub fn target_hash(&self) -> &String {
        if self.trunk_or_branch {
            &self.source_trunk
        }else{
            &self.source_branch
        }
    }
}

