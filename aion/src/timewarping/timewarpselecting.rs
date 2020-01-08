

//use std::str;
// use std::{
//     collections::{HashMap, VecDeque, hash_map::DefaultHasher},
//     hash::{Hash, Hasher, BuildHasherDefault, BuildHasher},
// };
//use std::collections::LinkedList;
use crate::SETTINGS;
use riker::actors::*;
use riker::actors::Context;
use crate::iota_api::*;
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
#[macro_use]
use log; 
use std::sync::Arc;
use serde::{Serialize, Deserialize};
#[derive(Debug)]
pub struct TimewarpSelecting {
    picked_timewarp: Option<TimewarpData>,
    //available_timewarps: HashMap<String, TimewarpData>,
    storage: Arc<dyn Persistence>,
    min_distance_in_seconds: i64, 
    node: String
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimewarpSelectionState {
    last_picked_timewarp: TimewarpData,
    transitioned: bool,
    untransitioned_timewarp_heads: Vec<TimewarpData>
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
            Protocol::Timer => self.receive_timer(ctx, sender),
            Protocol::Ping => {
                info!("Ping");
                let _l = sender.unwrap().try_tell(Protocol::Pong, None);
            },
            Protocol::Pong => {
                info!("Pong");
            }
            //Protocol::TransactionConfirmed(__msg) => self.receive_transactionconfimation(ctx, __msg, sender),
            _ => ()
        }
    }   
    
}


impl TimewarpSelecting {
    fn pick_timewarp(&mut self){
        let timewarps:std::vec::Vec<TimewarpData> = crate::indexstorage::get_lastest_known_timewarps(self.storage.clone())
            .into_iter().filter(|t| t.timestamp > crate::now() - self.min_distance_in_seconds).collect();
        if self.picked_timewarp.is_none() && timewarps.len() == 0 {
            return;
        }
        let best_tw = self.best_timewarp(&timewarps);
        if self.picked_timewarp.is_none() {
            self.picked_timewarp = Some(best_tw.clone());
        }
        if self.picked_timewarp.as_ref().unwrap().timewarpid != best_tw.timewarpid {
            info!("Switched!");
            self.picked_timewarp = Some(best_tw.clone());
            //TODO add to last picked
            //let _a = self.storage.add_last_picked_tw(vec!(best_tw));
        }

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
            min_distance_in_seconds: 180
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

        self.pick_timewarp();


        ctx.schedule_once(
            std::time::Duration::from_secs(15),
             ctx.myself(), 
             None, 
             Protocol::Timer);
    }

}
