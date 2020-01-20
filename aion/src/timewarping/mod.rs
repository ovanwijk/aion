pub mod zmqlistener;
pub mod timewarpindexing;
pub mod timewarpwalker;
pub mod signing;
pub mod timewarpissuing;
pub mod timewarpselecting;
pub mod transactionpinning;
pub mod transactionpulling;
use serde::{Serialize};



#[derive(Clone, Debug)]
pub enum Protocol {
    RegisterRoutee,
    Start,
    Ping,
    Pong,
    Timer,
    Ready,
    StartTimewarpWalking(timewarpwalker::StartTimewarpWalking),
    TimewarpWalkingResult(String, Vec<Timewarp>),
    PullTxData(zmqlistener::PullTxData),
    StartListening(zmqlistener::StartListening),
    NewTransaction(zmqlistener::NewTransaction),
    TransactionConfirmed(String),
    RegisterZMQListener(zmqlistener::RegisterZMQListener),
    WebReply(String),
    WebRequest(WebRequestType)

}


#[derive(Clone, Debug, PartialEq)]
pub enum WebRequestType {
    PickedTimewarp,
    UnpinnedLifelife
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

