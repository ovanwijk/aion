


use std::sync::Arc;

use riker::actors::*;
use riker::actors::Context;
use std::convert::TryInto;
use crate::timewarping::{Protocol, Timewarp};
//use indexstorage::WarpWalk;

use crate::SETTINGS;
use crate::indexstorage::{get_time_key, Persistence};
//use iota_client::options::;
use iota_lib_rs::prelude::*;
//use iota_lib_rs::iota_client::*;
use iota_model::Transaction;
use crate::timewarping::signing;
// use iota_conversion::trytes_converter;
// use std::collections::HashMap;



//TODO include timewarp in fields, same sinature
#[derive(Clone, Debug)]
pub struct StartTimewarpWalking {
    pub source_hash: String,
    pub source_timestamp: i64,
    pub source_tag: String,
    pub source_branch: String,
    pub source_trunk: String,
    //pub source_signature: String,
    pub distance: i64,
    pub trunk_or_branch: bool,
    pub last_picked_tw_tx: String
}

impl StartTimewarpWalking {
    pub fn id(&self) -> String {
        format!("{} | {}",&self.source_hash, self.target_hash())
    }
    pub fn target_hash(&self) -> &String {
        if self.trunk_or_branch {
            &self.source_trunk
        }else{
            &self.source_branch
        }
    }
}

impl Default for StartTimewarpWalking {
    fn default() -> StartTimewarpWalking{
        StartTimewarpWalking{
            source_hash:String::from(""),
            source_branch:String::from(""),
            source_trunk:String::from(""),
            source_tag:String::from(""),
           // source_signature:String::from(""),
            distance: 0,
            source_timestamp:0,
            trunk_or_branch:false,
            last_picked_tw_tx: String::from("")
        }
    }
}


#[derive(Debug)]
pub struct TimewarpWalker {
    start: String,
    storage_actor:Arc<dyn Persistence>,
    timewarp_state: StartTimewarpWalking,
    node: String,
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
        let node = &SETTINGS.node_settings.iri_connection(); 
        TimewarpWalker {
            start: "".to_string(),
            storage_actor: storage_actor,
            timewarp_state: StartTimewarpWalking::default(),
            reply_to: None,
            node: node.to_string()
        }
    }
    pub fn props(storage_actor:Arc<dyn Persistence>) -> BoxActorProd<TimewarpWalker> {
        Props::new_args(TimewarpWalker::actor, storage_actor)
    }
 
    fn receive_startwalking(&mut self,
                _ctx: &Context<Protocol>,
                _msg: StartTimewarpWalking,
                _sender: Sender) {
            
            info!("Start walking");
            self.timewarp_state = _msg.clone();
            // self.reply_to = _sender;
            let id = _msg.id();
            let result = self.walk(_msg);
            if _sender.is_some() {
                info!("Done walking, sending result: {}", result.len());
                let _l = _sender.unwrap().try_tell(Protocol::TimewarpWalkingResult(id, result), _ctx.myself());
            }
            info!("Killing self");
            _ctx.stop(_ctx.myself());
            //let result = self.walk(_msg);
            //println!("Found timewalk with depth: {}", result.len());
    }


    fn walk(&mut self, timewalk:StartTimewarpWalking) -> Vec<Timewarp>{
        let mut txid = timewalk.target_hash().clone();
        let mut timestamp = timewalk.source_timestamp.clone();
        let mut iota = iota_client::Client::new(&self.node); //TODO get from settings
        let mut finished = false;   
        let mut to_return:Vec<Timewarp> = Vec::new();
        let mut api_cache: Option<Transaction> = None;

        to_return.push(Timewarp{
            source_hash: timewalk.source_hash.clone(),
            source_timestamp: timewalk.source_timestamp,
            distance: timewalk.distance,
            source_tag: timewalk.source_tag.clone(),
            source_branch: timewalk.source_branch.clone(),
            source_trunk: timewalk.source_trunk.clone(),                                
            trunk_or_branch: timewalk.trunk_or_branch
        });
      
        while finished == false {
            let tx_ = if api_cache.is_some() && api_cache.clone().expect("Value in api_cache").hash == txid.to_owned() {
               
                Some(api_cache.clone().expect("Value in api_cache"))
            }else{
                let result = iota.get_trytes(&[txid.to_owned()]);
                if result.is_ok() {
                    let tx_trytes = &result.unwrap_or_default().take_trytes().unwrap_or_default()[0];
                    let tx:Transaction = crate::aionmodel::transaction::parse_tx_trytes(&tx_trytes, &txid);
                    //We only care about signed messages
                    if tx.signature_fragments == "" ||  tx.signature_fragments.starts_with("999999999999999999999999999999999999999999999999999999999999999999999999999999999") {
                        //start of timewarp
                        None
                    }else{
                        Some(tx)
                    }
                }else{
                    None
                }
            };
            
            if tx_.is_some() {              
                let tx:Transaction = tx_.expect("Transaction");
                let diff = timestamp - tx.attachment_timestamp;
                if diff >= SETTINGS.timewarp_index_settings.detection_threshold_min_timediff_in_seconds 
                    && diff <= SETTINGS.timewarp_index_settings.detection_threshold_max_timediff_in_seconds {
                    //TODO Validate signature
                    let target_tx = if timewalk.trunk_or_branch {tx.trunk_transaction.clone() } else {tx.branch_transaction.clone()};
                    let result = iota.get_trytes(&[target_tx.to_owned()]);
                    let walked_tx = if result.is_ok() {
                        let tx_trytes = &result.unwrap_or_default().take_trytes().unwrap_or_default()[0];
                        let tx:Transaction = crate::aionmodel::transaction::parse_tx_trytes(&tx_trytes, &target_tx);
                        //We only care about signed messages
                        if tx.signature_fragments == "" || tx.signature_fragments.starts_with("999999999999999999999999999999999999999999999999999999999999999999999999999999999") {
                            None
                        }else{
                            Some(tx)
                        }
                    }else{
                        None
                    };
                    if walked_tx.is_some() {                        
                        let walked_unwrapped = walked_tx.expect("Walked transacion");
                        let tw_hash = signing::timewarp_hash(&tx.address, &tx.trunk_transaction, &tx.branch_transaction, &tx.tag);
                        let valid_signature = signing::validate_tw_signature(&walked_unwrapped.address, &tw_hash, &tx.signature_fragments);
                        if valid_signature {
                            to_return.push(Timewarp{
                                source_hash: txid.clone(),
                                source_timestamp: timestamp,
                                distance: diff,
                                source_tag: tx.tag.clone(),
                                source_branch: tx.branch_transaction.clone(),
                                source_trunk: tx.trunk_transaction.clone(),                                
                                trunk_or_branch: timewalk.trunk_or_branch
                            });
                            txid = if timewalk.trunk_or_branch {tx.trunk_transaction.clone() } else {tx.branch_transaction.clone()};
                            timestamp = tx.attachment_timestamp.clone().try_into().unwrap();
                            if txid == timewalk.last_picked_tw_tx {
                                info!("Found last picked timewarp, stopped searching");
                                finished = true;
                            }
                            let keys = self.storage_actor.tw_detection_get(&get_time_key(&timestamp));
                            if keys.contains_key(&txid){
                                info!("Found reference to already detected timewarp transaction.");
                                finished = true;
                            }
                            info!("Found and added {}/{}, ID:{}, ADDRESS:{}, TW: {}",to_return.len(),crate::SETTINGS.timewarp_index_settings.max_walking_depth_timewarp, tx.hash.clone(), &walked_unwrapped.address, tw_hash);
                        }else{
                            info!("Invalid timewarp signature, ID: {} ADDRESS:{} TW: {}",tx.hash.clone(), &walked_unwrapped.address, tw_hash);
                            finished = true;
                        }
                        api_cache.replace(walked_unwrapped);
                        //api_cache = Some(walked_unwrapped);
                    }else{
                         info!("Walked transaction not found! {}", target_tx.to_owned());
                        finished = true;
                    }
                    //found the timewarp!
                }else{
                    //nothing to see possibly beginning of timewarp. Set timewarpID
                    info!("Transaction not found! {}", txid.to_owned());
                    finished = true;
                }
                   
             
               println!("Got transaction!");
               
           }else{
            //nothing to see possibly beginning of timewarp. Set timewarpID
            info!("Is none {}", txid.to_owned());
            finished = true;
        }
        if to_return.len() >= crate::SETTINGS.timewarp_index_settings.max_walking_depth_timewarp as usize {
            finished = true;
            info!("Stopped at {} walking depth", crate::SETTINGS.timewarp_index_settings.max_walking_depth_timewarp);
        }
        }
       

        to_return      
  
    }


  
}

