pub mod zmqlistener;
pub mod timewarpindexing;

#[derive(Clone, Debug)]
pub enum Protocol {
    StartListening(zmqlistener::StartListening),
    RegisterRoutee,
    Start,
    PullTxData(zmqlistener::PullTxData),
    NewTransaction(zmqlistener::NewTransaction),
    RegisterZMQListener(timewarpindexing::RegisterZMQListener)
}

pub struct Timewarp {
    from:String,
    to:String,
    distance:i64, //timestamp difference
    trunk:bool,
    branch:bool,
}

impl Timewarp {
    pub fn trunkOrBranch(&self) -> bool {
        if self.trunk {
            true
        } else {
            false
        }
    }
}