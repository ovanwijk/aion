

use std::collections::LinkedList;
use std::sync::Arc;

use riker::actors::*;
use riker::actors::Context;
use std::convert::TryInto;
use timewarping::Protocol;
use indexstorage::WarpWalk;
use crate::SETTINGS;
use indexstorage::{get_time_key, Persistence};
//use iota_client::options::;
use iota_lib_rs::prelude::*;
use iota_lib_rs::iota_client::*;
use iota_model::Transaction;
use iota_conversion::trytes_converter;
use std::collections::HashMap;




#[derive(Clone, Debug)]
pub struct StartTimewarpWalking {
    pub target_hash: String,
    pub source_timestamp: i64,
    pub trunk_or_branch: bool,
    pub last_picked_tw_tx: String
}

impl Default for StartTimewarpWalking {
    fn default() -> StartTimewarpWalking{
        StartTimewarpWalking{
            target_hash:String::from(""),
            source_timestamp:0,
            trunk_or_branch:false,
            last_picked_tw_tx: String::from("")
        }
    }
}


#[derive(Debug)]
pub struct TimewarpWalker {
    start: String,
    path: LinkedList<WarpWalk>,
    storage_actor:Arc<dyn Persistence>,
    timewarp_state: StartTimewarpWalking,
    reply_to: Option<BasicActorRef>
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
    fn actor(storage_actor:Arc<dyn Persistence>) -> Self {        
        TimewarpWalker {
            start: "".to_string(),
            path: LinkedList::new(),
            storage_actor: storage_actor,
            timewarp_state: StartTimewarpWalking::default(),
            reply_to: None
        }
    }
    pub fn props(storage_actor:Arc<dyn Persistence>) -> BoxActorProd<TimewarpWalker> {
        Props::new_args(TimewarpWalker::actor, storage_actor)
    }

    fn receive_startwalking(&mut self,
                _ctx: &Context<Protocol>,
                _msg: StartTimewarpWalking,
                _sender: Sender) {
            
            self.timewarp_state = _msg.clone();
           // self.reply_to = _sender;
            let result = self.walk(_msg);
            if _sender.is_some() {
                let _l = _sender.unwrap().try_tell(Protocol::TimewarpWalkingResult(result), None);
            }
            _ctx.stop(_ctx.myself());
            //let result = self.walk(_msg);
            //println!("Found timewalk with depth: {}", result.len());
    }


    fn walk(&mut self, timewalk:StartTimewarpWalking) -> Vec<WarpWalk>{
        let mut txid = timewalk.target_hash.clone();
        let mut timestamp = timewalk.source_timestamp.clone();
        let mut iota = iota_client::Client::new("http://localhost:14265"); //TODO get from settings
        let mut finished = false;   
        let mut to_return:Vec<WarpWalk> = Vec::new();    

      
        while finished == false {
            let result = iota.get_trytes(&[txid.to_owned()]);
            if result.is_ok() {
               let tx_trytes = &result.unwrap_or_default().take_trytes().unwrap_or_default()[0];
               let tx:Transaction = tx_trytes.parse().unwrap_or_default();
               if tx.hash != "999999999999999999999999999999999999999999999999999999999999999999999999999999999" {

                    let diff = timestamp - tx.timestamp;
                    if diff >= SETTINGS.timewarp_index_settings.detection_threshold_min_timediff_in_seconds 
                        && diff <= SETTINGS.timewarp_index_settings.detection_threshold_max_timediff_in_seconds {
                        
                        to_return.push(WarpWalk{
                            from: txid.clone(),
                            timestamp: timestamp,
                            distance: diff,
                            target: if timewalk.trunk_or_branch {tx.trunk_transaction.clone() } else {tx.branch_transaction.clone()},
                            trunk_or_branch: timewalk.trunk_or_branch
                        });

                        txid = if timewalk.trunk_or_branch {tx.trunk_transaction.clone() } else {tx.branch_transaction.clone()};
                        timestamp = tx.timestamp.clone().try_into().unwrap();
                        if txid == timewalk.last_picked_tw_tx {
                            info!("Found last picked timewarp, stopped searching");
                            finished = true;
                        }
                        let keys = self.storage_actor.tw_detection_get(&get_time_key(&timestamp));
                        if keys.contains_key(&txid){
                            finished = true;
                        }
                        //found the timewarp!
                    }else{
                        //nothing to see possibly beginning of timewarp. Set timewarpID
                        finished = true;
                    }
                    
               }else{
                   //nothing to see possibly beginning of timewarp. Set timewarpID
                   finished = true;
               }
               println!("Got transaction!");
               
           }
        }
        for v in &to_return {
            let _res = self.storage_actor.tw_detection_add_to_index(get_time_key(&v.timestamp), vec![(v.from.to_string(), v.target.to_string())]);
        }

        to_return      
  
    }


  
}

