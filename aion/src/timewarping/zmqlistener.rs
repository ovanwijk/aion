

use zmq::*;
//use std::str;
use std::collections::LinkedList;
use riker::actors::*;
use riker::actors::Context;

use aionmodel::transaction::*;
use timewarping::Protocol;


#[derive(Clone, Debug)]
pub struct StartListening {
    pub host: String,
}

// #[derive(Clone, Debug)]
// pub struct RegisterRoutee;


#[derive(Clone, Debug)]
pub struct PullTxData;


#[derive(Clone, Debug)]
pub struct NewTransaction {
    pub tx: Transaction
}


//#[actor(StartListening, RegisterRoutee, PullTxData)]
pub struct ZMQListener {
     routees: LinkedList<BasicActorRef>,
     socket: Socket
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
            _ => ()
        }
    }
    
}



impl ZMQListener {
    fn actor() -> Self {
        let ctx = zmq::Context::new();
        let socket = ctx.socket(zmq::SUB).unwrap();
        ZMQListener {
            routees: LinkedList::new(),
            socket: socket
        }
    }
    fn receive_startlistening(&mut self,
                _ctx: &Context<Protocol>,
                _msg: StartListening,
                _sender: Sender) {
        println!("Got start listening {}", _msg.host);

        self.socket.connect("tcp://127.0.0.1:5556").unwrap();
        self.socket.set_subscribe("tx ".as_bytes()).unwrap();

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
       //  println!("pulling data");
       // let mut counter = 0;
        
        
            let msg = self.socket.recv_msg(zmq::DONTWAIT);
            if msg.is_ok() {
                let msg = msg.unwrap();
                let msg_string = msg.as_str().unwrap();

                let tx_ = parse_zmqtransaction(&msg_string.to_string());
                //let routees = ;
                for routee in &self.routees {
                    //let nTx = ;
                    routee.try_tell(Protocol::NewTransaction(NewTransaction{tx: tx_.clone()}), None);
                    //routee.
                }
                //println!("test {}", msg_string);
            }
            _ctx.myself().tell(Protocol::PullTxData(PullTxData), None);
        //println!("Got start listening {}", _msg.host);
    }

    pub fn props() -> BoxActorProd<ZMQListener> {
        Props::new(ZMQListener::actor)
    }
}

