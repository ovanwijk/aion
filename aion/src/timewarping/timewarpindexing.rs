


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
use timewarping::zmqlistener::*;
use timewarping::Protocol;
use timewarping::Timewarp;




#[derive(Clone, Debug)]
pub struct RegisterZMQListener {
    pub zmq_listener: BasicActorRef
}


pub struct TimewarpIndexing {
    pub tangle:Tangle,
    avg_count: i64,
    avg_distance: f64,
    
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
        println!("AVG Timeleap {}", self.avg_distance);  
    }

    fn detect_timewarp(&mut self, tx:&Transaction) -> Option<Timewarp>{
        let branch_step = self.tangle.get(&tx.branch);
        
        if branch_step.is_some() {
            let diff = (tx.timestamp - branch_step.unwrap().timestamp) as usize;
            if diff > 0 && 
                diff > SETTINGS.timewarp_index_settings.detection_threshold_lower_bound_in_seconds &&
                diff < SETTINGS.timewarp_index_settings.detection_threshold_upper_bound_in_seconds { //Filterout direct references through bundes
                //Found branch timewarp
                 return Some(Timewarp{
                    from: tx.id.clone(),
                    to: branch_step.unwrap().id.clone(),
                    distance: diff as i64,
                    trunk_or_branch: false
                })    
            }
        }
        let trunk_step = self.tangle.get(&tx.trunk);
        if trunk_step.is_some() {
            let diff = (tx.timestamp - trunk_step.unwrap().timestamp) as usize;
            if diff > 0 && 
                diff > SETTINGS.timewarp_index_settings.detection_threshold_lower_bound_in_seconds &&
                diff < SETTINGS.timewarp_index_settings.detection_threshold_upper_bound_in_seconds { //Filterout direct references through bundes
                //Found trunk timewarp
                return Some(Timewarp{
                    from: tx.id.clone(),
                    to: trunk_step.unwrap().id.clone(),
                    distance: diff as i64,
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
    fn actor() -> Self {   
        //println!("{:?}{:?}", SETTINGS.node_settings.iri_host.to_string(), " Hello");
        //println!("{:?}", SETTINGS.timewarp_index_settings.detection_threshold_upper_bound_in_seconds);
        TimewarpIndexing {
            tangle: Tangle::default(),
            avg_count: 1,
            avg_distance: 1000.0 //default 1 second
        }
    }




     fn receive_newtransaction(&mut self,
                _ctx: &Context<Protocol>,
                _msg: NewTransaction,
                _sender: Sender) {
        let cpy = _msg.tx.clone();
        
        self.tangle.insert(_msg.tx);
        //self.cal_avarage_timeleap(&cpy);
        let timewarp = self.detect_timewarp(&cpy);
        if timewarp.is_some() {
            println!("Found a timewarp!!!!!");
        }
        self.tangle.maintain();
        //println!("Receiving transaction {}", _msg.tx.id);               
    }

    fn receive_registerzmqlistener(&mut self,
                ctx: &Context<Protocol>,
                msg: RegisterZMQListener,
                _sender: Sender) {        
     
        let res =  msg.zmq_listener.try_tell(Protocol::RegisterRoutee, ctx.myself());

        println!("Registering {:?}", res);
    }

    pub fn props() -> BoxActorProd<TimewarpIndexing> {
        Props::new(TimewarpIndexing::actor)
    }
}

