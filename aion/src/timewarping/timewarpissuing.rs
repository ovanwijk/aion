


use std::sync::Arc;

use riker::actors::*;
use riker::actors::Context;

use crate::timewarping::Protocol;
use crate::timewarping::zmqlistener::RegisterZMQListener;
use crate::indexstorage::*;
use crate::SETTINGS;
use crate::indexstorage::{Persistence};
//use iota_client::options::;
use iota_lib_rs::prelude::*;
use iota_utils::generate_new_seed;
use iota_lib_rs::iota_client::*;
//use iota_client::client::*;
use iota_client::options::*;
use iota_model::{Transaction,Transfer};
//use iota_conversion::trytes_converter;
use std::collections::{VecDeque, HashMap};
use crate::timewarping::signing::*;

//use crate::Result;



#[derive(Debug)]
pub struct TimewarpIssuer {
    state: TimewarpIssuingState,
    timeout_in_seconds: i64,
    max_queue_size: i64,
    latest_tx: String,
    milestone_interval_in_seconds: i64,
    promote_timeout_in_seconds: i64,
    storage:Arc<dyn Persistence>,
    node: String,
    active: bool,
    timewarp_selecting: ActorRef<Protocol>  
}

impl Actor for TimewarpIssuer {
    type Msg = Protocol;

    fn recv(&mut self,
                ctx: &Context<Self::Msg>,
                msg: Self::Msg,
                sender: Sender) {

        // Use the respective Receive<T> implementation
        match msg {
            Protocol::RegisterZMQListener(__msg) => self.receive_registerzmqlistener(ctx, __msg, sender),
            Protocol::Start => {
                
                let _res = self.timewarp_selecting.tell(Protocol::RegisterRoutee, Some(BasicActorRef::from(ctx.myself().clone())));
               // let _res = self.timewarp_selecting.tell(Protocol::RegisterRoutee, Some(BasicActorRef::from(ctx.myself())));
                // if _res.is_err() {
                //     error!("Erro sending");
                // }
                self.receive_step(ctx,  sender)
            },
            Protocol::Timer => self.receive_step(ctx, sender),
            Protocol::Ping => {
                info!("Ping");
                let _l = sender.unwrap().try_tell(Protocol::Pong, None);
            },
            Protocol::Pong => {
                info!("Pong");
            },
            Protocol::KnownTimewarps(_msg) => self.decide_active(_msg), //TODO
            Protocol::TransactionConfirmed(__msg) => self.receive_transactionconfimation(ctx, __msg, sender),
            _ => ()
        }
    }
   
    
}



impl TimewarpIssuer {
    fn actor(args:(Arc<dyn Persistence>, ActorRef<Protocol> )) -> Self {
        let last_state = args.0.get_timewarp_state();       
        let node = &SETTINGS.node_settings.iri_connection(); 
        TimewarpIssuer {
            state:  if last_state.is_some() {
                last_state.unwrap()
            } else {
                TimewarpIssuingState {
                    seed: generate_new_seed(),
                    latest_index: String::from("99999"),
                    unconfirmed_txs: vec!(),
                    latest_confimed_timestamp: -1,
                    latest_confimed_tx: String::from(""),
                    original_tx: "999999999".to_string(),
                    latest_private_key: Vec::new(),
                    tx_timestamp: HashMap::new(),
                    latest_timestamp: -1
                }
            },
            node: node.to_string(),
            latest_tx: "".to_string(),
            timeout_in_seconds: SETTINGS.timewarp_issuing_settings.interval_in_seconds,
            milestone_interval_in_seconds: 120,
            max_queue_size: 20,
            promote_timeout_in_seconds: 15,
            storage: args.0.clone(),
            active: false,
            timewarp_selecting: args.1
           
        }   
    }
    pub fn props(args:(Arc<dyn Persistence>, ActorRef<Protocol> ))  -> BoxActorProd<TimewarpIssuer> {
        Props::new_args(TimewarpIssuer::actor, args)
    }

    fn should_restart(&mut self) -> bool {
        if !self.active {
            
            return false;
        }
        let diff = crate::now() - (self.milestone_interval_in_seconds * 5);
        //self.state.latest_index_num() == 0 ||
        if  diff > self.state.latest_confimed_timestamp  {
            info!("Restarting timewarp");
            return true;
        };
        false
    }

    fn decide_active(&mut self, timewarps:Vec<TimewarpData>) {
        
        let selfid = &self.state.original_tx[0..9];
        let filtered_maturity:Vec<&TimewarpData> = timewarps.iter().filter(|tw| tw.score() == TimewarpData::max_score() && tw.timestamp > crate::now() - (tw.avg_distance * 3)).collect(); //120 * 5, timewarps that at least are 5 milestones old
        if filtered_maturity.len() >=5 {
            if filtered_maturity.iter().any(|tw| tw.timewarpid == selfid) {
               
                self.active = true;    
            }else{
                if self.active == true {
                    info!("Disabling timewarping");
                }
                self.active = false;
            }
            return;
        }
        let mut filtered_elegible:Vec<&TimewarpData> = timewarps.iter().filter(|tw| tw.score() >= 600 && tw.timestamp > crate::now() - (tw.avg_distance * 5))
            .into_iter().collect();
        if filtered_elegible.len() < 5 {
            if self.active == false {
                info!("Activating timewarping");
            }
            self.active = true;
            return
        }
        filtered_elegible.sort_by(|a, b| b.score().cmp(&a.score())); //descending
        let mut counter = 0;
        for tw in filtered_elegible.iter() {
            if tw.timewarpid == selfid && counter <= 15 {
                if self.active == false {
                    info!("Activating timewarping");
                }
                self.active = true;
                return
            }
            counter += 1;
        }
        self.active = false;
    }

    fn latest_tx_confirmed(&mut self) -> bool {
        //We standard give it 4 milestones to confirm.
        return self.state.latest_confimed_timestamp > crate::now() - self.milestone_interval_in_seconds * 4;
    }

    fn should_step(&mut self) -> bool {
        if !self.active {
            return false;
        }
        let diff = crate::now() - self.state.latest_timestamp;
        (self.latest_tx_confirmed() && diff > self.timeout_in_seconds)
    }

     fn receive_transactionconfimation(&mut self,
                _ctx: &Context<Protocol>,
                _msg: String,
                _sender: Sender) {
                    
                    if self.state.tx_timestamp.contains_key(&_msg) {
                        info!("Timewarp confirmed");
                        let unconfirmed = self.state.unconfirmed_txs.clone();
                        let (mut x, mut rest) = unconfirmed.split_first().unwrap();
                        
                        let mut latest_ts = self.state.tx_timestamp.get(&_msg).unwrap().clone();
                        while x != &_msg {
                            let removed = self.state.tx_timestamp.remove(x);
                            latest_ts = removed.unwrap().clone();
                            let a = rest.split_first();
                            if a.is_none(){
                                break;
                            }
                            let b = a.unwrap();
                            x = b.0;
                            rest = b.1;
                            
                        }
                        self.state.unconfirmed_txs = rest.to_vec();
                        self.state.latest_confimed_tx = _msg.clone();
                        self.state.latest_confimed_timestamp = latest_ts;
                        self.storage.save_timewarp_state(self.state.clone());
                    }
                }


    fn receive_registerzmqlistener(&mut self,
                ctx: &Context<Protocol>,
                msg: RegisterZMQListener,
                _sender: Sender) {        
     
        let res =  msg.zmq_listener.try_tell(Protocol::RegisterRoutee, ctx.myself());

        info!("Registering {:?}", res);
    }


    fn receive_step(&mut self,
                _ctx: &Context<Protocol>,
                _sender: Sender) {
            if self.should_step() {
                    self.issue_next_transaction();
                }else{
            if self.should_restart() {
                self.issue_first_transaction();
            }else{
                self.promote();
            }
                                
            }
            _ctx.schedule_once(
                std::time::Duration::from_secs(15),
                 _ctx.myself(), 
                 None, 
                 Protocol::Timer);
            
           
    }
    fn promote(&mut self) {
        if self.latest_tx != "" {
            let mut iota = iota_client::Client::new(&self.node); //TODO get from settings
            let tips_result = iota.get_transactions_to_approve(GetTransactionsToApproveOptions {
                depth: SETTINGS.timewarp_issuing_settings.tip_selection_depth as usize,
                reference: None
            }).expect("Tips to work");

            let transfer = Transfer {
                address: "AION9TIMEWARP9PROMOTE999999999999999999999999999999999999999999999999999999999999".to_string(),
                tag: "AION9TIMEWARP9PROMOTE999999".to_string(), //Contains the hash to calculated the normalized TW_HASH,               
                ..Transfer::default()
            };
 
            let prepared_transactions = iota.prepare_transfers(&self.state.seed, vec![transfer], options::PrepareTransfersOptions::default());
            let pow_trytes = iota.attach_to_tangle(options::AttachOptions {
                branch_transaction: if SETTINGS.timewarp_issuing_settings.trunk_or_branch {&tips_result.branch_transaction().as_ref().unwrap()} else {&self.latest_tx},
                trunk_transaction: if SETTINGS.timewarp_issuing_settings.trunk_or_branch {&self.latest_tx} else {&tips_result.trunk_transaction().as_ref().unwrap() }, 
                min_weight_magnitude: SETTINGS.timewarp_issuing_settings.minimum_weight_magnitude as usize,
                trytes: &prepared_transactions.unwrap(),
                ..options::AttachOptions::default()
            }).unwrap().trytes().unwrap();
    
            iota.store_transactions(&pow_trytes);
            iota.broadcast_transactions(&pow_trytes);
            let tx: Transaction = pow_trytes[0].parse().unwrap();

            self.latest_tx = tx.hash.to_string();
        }
    }

    fn issue_next_transaction(&mut self)  {
        let mut iota = iota_client::Client::new(&self.node); //TODO get from settings
        let tips_result = iota.get_transactions_to_approve(GetTransactionsToApproveOptions {
            depth: SETTINGS.timewarp_issuing_settings.tip_selection_depth as usize,
            reference: None
        }).expect("Tips to work");
       
        let increased_index = increase_index(&self.state.latest_index); 
        let key_addres = generate_key_and_address(&self.state.seed, self.state.latest_index_num() as usize);
        let latest_tx = self.state.latest_tx();
        let tw_hash = calculate_normalized_timewarp_hash(&key_addres.1,
              if SETTINGS.timewarp_issuing_settings.trunk_or_branch {&latest_tx} else {&tips_result.trunk_transaction().as_ref().unwrap()} ,
              if SETTINGS.timewarp_issuing_settings.trunk_or_branch {&tips_result.branch_transaction().as_ref().unwrap()} else {&latest_tx},
              &increased_index,
             &self.state.random_id());
        let signed_message_fragment = sign_tw_hash(&self.state.latest_private_key, &tw_hash.0);

        let transfer = Transfer {
            address: key_addres.1,
            tag: tw_hash.1, //Contains the hash to calculated the normalized TW_HASH,
            message: signed_message_fragment,
            ..Transfer::default()
        };
        
        let prepared_transactions = iota.prepare_transfers(&self.state.seed, vec![transfer], options::PrepareTransfersOptions::default());
        let pow_trytes = iota.attach_to_tangle(options::AttachOptions {
            branch_transaction: if SETTINGS.timewarp_issuing_settings.trunk_or_branch {&tips_result.branch_transaction().as_ref().unwrap()} else {&latest_tx},
            trunk_transaction: if SETTINGS.timewarp_issuing_settings.trunk_or_branch {&latest_tx} else {&tips_result.trunk_transaction().as_ref().unwrap() }, 
            min_weight_magnitude: SETTINGS.timewarp_issuing_settings.minimum_weight_magnitude as usize,
            trytes: &prepared_transactions.unwrap(),
            ..options::AttachOptions::default()
        }).unwrap().trytes().unwrap();

        iota.store_transactions(&pow_trytes);
        iota.broadcast_transactions(&pow_trytes);

        let tx: Transaction = pow_trytes[0].parse().unwrap();
        
        self.state.unconfirmed_txs.push(tx.hash.to_string());
        self.state.tx_timestamp.insert(tx.hash.to_string(), tx.attachment_timestamp / 1000);
        let new_state = TimewarpIssuingState {
                    latest_index: increased_index,
                    original_tx: self.state.original_tx.clone(),
                    unconfirmed_txs: self.state.unconfirmed_txs.clone(),
                    tx_timestamp: self.state.tx_timestamp.clone(),
                    latest_confimed_timestamp: self.state.latest_confimed_timestamp,
                    latest_confimed_tx: self.state.latest_confimed_tx.clone(),
                    latest_private_key: key_addres.0,
                    seed: self.state.seed.clone(),
                    latest_timestamp: tx.attachment_timestamp / 1000,
                
                    
                };
        self.latest_tx = tx.hash.to_string();
        self.storage.save_timewarp_state(new_state.clone());
        self.state = new_state;
        //let mut txs:Vec<Transaction> = pow_trytes.iter().map(|x| x.parse()).collect();
        info!("Issued new timewarp: {}", &tx.hash.to_string());
    }

    


    fn issue_first_transaction(&mut self)  {
        self.state = TimewarpIssuingState {
                    seed: generate_new_seed(),
                    latest_index: String::from("99999"),
                    latest_confimed_timestamp: 0,
                    latest_confimed_tx: String::from(""),
                    tx_timestamp: HashMap::new(),
                    unconfirmed_txs: vec!(),
                    original_tx: "".to_string(),
                    latest_private_key: Vec::new(),
                    latest_timestamp: 0,
                };

        let mut iota = iota_client::Client::new(&self.node); //TODO get from settings
        let tips_result = iota.get_transactions_to_approve(GetTransactionsToApproveOptions {
            depth: SETTINGS.timewarp_issuing_settings.tip_selection_depth as usize,
            reference: None
        }).expect("Tips to work");
       
        
        let key_addres = generate_key_and_address(&self.state.seed, 0);
        let transfer = Transfer {
            address: key_addres.1,
            ..Transfer::default()
        };
        
        let prepared_transactions = iota.prepare_transfers(&self.state.seed, vec![transfer], options::PrepareTransfersOptions::default());
        let pow_trytes = iota.attach_to_tangle(options::AttachOptions {
            branch_transaction: &tips_result.branch_transaction().as_ref().unwrap(),
            trunk_transaction: &tips_result.trunk_transaction().as_ref().unwrap(),
            min_weight_magnitude: SETTINGS.timewarp_issuing_settings.minimum_weight_magnitude as usize,
            trytes: &prepared_transactions.unwrap(),
            ..options::AttachOptions::default()
        }).unwrap().trytes().unwrap();

        iota.store_transactions(&pow_trytes);
        iota.broadcast_transactions(&pow_trytes);

        let tx: Transaction = pow_trytes[0].parse().unwrap();
        let mut tx_timestamp = HashMap::new();
        tx_timestamp.insert(tx.hash.to_string(), tx.attachment_timestamp/1000);
        let new_state = TimewarpIssuingState {
                    latest_index: self.state.latest_index.to_string(),
                    unconfirmed_txs: vec!(tx.hash.to_string()),
                    original_tx: tx.hash.to_string(),
                    tx_timestamp: tx_timestamp,
                    latest_confimed_timestamp: crate::now(),
                    latest_confimed_tx: String::from(""),
                    latest_private_key: key_addres.0,
                    seed: self.state.seed.clone(),
                    latest_timestamp: tx.attachment_timestamp / 1000,
                };
        self.storage.save_timewarp_state(new_state.clone());
        self.state = new_state;
        self.latest_tx = self.state.original_tx.clone();
        info!("Started new timewarp: {}", &self.state.original_tx);
        //let mut txs:Vec<Transaction> = pow_trytes.iter().map(|x| x.parse()).collect();
        
    }

}


    


  


