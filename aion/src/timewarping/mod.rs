pub mod zmqlistener;
pub mod timewarpindexing;
pub mod timewarpwalker;
use indexstorage;

#[derive(Clone, Debug)]
pub enum Protocol {
    RegisterRoutee,
    Start,
    StartTimewarpWalking(timewarpwalker::StartTimewarpWalking),
    PullTxData(zmqlistener::PullTxData),
    StartListening(zmqlistener::StartListening),
    NewTransaction(zmqlistener::NewTransaction),
    RegisterZMQListener(timewarpindexing::RegisterZMQListener),
    AddToIndexPersistence(indexstorage::TimewarpIndexEntry),
    GetFromIndexPersistence(u64),
    RemoveFromIndexPersistence(indexstorage::TimewarpIndexEntry)
}

pub struct Timewarp {
    from:String,
    to:String,
    distance:i64, //timestamp difference
    trunk_or_branch:bool
}
