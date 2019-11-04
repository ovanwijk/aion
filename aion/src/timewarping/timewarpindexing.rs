


//use std::str;
use std::{
    collections::{HashMap, VecDeque, hash_map::DefaultHasher},
    hash::{Hash, Hasher, BuildHasherDefault, BuildHasher},
};
use std::collections::LinkedList;
use crate::SETTINGS;
use riker::actors::*;
use riker::actors::Context;
use aionmodel::transaction::*;
use aionmodel::tangle::*;
use indexstorage::Persistence;
use timewarping::zmqlistener::*;
use timewarping::Protocol;
use timewarping::Timewarp;
use timewarping::timewarpwalker::*;
//use std::collections::HashMap;
use indexstorage::*;
#[macro_use]
use log;
use std::sync::Arc;



#[derive(Clone, Debug)]
pub struct RegisterZMQListener {
    pub zmq_listener: BasicActorRef
}


pub struct TimewarpIndexing {
    pub tangle:Tangle,
    avg_count: i64,
    avg_distance: f64,
    storage_actor: Arc<dyn Persistence>,
    //Map containing TXID as key and TimewarpID as Value
    known_timewarp_tips: HashMap<String, String>
    
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
        let branch_step = self.tangle.get(&tx.branch);
        
        if branch_step.is_some() {
            let diff = tx.timestamp - branch_step.unwrap().timestamp;
            if diff > 0 { //Filterout direct references through bundes
                self.avg_distance = ((self.avg_distance * self.avg_count as f64) + diff as f64) / (self.avg_count + 1) as f64;
                self.avg_count = self.avg_count + 1;
                
            }
        }
        let trunk_step = self.tangle.get(&tx.trunk);
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
        let branch_step = self.tangle.get(&tx.branch);
        
        if branch_step.is_some() {
            let diff = tx.timestamp - branch_step.unwrap().timestamp;
            if diff > 0 &&  //Filterout direct references through bundles
                diff > SETTINGS.timewarp_index_settings.detection_threshold_min_timediff_in_seconds &&
                diff < SETTINGS.timewarp_index_settings.detection_threshold_max_timediff_in_seconds {
                //Found branch timewarp
                 return Some(Timewarp{
                    from: tx.id.clone(),
                    to: branch_step.unwrap().id.clone(),
                    distance: diff,
                    trunk_or_branch: false
                })    
            }
        }
        let trunk_step = self.tangle.get(&tx.trunk);
        if trunk_step.is_some() {
            let diff = tx.timestamp - trunk_step.unwrap().timestamp;
            if diff > 0 && 
                diff > SETTINGS.timewarp_index_settings.detection_threshold_min_timediff_in_seconds &&
                diff < SETTINGS.timewarp_index_settings.detection_threshold_max_timediff_in_seconds {
                //Found trunk timewarp
                return Some(Timewarp{
                    from: tx.id.clone(),
                    to: trunk_step.unwrap().id.clone(),
                    distance: diff,
                    trunk_or_branch: true
                })     
            }
        }
        None
    }
}



/**
 * Actor receive messages
 */
impl TimewarpIndexing {
    fn actor(storage_actor:Arc<dyn Persistence>) -> Self {   
        TimewarpIndexing {
            tangle: Tangle::default(),
            avg_count: 1,
            avg_distance: 1000.0 ,//default 1 second
            storage_actor: storage_actor,
            known_timewarp_tips: HashMap::new()
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
                if self.known_timewarp_tips.get(&tw.to).is_some() {
                    info!("Found a known timewarp Old: {} and new {}", tw.to.to_string(), tw.from.to_string());
                    self.known_timewarp_tips.remove(&tw.to);
                    self.known_timewarp_tips.insert(tw.from.to_string(), tw.to.to_string());
                    self.storage_actor.tw_detection_add_to_index(get_time_key(&cpy.timestamp),
                        vec![(tw.from, tw.to)]);
                    // let _res = self.storage_actor.try_tell(Protocol::AddToIndexPersistence(
                       
                    // ), None);
                }else{
                    self.known_timewarp_tips.insert(tw.from.to_string(), tw.to.to_string());
                    info!("Found a new timewarp!!!!!, start following{}-{}", tw.from.to_string(), tw.to.to_string());
                    let my_actor3 = _ctx.actor_of(TimewarpWalker::props(self.storage_actor.clone()), &format!("timewarp-walking-{}", tw.from.to_string())).unwrap();
                    
                    

                    my_actor3.tell(Protocol::StartTimewarpWalking(StartTimewarpWalking { 
                        target_hash: tw.to, 
                        source_timestamp: cpy.timestamp, 
                        trunk_or_branch: tw.trunk_or_branch})
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

    pub fn props(storage_actor:Arc<dyn Persistence>) -> BoxActorProd<TimewarpIndexing> {
        Props::new_args(TimewarpIndexing::actor, storage_actor)
    }
}

