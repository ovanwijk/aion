

//use std::str;
// use std::{
//     collections::{HashMap, VecDeque, hash_map::DefaultHasher},
//     hash::{Hash, Hasher, BuildHasherDefault, BuildHasher},
// };
//use std::collections::LinkedList;
use crate::SETTINGS;
use riker::actors::*;
use riker::actors::Context;

//use iota_lib_rs::iota_model::Transaction;
//use crate::aionmodel::tangle::*;
use crate::indexstorage::Persistence;
use crate::pathway::PathwayDescriptor;
//use crate::timewarping::zmqlistener::*;
use crate::timewarping::Protocol;
use crate::timewarping::WebRequestType;
// use crate::timewarping::Timewarp;
// use crate::timewarping::signing;
// use crate::timewarping::timewarpwalker::*;
//use std::collections::HashMap;
use crate::indexstorage::*;


use std::sync::Arc;
use serde::{Serialize, Deserialize};
#[derive(Debug)]
pub struct TimewarpSelecting {
    picked_timewarp: Option<TimewarpSelectionState>,
    //available_timewarps: HashMap<String, TimewarpData>,
    start_time: i64,
    storage: Arc<dyn Persistence>,
    min_distance_in_seconds: i64, 
    node: String,
    ready: bool
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimewarpSelectionState {    
    pub last_picked_timewarp: TimewarpData,    
    pub untransitioned_timewarp_heads: Vec<TimewarpData>
}


impl Actor for TimewarpSelecting {
    type Msg = Protocol;

    fn recv(&mut self,
                ctx: &Context<Self::Msg>,
                msg: Self::Msg,
                sender: Sender) {

        // Use the respective Receive<T> implementation
        match msg {
            Protocol::WebRequest(__msg) => self.receive_webrequest(ctx, __msg, sender),
            // Protocol::Start => self.receive_step(ctx,  sender),
            Protocol::Timer => {
                self.receive_timer(ctx, sender);
            },
            Protocol::Ping => {
                info!("Ping");
                let _l = sender.unwrap().try_tell(Protocol::Pong, None);
            },
            Protocol::Pong => {
                info!("Pong");
            },
            Protocol::Ready => {
                self.ready = true;
                self.storage.set_generic_cache(crate::indexstorage::ALL_STARTED, b"true".to_vec());
                self.receive_timer(ctx, sender);
            }
            //Protocol::TransactionConfirmed(__msg) => self.receive_transactionconfimation(ctx, __msg, sender),
            _ => ()
        }
    }   
    
}


impl TimewarpSelecting {
    fn pick_timewarp(&mut self, ctx:&Context<Protocol>){
        //We filter using the min_distance_in_seconds of timewarp time so that we are not switching from one to the
        //if two are started around the same time.
        let timewarps:std::vec::Vec<TimewarpData> = crate::indexstorage::get_lastest_known_timewarps(self.storage.clone())
            .into_iter().filter(|t| t.timestamp > crate::now() - (self.min_distance_in_seconds * 2)).collect();
        if timewarps.len() == 0 {
            return;
        }
        let best_tw = self.best_timewarp(&timewarps);
        if self.picked_timewarp.is_none() {     
            if best_tw.timewarpid.is_empty() {
                info!("No timewarps yet");
                return;
            };
            self.picked_timewarp = Some(TimewarpSelectionState {
                last_picked_timewarp: best_tw.clone(),
                untransitioned_timewarp_heads: vec!()
            });
            info!("First pick: {}", best_tw.timewarpid);
            
            self.store_newly_pick(best_tw.clone());
            self.storage.set_last_picked_tw(self.picked_timewarp.as_ref().unwrap().clone());

            return
        }
        if self.picked_timewarp.as_ref().unwrap().last_picked_timewarp.timewarpid != best_tw.timewarpid {
           
            let fut = crate::iota_api::find_paths(SETTINGS.node_settings.iri_connection(), best_tw.hash.clone(), 
            vec!(self.picked_timewarp.as_ref().unwrap().last_picked_timewarp.hash.to_string()));
            
            //let foundPath = futures::executor::block_on(ctx.run(fut).unwrap());
            if !fut.is_err() {
                info!("Switched! From: {} to {}", self.picked_timewarp.as_ref().unwrap().last_picked_timewarp.hash, best_tw.hash);
                self.picked_timewarp = Some(TimewarpSelectionState {
                    last_picked_timewarp: best_tw.clone(),
                    untransitioned_timewarp_heads: vec!(self.picked_timewarp.as_ref().unwrap().last_picked_timewarp.clone())
                });
                //info!("Path found: {}", serde_json::to_string_pretty(&fut.unwrap()).unwrap());
                self.store_newly_pick(best_tw.clone());
                self.storage.set_last_picked_tw(self.picked_timewarp.as_ref().unwrap().clone());
            }else{
                warn!("Switch required but not yet a path found. {}:{} , {}:{}",
                 self.picked_timewarp.as_ref().unwrap().last_picked_timewarp.timewarpid,
                 self.picked_timewarp.as_ref().unwrap().last_picked_timewarp.score(),
                 best_tw.timewarpid, best_tw.score());
            }
            //TODO add to last picked
            //let _a = self.storage.add_last_picked_tw(vec!(best_tw));
        }else{
            if self.picked_timewarp.as_ref().unwrap().last_picked_timewarp.hash != best_tw.hash {
                info!("No switch add");
                self.store_newly_pick(best_tw.clone());
                self.picked_timewarp = Some(TimewarpSelectionState {
                    last_picked_timewarp: best_tw.clone(),
                    untransitioned_timewarp_heads: vec!()
                });
                self.storage.set_last_picked_tw(self.picked_timewarp.as_ref().unwrap().clone());
            }
            
        }

    }
    fn store_newly_pick(&mut self, data: TimewarpData) -> Result<(), String> {
        let last_ll = self.storage.get_last_lifeline();
        if last_ll.is_some() {
            // We will use this field to internally advance our checks.
            let mut ll_unwrapped = &last_ll.unwrap();
            let next_step = self.storage.tw_detection_get_decision_data(data.target_hash());
            let mut timewarps:Vec<TimewarpData> = vec!(data.clone());
            //First follow the timewarp until the closest point to the lifeline.
            if next_step.is_some() {
                let mut unwrapped = next_step.unwrap();
                while unwrapped.hash != ll_unwrapped.timewarp_tx && unwrapped.timestamp > ll_unwrapped.timestamp {
                    //let unwrapped = next_step.as_ref().unwrap();
                    timewarps.push(unwrapped.clone());
                    let _t = self.storage.tw_detection_get_decision_data(unwrapped.target_hash());
                    if _t.is_some() {
                        unwrapped = _t.unwrap();
                    }else{
                        break;
                    }
                }
            }
            // We reverse because pulling the timewarp is from present to past. 
            // and we need to append from past to present
            let mut lifelines:Vec<LifeLineData> = vec!();
            for connecting_timewarp in timewarps.iter().rev() {
                //let mut skip = false;
                let connecting_txs = if connecting_timewarp.target_hash() != ll_unwrapped.timewarp_tx.as_ref() {
                    let path_found = crate::iota_api::find_paths(SETTINGS.node_settings.iri_connection(), 
                        connecting_timewarp.hash.to_string(), vec!(ll_unwrapped.timewarp_tx.clone()));
                        if path_found.is_err() {
                            //IGNORE this timewarp end, we cannot find a path to the location
                            warn!("Found error {:?}", path_found.unwrap_err());
                            info!("Breaking and retrying");
                            //skip = true;
                            (vec!(), None)

                        }else {
                            info!("Connecting path found");                        
                            let u_path = path_found.unwrap();
                            (*u_path.txIDs.clone(), Some(u_path.to_pathway(connecting_timewarp.hash.to_string())))
                           
                        }
                }else {
                    (vec!(), Some(if data.trunk_or_branch {PathwayDescriptor::trunk()}else{PathwayDescriptor::branch()}))
                };
                //if skipped it mean we consider this particular timewarp transaction to reach to far.
                //Thus skipping it to add it to our lifeline
                if connecting_txs.1.is_some() { 

                    lifelines.push(LifeLineData {                       
                        timewarp_tx: connecting_timewarp.hash.clone(),                       
                        timestamp: connecting_timewarp.timestamp,
                        unpinned_connecting_txs: connecting_txs.0.clone(),
                        paths: vec!(LifeLinePathData {
                            oldest_timestamp: if ll_unwrapped.paths.is_empty() {ll_unwrapped.timestamp.clone()}else{ll_unwrapped.paths[0].oldest_timestamp.clone()},
                        
                            //TODO count transactions and append
                            transactions_till_oldest : 0,
                            oldest_tx: if ll_unwrapped.paths.is_empty() {ll_unwrapped.timewarp_tx.clone()}else{ll_unwrapped.paths[0].oldest_tx.clone()},
                            connecting_pathway: connecting_txs.1.clone().unwrap(),
                            connecting_timestamp: ll_unwrapped.timestamp,
                            connecting_timewarp: ll_unwrapped.timewarp_tx.clone()
                        })
                    });
                    ll_unwrapped = lifelines.last().unwrap(); //TODO Fix immutable borrow
                }

            }
            //lifelines.reverse();
            if lifelines.len() > 0 {
                let _r = self.storage.add_to_lifeline(lifelines);
                if _r.is_err() { warn!("Error occured {:?}", _r.unwrap_err());}
            }
          
            

        }else{
            //Unique case, only the first time.
            info!("Initializing lifeline");
            let _r = self.storage.add_to_lifeline(vec!(LifeLineData {                
                timewarp_tx: data.hash.clone(),
                timestamp: data.timestamp,
                unpinned_connecting_txs: vec!(),
                paths: vec!()
            }));
        };
        Ok(())
    }

    fn best_timewarp(&self, warps: &Vec<TimewarpData>) -> TimewarpData {
        let leader = crate::iota_api::get_preffered_timewarp();
        let mut selected =  &TimewarpData{
            timewarpid: String::new(),
            hash: String::new(),
            trunk: String::new(),
            branch: String::new(),
            distance: 0,
            trunk_or_branch: false,
            timestamp: 0,
            timestamp_deviation_factor:0.0,
            avg_distance: 0,
            index_since_id: 0,
        };
        //warps.first().expect("At least one element");
        let mut leader_selected = false;
        for x in warps {
            let score_sub = if self.picked_timewarp.is_some() && self.picked_timewarp.as_ref().unwrap().last_picked_timewarp.timewarpid == x.timewarpid { 0}else {
                SETTINGS.timewarp_index_settings.detection_threshold_switch_timewarp_in_seconds as isize
            };
            let selected_sub = if self.picked_timewarp.is_some() && self.picked_timewarp.as_ref().unwrap().last_picked_timewarp.timewarpid == selected.timewarpid { 0}else {
                SETTINGS.timewarp_index_settings.detection_threshold_switch_timewarp_in_seconds as isize
            };
            if !leader_selected && x.score() - score_sub  
                > selected.score() -  selected_sub {
                selected = x;
            }
            match &leader {
                Some(v) => {
                    if &x.timewarpid == v {
                        if self.picked_timewarp.is_some() && &self.picked_timewarp.as_ref().unwrap().last_picked_timewarp.timewarpid != v {
                            info!("Leader selected for tw {}", v.clone());
                        }
                        
                        selected = x;
                        leader_selected = true;
                    }
                },
                None => {}
            }
        }
        selected.clone()
    }
}


impl TimewarpSelecting {
    fn actor(storage:Arc<dyn Persistence>) -> Self {
        let node = &SETTINGS.node_settings.iri_connection(); 
        let last_picked =  &storage.get_last_picked_tw();
        let _r = storage.set_generic_cache(crate::indexstorage::ALL_STARTED, "false".as_bytes().to_vec());
        if _r.is_err() {
            info!("{}", _r.unwrap_err());
        }
        TimewarpSelecting {
            picked_timewarp: if last_picked.is_some() {
                Some(last_picked.as_ref().unwrap().clone())
            }else { None },
            storage: storage.clone(),            
            node: node.to_string(),
            min_distance_in_seconds: crate::SETTINGS.timewarp_index_settings.detection_threshold_switch_timewarp_in_seconds,
            start_time: crate::now(),
            ready: false
        }
        
       
    }
    pub fn props(storage:Arc<dyn Persistence>) -> BoxActorProd<TimewarpSelecting> {
        Props::new_args(TimewarpSelecting::actor, storage)
    }
    pub fn receive_webrequest(&mut self,
        ctx: &Context<Protocol>,
        msg: WebRequestType,
        sender: Sender) {

            if msg == WebRequestType::PickedTimewarp {
                if self.picked_timewarp.is_some() {
                    let _r = sender.unwrap().try_tell(Protocol::WebReply(serde_json::to_string_pretty(&self.picked_timewarp.as_ref().unwrap()).unwrap()), None);
                } else {
                    let _r = sender.unwrap().try_tell(Protocol::WebReply("{data:null}".to_string()), None);
                }
                
            }
      
    }

    pub fn receive_timer(&mut self,
        ctx: &Context<Protocol>,
        _sender: Sender) {

        self.pick_timewarp(ctx);


        ctx.schedule_once(
            std::time::Duration::from_secs(15),
             ctx.myself(), 
             None, 
             Protocol::Timer);
    }

}
