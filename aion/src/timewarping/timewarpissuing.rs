

use std::collections::LinkedList;
use std::sync::Arc;

use riker::actors::*;
use riker::actors::Context;
use std::convert::TryInto;
use timewarping::Protocol;
use indexstorage::TimewarpIssuingState;
use crate::SETTINGS;
use indexstorage::{get_time_key, Persistence};
//use iota_client::options::;
use iota_lib_rs::prelude::*;
use iota_utils::generate_new_seed;
use iota_lib_rs::iota_client::*;
use iota_client::client::*;
use iota_client::options::GetTransactionsToApproveOptions;
use iota_model::{Transaction,Transfer};
use iota_conversion::trytes_converter;
use std::collections::HashMap;
use timewarping::signing::*;




#[derive(Clone, Debug)]
pub struct StartTimewarpWalking {
    pub target_hash: String,
    pub source_timestamp: i64,
    pub trunk_or_branch: bool,
    pub last_picked_tw_tx: String
}


#[derive(Debug)]
pub struct TimewarpIssuer {
    state: TimewarpIssuingState,
    timeout_in_seconds: i64,
    promote_timeout_in_seconds: i64,
    storage:Arc<dyn Persistence>,
}

impl Actor for TimewarpIssuer {
    type Msg = Protocol;

    fn recv(&mut self,
                ctx: &Context<Self::Msg>,
                msg: Self::Msg,
                sender: Sender) {

        // Use the respective Receive<T> implementation
        match msg {
            Protocol::Start => self.receive_step(ctx,  sender),
            _ => ()
        }
    }
   
    
}



impl TimewarpIssuer {
    fn actor(storage:Arc<dyn Persistence>) -> Self {
        let last_state = storage.get_timewarp_state();       
        TimewarpIssuer {
            state:  if last_state.is_some() {
                last_state.unwrap()
            } else {
                TimewarpIssuingState {
                    seed: generate_new_seed(),
                    latest_index: 0,
                    latest_tx: "".to_string(),
                    original_tx: "".to_string(),
                    latest_private_key: Vec::new(),
                    next_address: "".to_string()
                }
            },
            timeout_in_seconds: 90,
            promote_timeout_in_seconds: 15,
            storage: storage,
           
        }
    }
    pub fn props(storage_actor:Arc<dyn Persistence>) -> BoxActorProd<TimewarpIssuer> {
        Props::new_args(TimewarpIssuer::actor, storage_actor)
    }

    fn receive_step(&mut self,
                _ctx: &Context<Protocol>,
                _sender: Sender) {
      
           
    }

    fn issue_first_transaction(&self) {
        let mut iota = iota_client::Client::new("http://localhost:14265"); //TODO get from settings
        let tips_result = iota.get_transactions_to_approve(GetTransactionsToApproveOptions {
            depth: SETTINGS.timewarp_issuing_settings.tip_selection_depth as usize,
            reference: None
        });
        if tips_result.is_err() {
            //TODO send self recovery
            return;
        }
        let tips_result_ = tips_result.unwrap();
        let key_addres = generate_key_and_address(&self.state.seed, 0);
        let transfer = Transfer {
            address: key_addres.1,
            ..Transfer::default()
        };
        
        
       
    }

    }


    


  


