

use std::collections::LinkedList;
use riker::actors::*;
use riker::actors::Context;
use std::convert::TryInto;
use timewarping::Protocol;
use crate::SETTINGS;
//use iota_client::options::;
use iota_lib_rs::prelude::*;
use iota_lib_rs::iota_client::*;
use iota_model::Transaction;
use iota_conversion::trytes_converter;
use std::str::FromStr;



#[derive(Clone, Debug)]
pub struct StartTimewarpWalking {
    pub target_hash: String,
    pub source_timestamp: usize,
    pub trunk_or_branch: bool
}


#[derive(Clone, Debug)]
pub struct WarpWalk {
    distance: usize,
    trunk_or_branch:bool,
    target: String
}

#[derive(Debug)]
pub struct TimewarpWalker {
    start: String,
    path: LinkedList<WarpWalk>,
    storage_actor:BasicActorRef
}

impl Actor for TimewarpWalker {
      // we used the #[actor] attribute so CounterMsg is the Msg type
    type Msg = Protocol;

    fn recv(&mut self,
                ctx: &Context<Self::Msg>,
                msg: Self::Msg,
                sender: Sender) {

        // Use the respective Receive<T> implementation
        match msg {
            Protocol::StartTimewarpWalking(__msg) => self.receive_startwalking(ctx, __msg, sender),           
            _ => ()
        }
    }
   
    
}



impl TimewarpWalker {
    fn actor(storage_actor:BasicActorRef) -> Self {        
        TimewarpWalker {
            start: "".to_string(),
            path: LinkedList::new(),
            storage_actor: storage_actor
        }
    }
     pub fn props(storage_actor:BasicActorRef) -> BoxActorProd<TimewarpWalker> {
        Props::new_args(TimewarpWalker::actor, storage_actor)
    }
    fn receive_startwalking(&mut self,
                _ctx: &Context<Protocol>,
                _msg: StartTimewarpWalking,
                _sender: Sender) {
            

            let result = self.walk(_msg);
            println!("Found timewalk with depth: {}", result.len());
    }

    fn walk(&mut self, timewalk:StartTimewarpWalking) -> LinkedList<WarpWalk>{
        let mut txid = timewalk.target_hash.clone();
        let mut timestamp = timewalk.source_timestamp.clone();
        let mut iota = iota_client::Client::new("http://localhost:14265"); 
        let mut finished = false;   
        let mut to_return:LinkedList<WarpWalk> = LinkedList::new();    

      
        while finished == false {
            let result = iota.get_trytes(&[txid.to_owned()]);
            if result.is_ok() {
               let tx_trytes = &result.unwrap_or_default().take_trytes().unwrap_or_default()[0];
               let tx:Transaction = tx_trytes.parse().unwrap_or_default();
               if tx.hash != "999999999999999999999999999999999999999999999999999999999999999999999999999999999" {

                    let diff = timestamp - tx.timestamp as usize;
                    if diff >= SETTINGS.timewarp_index_settings.detection_threshold_min_timediff_in_seconds 
                        && diff <= SETTINGS.timewarp_index_settings.detection_threshold_max_timediff_in_seconds {
                        
                        to_return.push_back(WarpWalk{
                            distance: diff,
                            target: txid.clone(),
                            trunk_or_branch: timewalk.trunk_or_branch
                        });

                        txid = if timewalk.trunk_or_branch {tx.trunk_transaction.clone() } else {tx.branch_transaction.clone()};
                        timestamp = tx.timestamp.clone().try_into().unwrap();
                        //found the timewarp!
                    }else{
                        //nothing to see
                        finished = true;
                    }
                    
               }else{
                   finished = true;
               }
               println!("Got transaction!");
               
           }
        }

        to_return      
  
    }


  
}

