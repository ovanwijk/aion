


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
use crate::aionmodel::tangle::*;
use crate::indexstorage::Persistence;
use crate::timewarping::zmqlistener::*;
use crate::timewarping::timewarpselecting::TimewarpSelectionState;
use crate::timewarping::Protocol;
use crate::timewarping::Timewarp;
use crate::timewarping::signing;
use crate::timewarping::timewarpwalker::*;
//use std::collections::HashMap;
use crate::indexstorage::*;
#[macro_use]
use crate::log;
use std::sync::Arc;





pub struct TimewarpIndexing {
    pub tangle:Tangle,
    avg_count: i64,
    avg_distance: f64,
    storage: Arc<dyn Persistence>,
    timewarp_selecting: ActorRef<Protocol> ,
    //Map containing TXID as key and TimewarpID as Value
    //Timewarp ID is used when using the timewarp walker and cache intermediate progress on the timewarp.
    known_timewarp_tips: HashMap<String, String>,
    start_time: i64,
    known_active_walks: HashMap<String, Vec<Timewarp>>,    
    last_picked_tw: Option<TimewarpSelectionState>    
}

impl Actor for TimewarpIndexing { 
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
            Protocol::TimewarpWalkingResult(id, __msg) => self.receive_timewalkresult(ctx, id, __msg, sender),
           
            Protocol::Timer => {
                self.receive_timer(ctx, sender);
            },
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
            let diff = tx.attachment_timestamp - branch_step.unwrap().attachment_timestamp;
            if diff > 0 { //Filterout direct references through bundes
                self.avg_distance = ((self.avg_distance * self.avg_count as f64) + diff as f64) / (self.avg_count + 1) as f64;
                self.avg_count = self.avg_count + 1;
                
            }
        }
        let trunk_step = self.tangle.get(&tx.trunk_transaction);
         if trunk_step.is_some() {
            let diff = tx.attachment_timestamp - trunk_step.unwrap().attachment_timestamp;
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
        if tx.nonce == "NO" {
            // warn!("Transaction timestamp out of bounds");
            return None;
        }
        let branch_step = self.tangle.get(&tx.branch_transaction);
        //let mut branch_mismatch = false;
        if branch_step.is_some() {
            let branch = branch_step.unwrap();
            let diff = tx.attachment_timestamp - branch.attachment_timestamp;
            if diff > 0 &&  //Filterout direct references through bundles
                (tx.tag[15..] == branch.tag[15..] || tx.tag[15..] == branch.hash[0..9]) && // Check the if the tag endings are correct
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
                        source_timestamp: tx.attachment_timestamp,
                        source_tag: tx.tag.clone(),
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
            let diff = tx.attachment_timestamp - trunk.attachment_timestamp;
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
                        source_timestamp: tx.attachment_timestamp,
                        source_tag: tx.tag.clone(),
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

    fn ready_check(&mut self) -> bool {
        //We wait at least the time for switching to look at the tangle before starting to make any judgements.
        if crate::now() - self.start_time > SETTINGS.timewarp_index_settings.detection_threshold_switch_timewarp_in_seconds {
            if self.known_active_walks.is_empty() {
                return true;
            }
        }
        return false;
    }
}



/**
 * Actor receive messages
 */
impl TimewarpIndexing {
    fn actor(args:(Arc<dyn Persistence>, ActorRef<Protocol>)) -> Self {
        let last_picked =  &args.0.get_last_picked_tw();
        TimewarpIndexing {
            tangle: Tangle::default(),
            avg_count: 1,
          
            avg_distance: 1000.0 ,//default 1 second
            storage: args.0,
            start_time: crate::now(),
            known_timewarp_tips: HashMap::new(),
            known_active_walks: HashMap::new(),
            last_picked_tw: last_picked.to_owned(),
            timewarp_selecting: args.1
        }
    }


    pub fn receive_timer(&mut self,
        ctx: &Context<Protocol>,
        _sender: Sender) {
        
        if self.ready_check() {
            info!("Timewarp indexing ready for analysis");
            self.timewarp_selecting.tell(Protocol::Ready, None);
        }else {
        
            ctx.schedule_once(
                std::time::Duration::from_secs(5),
                 ctx.myself(), 
                 None, 
                 Protocol::Timer);
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

    fn receive_timewalkresult(&mut self,
        _ctx: &Context<Protocol>,
        id: String,
        timewarps: Vec<Timewarp>,
        _sender: Sender) {
        info!("Got timewalk result");
        //We need to reverse the insertion order for the found timewarp. The walker walks from present to past but we want to insert from
        //past to present to keep statistics.
        for v in timewarps.iter().rev() {
            let tw_data = self.storage.tw_detection_add_decision_data(v.clone());
            let _res = self.storage.tw_detection_add_to_index(get_time_key(&v.source_timestamp), 
                vec![
                    (v.source_hash.to_string(), v.target_hash().to_string()), 
                    (format!("{}{}", TIMEWARP_ID_PREFIX, tw_data.timewarpid), tw_data.hash.to_string())
                ]);
        }
        //Here we don't need to reverse because they get in the correct order.
        for v in self.known_active_walks.get(&id).expect("The vec to exists even when empty") {
            info!("Inserting lagging timewarp.");
            let tw_data = self.storage.tw_detection_add_decision_data(v.clone());
            let _res = self.storage.tw_detection_add_to_index(get_time_key(&v.source_timestamp), 
                vec![
                    (v.source_hash.to_string(), v.target_hash().to_string()), 
                    (format!("{}{}",TIMEWARP_ID_PREFIX, tw_data.timewarpid), tw_data.hash.to_string())
                ]);
        }

        let _a = self.known_active_walks.remove(&id);
        if _a.is_some() {
            info!("Removing: {}", &id);
        }
       
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
                    if &unwrapped.last_picked_timewarp.hash == tw.target_hash() {
                        info!("Found connecting timewarp");
                    }
                }
                let known_tip = self.known_timewarp_tips.get(tw.target_hash());
                if known_tip.is_some() {
                    let unwrapped = known_tip.unwrap();
                    //check if we have an active walker ID.
                    //Not having this is default and will result in immediate storage of the ID
                    if unwrapped == "" || !self.known_active_walks.contains_key(unwrapped) {
                    
                        info!("Found a known timewarp Old: {} and new {}", tw.target_hash().to_string(), tw.source_hash.to_string());
                        self.known_timewarp_tips.remove(tw.target_hash());
                        self.known_timewarp_tips.insert(tw.source_hash.to_string(), String::new());

                        let tw_data = self.storage.tw_detection_add_decision_data(tw.clone());
                        let _res = self.storage.tw_detection_add_to_index(get_time_key(&tw.source_timestamp), 
                            vec![
                                (tw.source_hash.to_string(), tw.target_hash().to_string()), 
                                (format!("{}{}", TIMEWARP_ID_PREFIX, tw_data.timewarpid), tw_data.hash.to_string())
                            ]);

                    }else{
                        info!("Caching lagging timewarp. {}", tw.source_hash.to_string());

                        let vec_to_add = self.known_active_walks.get_mut(unwrapped).expect("The key to be there");
                        //Todo how do I fix borrowing issues here?
                        self.known_timewarp_tips.insert(tw.source_hash.to_string(), unwrapped.to_string());
                        self.known_timewarp_tips.remove(tw.target_hash());
                        vec_to_add.push(tw.clone());
                    }
                   
                }else{

                    info!("Found a new timewarp!!!!!, start following ToB {} - {} - {}", tw.trunk_or_branch , tw.source_hash.to_string(), tw.target_hash().to_string());
                    let my_actor3 = _ctx.actor_of(TimewarpWalker::props(self.storage.clone()), &format!("timewarp-walking-{}", tw.source_hash.to_string())).unwrap();

                    let tw_start = StartTimewarpWalking { 
                        source_hash: cpy.hash, 
                        source_branch: cpy.branch_transaction,
                        source_trunk: cpy.trunk_transaction,
                        source_tag: cpy.tag,
                        distance: tw.distance,
                        source_timestamp: cpy.attachment_timestamp, 
                        trunk_or_branch: tw.trunk_or_branch,
                        last_picked_tw_tx: {
                           if self.last_picked_tw.is_some() {
                                let r = &self.last_picked_tw.as_ref().unwrap().last_picked_timewarp.hash;
                                r.to_owned()
                           }else{
                                String::from("")
                           }
                        }};
                    //Insert in caching of active timewarps. So they get added 
                    self.known_timewarp_tips.insert(tw.source_hash.to_string(), tw_start.id());
                    self.known_active_walks.insert(tw_start.id(), Vec::new());

                    my_actor3.tell(Protocol::StartTimewarpWalking(tw_start)
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

        info!("Registering {:?}", res);
    }

    pub fn props(args:(Arc<dyn Persistence>, ActorRef<Protocol>)) -> BoxActorProd<TimewarpIndexing> {
        Props::new_args(TimewarpIndexing::actor, args)
    }
}

