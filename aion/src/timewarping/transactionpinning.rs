

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
use crate::txstorage::*;


use std::sync::Arc;
use serde::{Serialize, Deserialize};
#[derive(Debug)]
pub struct TransactionPinning {   
    
    start_time: i64,
    storage: Arc<dyn Persistence>,
    tx_storage: Arc<dyn TXPersistence>,
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
   async fn pin_lifelife(&self) -> bool {
       let to_pin_list = self.storage.get_unpinned_lifeline();
       
       if to_pin_list.len() > 0 {
           let (to_pin, _rest_lifelines) = to_pin_list.split_first().unwrap();
           let mut lifelinetx = self.storage.get_lifeline_tx(&to_pin).expect("The lifeline transaction to be valid");
           //let mut to_pin: Vec<String> = vec!(lifelinetx.timewarp_tx.clone());
           
           let (mut t25, mut rest) = (vec!(lifelinetx.timewarp_tx.clone()), lifelinetx.unpinned_connecting_txs.clone());
           while t25.len() > 0 && rest.len() > 0 {
             
             let t1 = iota_api::get_trytes_async(self.node.clone(), t25.to_vec()).await;
             if t1.is_err() {
                warn!("Error occured {}", t1.unwrap_err().to_string());
                return false;
            }else{
                let mut to_store:Vec<(String, String)> = vec!();
                let unwrapped = t1.unwrap();
                for i in 0..t25.len() {
                    to_store.push((t25[i].clone(), unwrapped.trytes[i].clone()));
                }
                match self.tx_storage.store_txs(to_store) {
                    Ok(_) => { info!("Pinned {} transactions", t25.len());}
                    Err(s) => {error!("Something went wrong here: {}", s);}
                }

            }
            let (a, b) = rest.split_at(std::cmp::min(25, rest.len()));
            t25 = a.to_vec();
            
            rest = b.to_vec();
            info!("Rest:{}, len:{}", rest.len(), t25.len());
           };
     
                lifelinetx.unpinned_connecting_txs = vec!();
                self.storage.update_lifeline_tx(lifelinetx);
     
                self.storage.set_unpinned_lifeline(rest.to_vec());
                if ( _rest_lifelines.len() > 0) {
                    return false;
                }
                return true;
     
       }
       return true;
   }
}


impl TransactionPinning {
    fn actor(storage:(Arc<dyn Persistence>, Arc<dyn TXPersistence>)) -> Self {
        let node = &SETTINGS.node_settings.iri_connection(); 
        
        TransactionPinning {
            ready: false,
           
            storage: storage.0.clone(),
            tx_storage: storage.1.clone(),
            node: node.to_string(),           
            start_time: crate::now()
        }
    }
    pub fn props(storage:(Arc<dyn Persistence>, Arc<dyn TXPersistence>)) -> BoxActorProd<TransactionPinning> {
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

            let finished = async_std::task::block_on(self.pin_lifelife());     
            if finished {
                ctx.schedule_once(
                    std::time::Duration::from_secs(2),
                     ctx.myself(), 
                     None, 
                     Protocol::Timer);
            }else {
                ctx.myself().tell(Protocol::Timer, None);
            }

        // if self.pin_lifelife() {
        //     ctx.myself().tell(Protocol::Timer,None);
        // }else{

        // ctx.schedule_once(
        //     std::time::Duration::from_secs(2),
        //      ctx.myself(), 
        //      None, 
        //      Protocol::Timer);
        // }
       
    }

}
