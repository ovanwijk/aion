

//use std::str;
// use std::{
//     collections::{HashMap, VecDeque, hash_map::DefaultHasher},
//     hash::{Hash, Hasher, BuildHasherDefault, BuildHasher},
// };
//use std::collections::LinkedList;
use crate::SETTINGS;
use riker::actors::*;
use riker::actors::Context;
use crate::iota_api;
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
pub struct TransactionPinning {   
    
    start_time: i64,
    storage: Arc<dyn Persistence>,
    node: String,
    ready: bool
}



impl Actor for TransactionPinning {
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


impl TransactionPinning {
   fn pin_lifelife(&self) {
       let to_pin_list = self.storage.get_unpinned_lifeline();
       
       if to_pin_list.len() > 0 {
           let (to_pin, rest) = to_pin_list.split_first().unwrap();
           let mut lifelinetx = self.storage.get_lifeline_tx(&to_pin).expect("The lifeline transaction to be valid");
           let mut to_pin: Vec<String> = vec!(lifelinetx.timewarp_tx.clone());
           to_pin.append(&mut lifelinetx.unpinned_connecting_txs.clone());
           let web_result = iota_api::pin_transaction_hashes(self.node.clone(), to_pin.clone());
           if web_result.is_err() {
               warn!("Error occured");
           }else{
               info!("Pinned {} transactions", to_pin.len());
                lifelinetx.unpinned_connecting_txs = vec!();
                self.storage.update_lifeline_tx(lifelinetx);
                //TODO fix race condition
                self.storage.set_unpinned_lifeline(rest.to_vec());
           }
         
           

       }
   }
}


impl TransactionPinning {
    fn actor(storage:Arc<dyn Persistence>) -> Self {
        let node = &SETTINGS.node_settings.iri_connection(); 
        
        TransactionPinning {
            ready: false,
           
            storage: storage.clone(),            
            node: node.to_string(),           
            start_time: crate::now()
        }
    }
    pub fn props(storage:Arc<dyn Persistence>) -> BoxActorProd<TransactionPinning> {
        Props::new_args(TransactionPinning::actor, storage)
    }
    pub fn receive_webrequest(&mut self,
        ctx: &Context<Protocol>,
        msg: WebRequestType,
        sender: Sender) {

           
      
    }

    pub fn receive_timer(&mut self,
        ctx: &Context<Protocol>,
        _sender: Sender) {
            self.pin_lifelife();
       
        ctx.schedule_once(
            std::time::Duration::from_secs(2),
             ctx.myself(), 
             None, 
             Protocol::Timer);
    }

}
