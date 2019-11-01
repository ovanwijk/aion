

use std::collections::LinkedList;
use riker::actors::*;
use riker::actors::Context;
use std::convert::TryInto;
use timewarping::Protocol;
use crate::SETTINGS;
use indexstorage::get_time_key;
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
    pub trunk_or_branch: bool
}

impl Default for StartTimewarpWalking {
    fn default() -> StartTimewarpWalking{
        StartTimewarpWalking{
            target_hash:String::from(""),
            source_timestamp:0,
            trunk_or_branch:false
        }
    }
}


#[derive(Clone, Debug)]
pub struct WarpWalk {
    from: String,
    timestamp: i64,
    distance: i64,
    trunk_or_branch:bool,
    target: String
}

#[derive(Debug)]
pub struct TimewarpWalker {
    start: String,
    path: LinkedList<WarpWalk>,
    storage_actor:BasicActorRef,
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
            Protocol::GetFromIndexPersistenceResponse(_, __msg) => self.receive_keys(ctx, __msg, sender),  
            _ => ()
        }
    }
   
    
}



impl TimewarpWalker {
    fn actor(storage_actor:BasicActorRef) -> Self {        
        TimewarpWalker {
            start: "".to_string(),
            path: LinkedList::new(),
            storage_actor: storage_actor,
            timewarp_state: StartTimewarpWalking::default(),
            reply_to: None
        }
    }
    pub fn props(storage_actor:BasicActorRef) -> BoxActorProd<TimewarpWalker> {
        Props::new_args(TimewarpWalker::actor, storage_actor)
    }

    fn receive_keys(&mut self,
                _ctx: &Context<Protocol>,
                tx_set: HashMap<String, String>,
                _sender: Sender) {
            
        self.step_b(tx_set, BasicActorRef::from(_ctx.myself()));
                
    }

    fn receive_startwalking(&mut self,
                _ctx: &Context<Protocol>,
                _msg: StartTimewarpWalking,
                _sender: Sender) {
            
            self.timewarp_state = _msg.clone();
            self.reply_to = _sender;
            self.step_a(BasicActorRef::from(_ctx.myself()));
            //let result = self.walk(_msg);
            //println!("Found timewalk with depth: {}", result.len());
    }

    fn exit(&self){
        if self.reply_to.is_some() {
            info!("Finished timewarp walk with {}", self.path.len());
            for v in &self.path {
                let _res = self.storage_actor.try_tell(Protocol::AddToIndexPersistence(get_time_key(v.timestamp), vec![(v.from.to_string(), v.target.to_string())]), None);
            }
        }

    }

    fn step_b(&mut self, tx_set: HashMap<String, String>, myself:BasicActorRef) {
        if !tx_set.contains_key(&self.timewarp_state.target_hash) {
            self.step_a(myself);
        }else{
            self.exit();
        }
    }

    fn step_a(&mut self, myself:BasicActorRef) {
            let timewalk = &self.timewarp_state;
            let txid = timewalk.target_hash.clone();
            let timestamp = timewalk.source_timestamp.clone();
            //TODO handle new creation all the time. Client now has a <`a> life time.
            let mut iota = iota_client::Client::new("http://localhost:14265"); 
       
      
            let result = iota.get_trytes(&[txid.to_owned()]);
            if result.is_ok() {
               let tx_trytes = &result.unwrap_or_default().take_trytes().unwrap_or_default()[0];
               let tx:Transaction = tx_trytes.parse().unwrap_or_default();
               if tx.hash != "999999999999999999999999999999999999999999999999999999999999999999999999999999999" {

                    let diff = timestamp - tx.timestamp;
                    if diff >= SETTINGS.timewarp_index_settings.detection_threshold_min_timediff_in_seconds 
                        && diff <= SETTINGS.timewarp_index_settings.detection_threshold_max_timediff_in_seconds {
                        
                        self.path.push_back(WarpWalk{
                            from: txid.clone(),
                            timestamp: timestamp,
                            distance: diff,
                            target: if timewalk.trunk_or_branch {tx.trunk_transaction.clone() } else {tx.branch_transaction.clone()},
                            trunk_or_branch: timewalk.trunk_or_branch
                        });
                        self.timewarp_state = StartTimewarpWalking{
                            source_timestamp: tx.timestamp.clone().try_into().unwrap(),
                            target_hash: if timewalk.trunk_or_branch {tx.trunk_transaction.clone() } else {tx.branch_transaction.clone()},
                            trunk_or_branch: timewalk.trunk_or_branch  
                        };
                        let _res = self.storage_actor.try_tell(Protocol::GetFromIndexPersistence(get_time_key(self.timewarp_state.source_timestamp)), myself);
                        //txid = if timewalk.trunk_or_branch {tx.trunk_transaction.clone() } else {tx.branch_transaction.clone()};
                        //timestamp = 
                        //found the timewarp!
                    }else{
                        self.exit()
                    }
               }else{
                        self.exit()
                    }
               println!("Got transaction!");
               
           }        
    }

    // fn walk(&mut self, timewalk:StartTimewarpWalking) -> LinkedList<WarpWalk>{
    //     let mut txid = timewalk.target_hash.clone();
    //     let mut timestamp = timewalk.source_timestamp.clone();
    //     let mut iota = iota_client::Client::new("http://localhost:14265"); 
    //     let mut finished = false;   
    //     let mut to_return:LinkedList<WarpWalk> = LinkedList::new();    

      
    //     while finished == false {
    //         let result = iota.get_trytes(&[txid.to_owned()]);
    //         if result.is_ok() {
    //            let tx_trytes = &result.unwrap_or_default().take_trytes().unwrap_or_default()[0];
    //            let tx:Transaction = tx_trytes.parse().unwrap_or_default();
    //            if tx.hash != "999999999999999999999999999999999999999999999999999999999999999999999999999999999" {

    //                 let diff = timestamp - tx.timestamp;
    //                 if diff >= SETTINGS.timewarp_index_settings.detection_threshold_min_timediff_in_seconds 
    //                     && diff <= SETTINGS.timewarp_index_settings.detection_threshold_max_timediff_in_seconds {
                        
    //                     to_return.push_back(WarpWalk{
    //                         distance: diff,
    //                         target: txid.clone(),
    //                         trunk_or_branch: timewalk.trunk_or_branch
    //                     });

    //                     txid = if timewalk.trunk_or_branch {tx.trunk_transaction.clone() } else {tx.branch_transaction.clone()};
    //                     timestamp = tx.timestamp.clone().try_into().unwrap();
    //                     //found the timewarp!
    //                 }else{
    //                     //nothing to see
    //                     finished = true;
    //                 }
                    
    //            }else{
    //                finished = true;
    //            }
    //            println!("Got transaction!");
               
    //        }
    //     }

    //     to_return      
  
    // }


  
}

