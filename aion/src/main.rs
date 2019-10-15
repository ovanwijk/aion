
#[macro_use]
extern crate actix_web;
extern crate zmq;

extern crate serde_derive;
extern crate serde;
extern crate hocon;
extern crate json;
extern crate serde_json;
extern crate riker;
mod timewarping;
mod aionmodel;
use actix_web::{web, App, HttpServer, HttpRequest, HttpResponse, Responder};
use riker::actors::*;
use hocon::HoconLoader;
use std::*;
use json::JsonValue;

use timewarping::zmqlistener::*;
use timewarping::timewarpindexing::*;
use timewarping::Protocol;

use serde_derive::{Deserialize, Serialize};
//fn index(info: web::Path<(u32, String)>) -> impl Responder {
//    format!("Hello {}! id:{}", info.1, info.0)
//}


#[derive(Serialize, Deserialize, Debug)]
struct MyObj {
    name: String,
    number: i32,
}

/// This handler uses json extractor
fn index(item: web::Json<MyObj>) -> HttpResponse {
    println!("model: {:?}", &item);
    HttpResponse::Ok().json(item.0) // <- send response
}

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    let mut configFile = "./config.conf";
    if (args.len() == 2){
        configFile = &args[1];
    }

    let config = HoconLoader::new()
        .load_file(configFile).unwrap()
        .hocon().unwrap();
    let host = config["iri_host"].as_string();
    let zmq_host = config["zmq_host"].as_string();
    let zmq_port = config["zmq_port"].as_string();
    let api_host = config["iri_host"].as_string();


    let sys = ActorSystem::new().unwrap();
    
    let propszmq = ZMQListener::props();
    let propstwi = TimewarpIndexing::props();
    let my_actor1 = sys.actor_of(propszmq, "zmq-listener").unwrap();
    let my_actor1_2 = my_actor1.clone();
    let my_actor2 = sys.actor_of(propstwi, "timewarp-indexing").unwrap();

    //{ zmq_listener: BasicActorRef::from(my_actor1.clone())}
    let _zmq_listner_result = BasicActorRef::from(my_actor1).try_tell(Protocol::StartListening(StartListening{host:"Hello my actor!".to_string()}),None);
    my_actor2.tell(Protocol::RegisterZMQListener(RegisterZMQListener{zmq_listener: BasicActorRef::from(my_actor1_2)}), None);
  //  my_actor1_2.tell(ZMQListenerMsg::StartListening(StartListening{host:"Hello my actor!".to_string()}), None);

    //std::thread::sleep(time::Duration::from_millis(2500));

    println!("Going to wait...");
    io::stdin().read_line(&mut String::new()).unwrap();
    //timewarping::start();

    // HttpServer::new(
    //     || App::new().service(
    //           web::resource("/{id}/{name}/index.html").to(index)))
    //     .bind("0.0.0.0:8080")
    //     .run();


}