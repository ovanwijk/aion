

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
use crate::indexstorage::*;
//use crate::timewarping::zmqlistener::*;
use crate::aionmodel::transaction::get_trunk_branch_ts_tag;
use crate::pathway::PathwayDescriptor;
use crate::timewarping::Protocol;
use crate::timewarping::WebRequestType;
use crate::aionmodel::lifeline_subgraph::GraphEntryEvent;
use crate::txstorage::*;
// use crate::timewarping::Timewarp;
// use crate::timewarping::signing;
// use crate::timewarping::timewarpwalker::*;
//use std::collections::HashMap;


extern crate async_std;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
#[derive(Debug)]
pub struct TransactionPulling {   
    
    start_time: i64,
    storage: Arc<dyn Persistence>,
    tx_storage: Arc<dyn TXPersistence>,
    batching_size: usize,
    working: bool,
    
}



impl Actor for TransactionPulling {
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
             
                self.receive_timer(ctx, sender);
            }
            //Protocol::TransactionConfirmed(__msg) => self.receive_transactionconfimation(ctx, __msg, sender),
            _ => ()
        }
    }   
    
}


impl TransactionPulling {
    async fn do_work(&mut self) -> bool {
        let finished = true;
        if !self.working {
            self.working = true;
            let job = self.storage.next_pull_job(&0);
            if job.is_some() {
            
                let mut unwrapped_job:PullJob = job.unwrap();
                let mut pathway_iter = crate::pathway::PathwayIterator {
                    descriptor: unwrapped_job.pathway.clone(),
                    index: unwrapped_job.current_index
                };
                //let is_local_node = unwrapped_job.node == SETTINGS.node_settings.iri_connection();
                unwrapped_job.status = PIN_STATUS_IN_PROGRESS.to_string();
                unwrapped_job.last_update = crate::now();
                let mut end_reached = false;
                self.storage.update_pull_job(&unwrapped_job);
                while !end_reached {
                    let mut batch_vec:Vec<(String, String)> = vec!();
                    //let mut batch_tx_vec:Vec<String> = vec!();
                    let mut ll_results:Vec<LifeLineData> = vec!();
                    let max_steps = unwrapped_job.max_steps();
                    //We explicitly do + one because we can walk to the transaction but we still need to pull it.
                    for _i in 0..(std::cmp::min(self.batching_size, max_steps) + 1) {
                        
                        let t1 = iota_api::get_trytes_async(unwrapped_job.node.clone(), vec!(unwrapped_job.current_tx.clone())).await;
                        if t1.is_err() {
                            warn!("Error occurred during pulling {}", t1.unwrap_err());
                            //TODO add error counter
                            unwrapped_job.status = PIN_STATUS_NODE_ERROR.to_string();
                            unwrapped_job.last_update = crate::now();
                            self.storage.update_pull_job(&unwrapped_job);
                            self.working = false;
                            return true;
                        }
                        let u_trytes = t1.unwrap().trytes[0].clone();
                        let t_b_ts_tag = get_trunk_branch_ts_tag(&u_trytes);
                        batch_vec.push((unwrapped_job.current_tx.clone(), u_trytes));
                        
                        
                        let (current_index, current_tx) = (unwrapped_job.current_index.clone(), unwrapped_job.current_tx.clone());
                        match pathway_iter.next() {
                            Some(step) =>  match step {
                                crate::pathway::_Y => {
                                    unwrapped_job.current_tx = t_b_ts_tag.0.clone();
                                    unwrapped_job.history.push(t_b_ts_tag.1);
                                    unwrapped_job.current_index += 1;                                    
                                },
                                crate::pathway::_T => {
                                    unwrapped_job.current_tx = t_b_ts_tag.0.clone();                            
                                    unwrapped_job.current_index += 1;
                                },
                                crate::pathway::_B => {
                                    unwrapped_job.current_tx = t_b_ts_tag.1.clone();                        
                                    unwrapped_job.current_index += 1;
                                },
                                crate::pathway::_E => {
                                    let split = unwrapped_job.history.split_last();
                                    unwrapped_job.current_index += 1;
                                    if split.is_none() {
                                        //means last transaction was pulled.
                                        break;
                                    };
                                    let u_split = split.unwrap();
                                    if unwrapped_job.lifeline_component.is_some() {
                                        error!("Do not use split pathways for lifelines.");
                                    }
                                    unwrapped_job.current_tx = u_split.0.to_string(); 
                                    
                                },
                                _ => {}
                            },
                            None => {end_reached=true;}
                        };
                        
                         //Handle lifeline case
                         if unwrapped_job.lifeline_component.is_some() {
                            let mut ll_comp = unwrapped_job.lifeline_component.unwrap();
                            if current_index == 0 {
                                if ll_comp.lifeline_transitions.contains_key(&(current_index as i64)) {
                                    ll_comp.lifeline_prev_index = Some(current_index.clone() as i64);
                                }
                                ll_comp.lifeline_prev = Some((current_tx.clone(),t_b_ts_tag.2, t_b_ts_tag.3.clone(), unwrapped_job.pathway.tx_count as i64));
                            }else {
                           
                                    let prev = ll_comp.lifeline_prev.clone().expect("Message to be there");
                                    if ll_comp.lifeline_prev_index.is_some() {
                                        let start_index = ll_comp.lifeline_prev_index.unwrap();
                                        let end_index = ll_comp.lifeline_transitions.get(&start_index).unwrap();
                                        if end_index.clone() as usize == current_index - start_index as usize {
                                            ll_results.push(LifeLineData{                                                            
                                                timewarp_tx: prev.0.clone(),                                                            
                                                timestamp: prev.1.clone(),
                                                unpinned_connecting_txs: vec!(), 
                                                    paths: vec!(LifeLinePathData {
                                                    transactions_till_oldest: prev.3.clone(),// TODO fix somehow
                                                    oldest_tx: ll_comp.lifeline_end_tx.clone(),                                                    
                                                    oldest_timestamp: ll_comp.lifeline_end_ts.clone(),                                                
                                                    connecting_pathway: unwrapped_job.pathway.slice(start_index as usize, (start_index + *end_index) as usize),
                                                    connecting_timestamp: t_b_ts_tag.2.clone(),
                                                    connecting_timewarp: current_tx.clone()
                                                })
                                            });
                                            match ll_comp.lifeline_transitions.get(&(current_index as i64)) {
                                                Some(end) => {
                                                    ll_comp.lifeline_prev_index = Some(current_index.clone() as i64);
                                                    ll_comp.lifeline_prev = Some((current_tx.clone() ,t_b_ts_tag.2, t_b_ts_tag.3, prev.3 - 1 ));
                                                },
                                                None => {
                                                    ll_comp.lifeline_prev_index = None;
                                                    ll_comp.lifeline_prev = Some((current_tx.clone(),t_b_ts_tag.2, t_b_ts_tag.3, prev.3 - end_index ));
                                                }
                                            }
                                            
                                            
                                        } else {
                                            //Ignore this step because it is withing the start and end indexes
                                        }
                                    } else {
                                            
                                                                                  
                                        ll_results.push(LifeLineData{
                                            timewarp_tx: prev.0.clone(),                                                      
                                            timestamp: prev.1.clone(),
                                            unpinned_connecting_txs: vec!(), 
                                            paths: vec!(LifeLinePathData {
                                                transactions_till_oldest: prev.3,// TODO fix somehow
                                                oldest_tx: ll_comp.lifeline_end_tx.clone(),                                                        
                                                oldest_timestamp: ll_comp.lifeline_end_ts.clone(),                                                            
                                                connecting_pathway: if unwrapped_job.pathway.get_at_index(current_index - 1).unwrap() == crate::pathway::_T {PathwayDescriptor::trunk()} else {PathwayDescriptor::branch()},
                                                connecting_timestamp: t_b_ts_tag.2.clone(),
                                                connecting_timewarp: current_tx.clone()
                                            })
                                        });
                                        match ll_comp.lifeline_transitions.get(&(current_index as i64)) {
                                            Some(end) => {
                                                        //let p = ll_comp.lifeline_prev.unwrap();
                                                        ll_comp.lifeline_prev_index = Some(current_index.clone() as i64);
                                                        ll_comp.lifeline_prev = Some((current_tx.clone() ,t_b_ts_tag.2, t_b_ts_tag.3, prev.3 - 1 ));
                                                        //ll_comp.lifeline_prev = Some((p.0.clone(),t_b_ts_tag.2, t_b_ts_tag.3.clone(), p.3 - end ));
                                                    },
                                            None => {      
                                                ll_comp.lifeline_prev_index = None;
                                                ll_comp.lifeline_prev = Some((current_tx.clone(),t_b_ts_tag.2, t_b_ts_tag.3, prev.3 - 1));
                                            }                                        
                                        }
                                }
                            }                                       
                                
                            unwrapped_job.lifeline_component = Some(ll_comp);
                        }
                    }

                    
                    let pinned_trytes = self.tx_storage.store_txs(batch_vec); //iota_api::pin_transaction_trytes_async(unwrapped_job.node.clone(), batch_vec).await;
                    if !pinned_trytes.is_err() {
                        //TODO update ll_comp
                        info!("Prepending: {}/{} : {}",unwrapped_job.current_index, ll_results.len(), unwrapped_job.id );
                        for ll_data in  ll_results {
                            match self.storage.prepend_to_lifeline(ll_data.clone()) {
                                Err(e) => {
                                    error!("Prepend lifeline {}", e.to_string())
                                },
                                _ => {info!("Prepend success {} ", ll_data.timewarp_tx)}
                            };
                        }
                        self.storage.update_pull_job(&unwrapped_job);
                    }else{
                        unwrapped_job.status = PIN_STATUS_PIN_ERROR.to_string();
                        unwrapped_job.last_update = crate::now();
                        self.storage.update_pull_job(&unwrapped_job);
                        self.working = false;
                        
                        break;
                    }
                }
                //TODO add event to subgraph
                if unwrapped_job.lifeline_component.is_some() {
                    let ll_comp_un = unwrapped_job.lifeline_component.unwrap();
                    let tx_cutoff = match (&ll_comp_un.between_start, &ll_comp_un.between_end) {
                        (Some(s), Some(e)) => {
                            Some(self.storage.get_lifeline_tx(s).expect("LL tx to exist").walk_towards(e).unwrap().transactions_till_oldest as i64 - unwrapped_job.pathway.tx_count as i64 )
                        },
                        _ => None
                    };
                    
                    let _r = self.storage.process_event(GraphEntryEvent {
                        between_end: ll_comp_un.between_end,
                        between_start: ll_comp_un.between_start,
                        between_tx_split: tx_cutoff,
                        index: self.storage.new_index(),
                        target_tx_id: ll_comp_un.lifeline_end_tx.clone(),
                        txid: ll_comp_un.lifeline_start_tx.clone(),
                        timestamp: ll_comp_un.lifeline_start_ts.clone(),
                        target_timestamp: ll_comp_un.lifeline_end_ts.clone(),
                        tx_distance_count: unwrapped_job.pathway.tx_count as i64, // TODO
    
                    });
                }                
               

                self.storage.pop_pull_job(unwrapped_job.id);
                self.working = false;
                return false;
            }else{
                self.working = false;
                return true;
            }

        }
        finished

    }
}


impl TransactionPulling {
    fn actor(storage:(Arc<dyn Persistence>, Arc<dyn TXPersistence>)) -> Self {
        
        TransactionPulling {
          
            batching_size: 25,
            working: false,
            storage: storage.0.clone(),            
            tx_storage: storage.1.clone(),    
            start_time: crate::now()
        }
    }
    pub fn props(storage:(Arc<dyn Persistence>, Arc<dyn TXPersistence>)) -> BoxActorProd<TransactionPulling> {
        Props::new_args(TransactionPulling::actor, storage)
    }
    pub fn receive_webrequest(&mut self,
        ctx: &Context<Protocol>,
        msg: WebRequestType,
        sender: Sender) {
      
    }

    pub fn receive_timer(&mut self,
        ctx: &Context<Protocol>,
        _sender: Sender) {
       
        let finished = async_std::task::block_on(self.do_work());     
        if finished {
            ctx.schedule_once(
                std::time::Duration::from_secs(2),
                 ctx.myself(), 
                 None, 
                 Protocol::Timer);
        }else {
            ctx.myself().tell(Protocol::Timer, None);
        }
       
    }

}
