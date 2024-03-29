

use zmq::*;
//use std::str;
use std::collections::LinkedList;
use riker::actors::*;
use riker::actors::Context;
use std::time::Duration;
use crate::aionmodel::transaction::*;
use crate::timewarping::Protocol;
use crate::SETTINGS;
use crate::indexstorage::Persistence;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct StartListening {
    pub host: String,
}

#[derive(Clone, Debug)]
pub struct RegisterZMQListener {
    pub zmq_listener: BasicActorRef
}
// #[derive(Clone, Debug)]
// pub struct RegisterRoutee;


#[derive(Clone, Debug)]
pub struct PullTxData;


#[derive(Clone, Debug)]
pub struct NewTransaction {
    pub tx: iota_lib_rs::iota_model::Transaction
}


//#[actor(StartListening, RegisterRoutee, PullTxData)]
pub struct ZMQListener {
     routees: LinkedList<BasicActorRef>,
     socket: Socket,
     storage: Arc<dyn Persistence>
}

impl Actor for ZMQListener {
      // we used the #[actor] attribute so CounterMsg is the Msg type
    type Msg = Protocol;

    fn recv(&mut self,
                ctx: &Context<Self::Msg>,
                msg: Self::Msg,
                sender: Sender) {

        // Use the respective Receive<T> implementation
        match msg {
            Protocol::StartListening(__msg) => self.receive_startlistening(ctx, __msg, sender),
            Protocol::PullTxData(__msg) => self.receive_pulltxdata(ctx, __msg, sender),
            Protocol::RegisterRoutee => self.receive_registerroutee(ctx, msg, sender),
            _ => warn!("Unsupported message")
        }
    }
    
}



impl ZMQListener {
    fn actor(storage: Arc<dyn Persistence>) -> Self {
        let ctx = zmq::Context::new();
        let socket = ctx.socket(zmq::SUB).unwrap();
        ZMQListener {
            routees: LinkedList::new(),
            storage: storage,
            socket: socket
        }
    }
    fn receive_startlistening(&mut self,
                _ctx: &Context<Protocol>,
                _msg: StartListening,
                _sender: Sender) {
        println!("Got start listening {}", _msg.host);
        let node = &SETTINGS.node_settings.zmq_connection(); 
        self.socket.connect(&node).unwrap();
        self.socket.set_subscribe("trytes ".as_bytes()).unwrap();
        self.socket.set_subscribe("sn ".as_bytes()).unwrap();
        self.socket.set_subscribe("lmi ".as_bytes()).unwrap();

        _ctx.myself.tell(Protocol::PullTxData(PullTxData), None);
      
    }


    fn receive_registerroutee(&mut self,
                _ctx: &Context<Protocol>,
                _msg: Protocol,
                _sender: Sender) {
        println!("Got routee");
        self.routees.push_back(_sender.unwrap());
        //println!("Got start listening {}", _msg.host);
    }

    fn receive_pulltxdata(&mut self,
                _ctx: &Context<Protocol>,
                _msg: PullTxData,
                _sender: Sender) {
        
        
        let msg = self.socket.recv_msg(zmq::DONTWAIT);
        if msg.is_ok() {
            let msg = msg.unwrap();
            let msg_string = msg.as_str().unwrap();
            
            if msg_string.starts_with("trytes "){
                // let split: Vec<&str> = msg_string.split(" ").collect();
                // let mut tx: iota_lib_rs::iota_model::Transaction = split[1].parse().unwrap();
                // tx.hash = String::from(split[2]);
                let tx = parse_zmqtransaction(msg_string);
                for routee in &self.routees {
                    let _res = routee.try_tell(Protocol::NewTransaction(NewTransaction{tx: tx.clone()}), None);                
                }
            }else if msg_string.starts_with("sn "){
                let tx_ = parse_zmq_confirmation_transaction(msg_string);
                for routee in &self.routees {
                    let _res = routee.try_tell(Protocol::TransactionConfirmed(tx_.to_string()), None);                
                }
            }else if msg_string.starts_with("lmi ") {
                let split: Vec<&str> = msg_string.split(" ").collect();
                info!("New milestone found");
                //No need to parse the number, it is cosmetic
                self.storage.set_generic_cache(crate::indexstorage::LMI_CONST, split[1].parse::<i64>().unwrap().to_be_bytes().to_vec());
                self.storage.set_generic_cache(crate::indexstorage::LMI_TIME, crate::now().to_be_bytes().to_vec()); 
            }         
    
            _ctx.myself().tell(Protocol::PullTxData(PullTxData), None);
        }else{
             //Let it chill a bit otherwise it acts like a infinate loop cause 100% cpu
             let delay = Duration::from_millis(20);
            _ctx.schedule_once(delay, _ctx.myself(), None, Protocol::PullTxData(PullTxData));
           
        }            
    }

    pub fn props(storage: Arc<dyn Persistence>) -> BoxActorProd<ZMQListener> {
        Props::new_args(ZMQListener::actor, storage)
    }
}

