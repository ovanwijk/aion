


//use std::str;
use std::{
    collections::{HashMap, VecDeque, hash_map::DefaultHasher},
    hash::{Hash, Hasher, BuildHasherDefault, BuildHasher},
};
use std::collections::LinkedList;
use crate::SETTINGS;
use riker::actors::*;
use riker::actors::Context;
use iota_lib_rs::iota_model::Transaction;
use aionmodel::tangle::*;
use indexstorage::Persistence;
use timewarping::zmqlistener::*;
use timewarping::Protocol;
use timewarping::Timewarp;
use timewarping::signing;
use timewarping::timewarpwalker::*;
//use std::collections::HashMap;
use indexstorage::*;
#[macro_use]
use log;
use std::sync::Arc;





pub struct TimewarpIndexing {
    pub tangle:Tangle,
    avg_count: i64,
    avg_distance: f64,
    storage: Arc<dyn Persistence>,
    //Map containing TXID as key and TimewarpID as Value
    known_timewarp_tips: HashMap<String, String>,
    last_picked_tw: Option<TimewarpData>    
}

impl Actor for TimewarpIndexing {
      // we used the #[actor] attribute so CounterMsg is the Msg type
    type Msg = Protocol;

    fn recv(&mut self,
                ctx: &Context<Self::Msg>,
                msg: Self::Msg,
                sender: Sender) {

        // Use the respective Receive<T> implementation
         match msg {
            Protocol::RegisterZMQListener(__msg) => self.receive_registerzmqlistener(ctx, __msg, sender),
            Protocol::NewTransaction(__msg) => self.receive_newtransaction(ctx, __msg, sender),
            Protocol::TransactionConfirmed(__msg) => self.receive_transactionconfimation(ctx, __msg, sender),
            _ => ()
        }
    }
}

/**
 * Timewarp functionality
 */
impl TimewarpIndexing {
    fn cal_avarage_timeleap(&mut self, tx:&Transaction){
        let branch_step = self.tangle.get(&tx.branch_transaction);
        
        if branch_step.is_some() {
            let diff = tx.timestamp - branch_step.unwrap().timestamp;
            if diff > 0 { //Filterout direct references through bundes
                self.avg_distance = ((self.avg_distance * self.avg_count as f64) + diff as f64) / (self.avg_count + 1) as f64;
                self.avg_count = self.avg_count + 1;
                
            }
        }
        let trunk_step = self.tangle.get(&tx.trunk_transaction);
         if trunk_step.is_some() {
            let diff = tx.timestamp - trunk_step.unwrap().timestamp;
            if diff > 0 { //Filterout direct references through bundes
                self.avg_distance = ((self.avg_distance * self.avg_count as f64) + diff as f64) / (self.avg_count + 1) as f64;
                self.avg_count = self.avg_count + 1;
                //println!("AVG Timeleap {}", self.avg_distance);  
            }

        }
       // println!("AVG Timeleap {}", self.avg_distance);  
    }

    fn detect_timewarp(&mut self, tx:&Transaction) -> Option<Timewarp>{
        
        if !tx.tag.ends_with("TW"){
           return None;
        }
        let now = crate::now();
        if tx.timestamp < now - 120 || tx.timestamp > now + 120 {
            // warn!("Transaction timestamp out of bounds");
            return None;
        }
        let branch_step = self.tangle.get(&tx.branch_transaction);
        //let mut branch_mismatch = false;
        if branch_step.is_some() {
            let branch = branch_step.unwrap();
            let diff = tx.timestamp - branch.timestamp;
            if diff > 0 &&  //Filterout direct references through bundles
                diff > SETTINGS.timewarp_index_settings.detection_threshold_min_timediff_in_seconds &&
                diff < SETTINGS.timewarp_index_settings.detection_threshold_max_timediff_in_seconds {
                //Validate signature
                let tw_hash = signing::timewarp_hash(&tx.address, &tx.trunk_transaction, &tx.branch_transaction, &tx.tag);
                let valid_signature = signing::validate_tw_signature(&branch.address, &tw_hash, &tx.signature_fragments);
                if valid_signature {
                    //Found branch timewarp
                    return Some(Timewarp{
                        source_hash: tx.hash.clone(),
                        source_branch: tx.branch_transaction.clone(),
                        source_trunk: tx.trunk_transaction.clone(),
                        source_timestamp: tx.timestamp,
                        distance: diff,
                        trunk_or_branch: false
                    })
                }else{
                   info!("Invalid timewarp signature, ID: {}, ADDRESS: {}, TW: {}",tx.hash.clone(), &branch.address, tw_hash);
                   //branch_mismatch = true;
                }
                    
            }
        }
        let trunk_step = self.tangle.get(&tx.trunk_transaction);
        if trunk_step.is_some() {
            let trunk = trunk_step.unwrap();
            let diff = tx.timestamp - trunk.timestamp;
            if diff > 0 && 
                diff > SETTINGS.timewarp_index_settings.detection_threshold_min_timediff_in_seconds &&
                diff < SETTINGS.timewarp_index_settings.detection_threshold_max_timediff_in_seconds {
                //Found trunk timewarp
                let tw_hash = signing::timewarp_hash(&tx.address, &tx.trunk_transaction, &tx.branch_transaction, &tx.tag);
                let valid_signature = signing::validate_tw_signature(&trunk.address, &tw_hash, &tx.signature_fragments);
                if valid_signature {
                    //Found branch timewarp
                    return Some(Timewarp{
                        source_hash: tx.hash.clone(),
                        source_branch: tx.branch_transaction.clone(),
                        source_trunk: tx.trunk_transaction.clone(),
                        source_timestamp: tx.timestamp,
                        distance: diff,
                        trunk_or_branch: true
                    })
                }else{
                      info!("Invalid timewarp signature, ID: {}, ADDRESS: {}, TW: {}",tx.hash.clone(), &trunk.address, tw_hash);
                    
                }

                // return Some(Timewarp{
                //     source_tx: tx.hash.clone(),
                //     target_tx: trunk.hash.clone(),
                //     distance: diff,
                //     trunk_or_branch: true
                // })     
            }
        }
        None
    }
}



/**
 * Actor receive messages
 */
impl TimewarpIndexing {
    fn actor(storage:Arc<dyn Persistence>) -> Self {
        let last_picked =  &storage.get_last_picked_tw();
        TimewarpIndexing {
            tangle: Tangle::default(),
            avg_count: 1,
            avg_distance: 1000.0 ,//default 1 second
            storage: storage,
            known_timewarp_tips: HashMap::new(),
            last_picked_tw: last_picked.to_owned()
        }
    }
     fn receive_newtransaction(&mut self,
                _ctx: &Context<Protocol>,
                _msg: NewTransaction,
                _sender: Sender) {
        let cpy = _msg.tx.clone();
        
        self.tangle.insert(_msg.tx);
        self.cal_avarage_timeleap(&cpy);       
        self.tangle.maintain();
    }

/**
 * Handles receiving a confirmed transaction.
 * It calles detect timewarp and if one is found it will look into it's cache to see
 * if it references a previously found timewarp. If not it will start a timewarp walker
 */
    fn receive_transactionconfimation(&mut self,
                _ctx: &Context<Protocol>,
                _msg: String,
                _sender: Sender) {
        let tx = self.tangle.get(&_msg);
        if tx.is_some() {
            let cpy = tx.unwrap().clone();          
            let timewarp = self.detect_timewarp(&cpy);
            if timewarp.is_some() {
                let tw = timewarp.unwrap();
                if self.last_picked_tw.is_some() {
                    let unwrapped = self.last_picked_tw.as_ref().unwrap();
                    if &unwrapped.hash == tw.target_hash() {
                        info!("Found connecting timewarp");
                    }
                }
                if self.known_timewarp_tips.get(tw.target_hash()).is_some() {
                    info!("Found a known timewarp Old: {} and new {}", tw.target_hash().to_string(), tw.source_hash.to_string());
                    self.known_timewarp_tips.remove(tw.target_hash());
                    self.known_timewarp_tips.insert(tw.source_hash.to_string(), tw.target_hash().to_string());
                    self.storage.tw_detection_add_decision_data(tw.clone());
                    self.storage.tw_detection_add_to_index(get_time_key(&cpy.timestamp),
                        vec![(tw.source_hash.to_string(), tw.target_hash().to_string())]);
                   
                }else{
                    self.known_timewarp_tips.insert(tw.source_hash.to_string(), tw.target_hash().to_string());
                    info!("Found a new timewarp!!!!!, start following ToB {} - {} - {}", tw.trunk_or_branch , tw.source_hash.to_string(), tw.target_hash().to_string());
                    let my_actor3 = _ctx.actor_of(TimewarpWalker::props(self.storage.clone()), &format!("timewarp-walking-{}", tw.source_hash.to_string())).unwrap();
                    
                    my_actor3.tell(Protocol::StartTimewarpWalking(StartTimewarpWalking { 
                        source_hash: cpy.hash, 
                        source_branch: cpy.branch_transaction,
                        source_trunk: cpy.trunk_transaction,
                        source_timestamp: cpy.timestamp, 
                        trunk_or_branch: tw.trunk_or_branch,
                        last_picked_tw_tx: {
                           if self.last_picked_tw.is_some() {
                                let r = &self.last_picked_tw.as_ref().unwrap().hash;
                                r.to_owned()
                           }else{
                                String::from("")
                           }
                        }})
                        , Some(BasicActorRef::from(_ctx.myself())));
                }
            }
        }
        
        //println!("Receiving transaction {}", _msg.tx.id);               
    }

    fn receive_registerzmqlistener(&mut self,
                ctx: &Context<Protocol>,
                msg: RegisterZMQListener,
                _sender: Sender) {        
     
        let res =  msg.zmq_listener.try_tell(Protocol::RegisterRoutee, ctx.myself());

        println!("Registering {:?}", res);
    }

    pub fn props(storage:Arc<dyn Persistence>) -> BoxActorProd<TimewarpIndexing> {
        Props::new_args(TimewarpIndexing::actor, storage)
    }
}

