
#[macro_use]
extern crate actix_web;
extern crate zmq;
extern crate lazy_static;
extern crate serde_derive;
extern crate serde;
extern crate hocon;
extern crate json;
extern crate serde_json;
extern crate riker;
extern crate iota_lib_rs;


mod timewarping;
mod aionmodel;

use lazy_static::*;
use iota_lib_rs::*;
use actix_web::{web, App, HttpServer, HttpRequest, HttpResponse, Responder};
use riker::actors::*;
use hocon::HoconLoader;
use std::*;
use json::JsonValue;

use timewarping::zmqlistener::*;
use timewarping::timewarpindexing::*;
use timewarping::timewarpwalker::*;
use timewarping::Protocol;

use serde_derive::{Deserialize, Serialize};
//fn index(info: web::Path<(u32, String)>) -> impl Responder {
//    format!("Hello {}! id:{}", info.1, info.0)
//}


#[derive(Clone, Debug, Deserialize)]
pub struct AppSettings {
    node_settings: NodeSettings,
    timewarp_index_settings: TimewarpIndexSettings,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NodeSettings {
    iri_host: String,
    zmq_host: String,
    iri_port: usize,
    zmq_port: usize
}

#[derive(Clone, Debug, Deserialize)]
pub struct TimewarpIndexSettings {
    detection_threshold_lower_bound_in_seconds: usize,
    detection_threshold_upper_bound_in_seconds: usize,
}

// /// This handler uses json extractor
// fn index(item: web::Json<MyObj>) -> HttpResponse {
//     println!("model: {:?}", &item);
//     HttpResponse::Ok().json(item.0) // <- send response
// }


static mut HOCON_CONFIG:Option<hocon::Hocon> = None; 
lazy_static! {
    pub static ref SETTINGS: AppSettings = {
        unsafe{
            let lconfig = HOCON_CONFIG.clone().unwrap();
            AppSettings {
                node_settings: NodeSettings {
                    iri_host: lconfig["iri_api_host"].as_string().unwrap(),
                    iri_port: lconfig["iri_api_port"].as_i64().unwrap() as usize,
                    zmq_host: lconfig["zmq_host"].as_string().unwrap(),
                    zmq_port: lconfig["zmq_port"].as_i64().unwrap() as usize
                },
                timewarp_index_settings: TimewarpIndexSettings {
                    detection_threshold_lower_bound_in_seconds: lconfig["timewarp_indexing"]["detection_threshold_lower_bound_in_seconds"].as_i64().unwrap() as usize,
                    detection_threshold_upper_bound_in_seconds: lconfig["timewarp_indexing"]["detection_threshold_upper_bound_in_seconds"].as_i64().unwrap() as usize
                }
            }
            
        }
    };
}
fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    let mut config_file = "./config.conf";
    if args.len() == 2 {
        config_file = &args[1];
    }
    unsafe{
        let ldr = HoconLoader::new();
        let fll = ldr.load_file(&config_file);
        HOCON_CONFIG = Some(fll.unwrap().hocon().unwrap());
    }

    lazy_static::initialize(&SETTINGS);

    let sys = ActorSystem::new().unwrap();
    
    let propszmq = ZMQListener::props();
    let propstwi = TimewarpIndexing::props();
    let my_actor1 = sys.actor_of(propszmq, "zmq-listener").unwrap();
    let my_actor1_2 = my_actor1.clone();
    let my_actor2 = sys.actor_of(propstwi, "timewarp-indexing").unwrap();


    let my_actor3 = sys.actor_of(TimewarpWalker::props(), "timewarp-walking").unwrap();


    my_actor3.tell(Protocol::StartTimewarpWalking(StartTimewarpWalking { 
        target_hash: "LRSUVNCHREADXPCRQH99XMRXOXQSZIYKQXRXOYSCLZWQ9KDEWUTFNKOLWYDOPEUCPWLMVEMTJMFGRYSBC".to_string(), 
        source_timestamp: 1000000, 
        trunk_or_branch: true, 
        warp_step: 10000, 
        warp_error: 0.2})
        , None);
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