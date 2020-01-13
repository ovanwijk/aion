

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
                self.receive_timer(ctx, sender);
            }
            //Protocol::TransactionConfirmed(__msg) => self.receive_transactionconfimation(ctx, __msg, sender),
            _ => ()
        }
    }   
    
}


impl TimewarpSelecting {
    fn pick_timewarp(&mut self, ctx:&Context<Protocol>){
        let timewarps:std::vec::Vec<TimewarpData> = crate::indexstorage::get_lastest_known_timewarps(self.storage.clone())
            .into_iter().filter(|t| t.timestamp > crate::now() - self.min_distance_in_seconds).collect();
        if self.picked_timewarp.is_none() && timewarps.len() == 0 {
            return;
        }
        let best_tw = self.best_timewarp(&timewarps);
        if self.picked_timewarp.is_none() {            
            self.picked_timewarp = Some(TimewarpSelectionState {
                last_picked_timewarp: best_tw.clone(),
                untransitioned_timewarp_heads: vec!()
            });
            info!("First pick: {}", best_tw.timewarpid);
        }
        if self.picked_timewarp.as_ref().unwrap().last_picked_timewarp.timewarpid != best_tw.timewarpid {
            info!("Switched! From: {} to {}", self.picked_timewarp.as_ref().unwrap().last_picked_timewarp.hash, best_tw.hash);
            self.picked_timewarp = Some(TimewarpSelectionState {
                last_picked_timewarp: best_tw.clone(),
                untransitioned_timewarp_heads: vec!(self.picked_timewarp.as_ref().unwrap().last_picked_timewarp.clone())
            });
            let fut = crate::iota_api::find_paths(SETTINGS.node_settings.iri_connection(), best_tw.hash, 
            vec!(self.picked_timewarp.as_ref().unwrap().untransitioned_timewarp_heads.first().unwrap().hash.to_string()));
            
            //let foundPath = futures::executor::block_on(ctx.run(fut).unwrap());
            if !fut.is_err() {
                info!("Path found: {}", serde_json::to_string_pretty(&fut.unwrap()).unwrap());
            }
            //TODO add to last picked
            //let _a = self.storage.add_last_picked_tw(vec!(best_tw));
        }

    }
    fn store_newly_pick(&mut self, data: TimewarpData) -> Result<(), String> {
        let last_ll = self.storage.get_last_lifeline();
        if last_ll.is_some() {
            // We will use this field to internally advance our checks.
            let mut ll_unwrapped = &last_ll.unwrap();
            let next_step = self.storage.tw_detection_get_decision_data(data.target_hash());
            let mut timewarps:Vec<TimewarpData> = vec!(data);
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
            // and we need to pull
            let mut lifelines:Vec<LifeLineData> = vec!();
            for connecting_timewarp in timewarps.iter().rev() {
                
                let connecting_txs = if connecting_timewarp.target_hash() != ll_unwrapped.timewarp_tx.as_ref() {
                    let path_found = crate::iota_api::find_paths(SETTINGS.node_settings.iri_connection(), 
                        connecting_timewarp.hash.to_string(), vec!(ll_unwrapped.timewarp_tx.clone()));
                        if path_found.is_err() {
                            //IGNORE this timewarp end, we cannot find a path to the location
                            vec!()
                        }else {
                            info!("Connecting path found");
                            //TODO turn to actual Vec
                            let mut a = *path_found.unwrap().txIDs.clone();
                            a.reverse();
                            a
                           
                        }
                }else {
                    vec!()
                };

                lifelines.push(LifeLineData {
                    timewarp_tx: connecting_timewarp.hash.clone(),    
                    trunk_or_branch: connecting_timewarp.trunk_or_branch.clone(),
                    timestamp: connecting_timewarp.timestamp,
                    oldest_timestamp: ll_unwrapped.timestamp,
                    connecting_txs: connecting_txs,
                    connecting_timestamp: None,
                    connecting_timewarp: None
                });
            }
            

        }else{
            //Unique case, only the first time.
            info!("Initializing lifeline");
            let _r = self.storage.add_to_lifeline(vec!(LifeLineData {
                timewarp_tx: data.hash,    
                trunk_or_branch: data.trunk_or_branch,
                timestamp: data.timestamp,
                oldest_timestamp: data.timestamp,
                connecting_txs: vec!(),
                connecting_timestamp: None,
                connecting_timewarp: None
            }));
        };
        Ok(())
    }

    fn best_timewarp(&self, warps: &Vec<TimewarpData>) -> TimewarpData {
        let mut selected = warps.first().expect("At least one element");
        for x in warps {
            if x.score() > selected.score() {
                selected = x;
            }
        }
        selected.clone()
    }
}


impl TimewarpSelecting {
    fn actor(storage:Arc<dyn Persistence>) -> Self {
        let node = &SETTINGS.node_settings.iri_connection(); 
        let last_picked =  &storage.get_last_picked_tw();
        TimewarpSelecting {
            picked_timewarp: last_picked.to_owned(),
            storage: storage.clone(),            
            node: node.to_string(),
            min_distance_in_seconds: 180,
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
