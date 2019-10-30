
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
extern crate rocksdb;
extern crate lru_cache;
#[macro_use] extern crate log;
extern crate env_logger;


mod timewarping;
mod aionmodel;
mod indexstorage;

use lazy_static::*;
use iota_lib_rs::*;
use actix_web::{web, App, HttpServer, HttpRequest, HttpResponse, Responder};
use riker::actors::*;
use hocon::HoconLoader;
use std::*;
use json::JsonValue;
use indexstorage::rocksdb::RocksDBProvider;
use indexstorage::TimewarpIndexEntry;
use timewarping::zmqlistener::*;
use timewarping::timewarpindexing::*;
use timewarping::timewarpwalker::*;

use timewarping::Protocol;
use serde_derive::{Deserialize, Serialize};
//fn index(info: web::Path<(u32, String)>) -> impl Responder {
//    format!("Hello {}! id:{}", info.1, info.0)
//}

pub const STORAGE_ACTOR:&str = "storage-actor";
pub const ZMQ_LISTENER_ACTOR:&str = "zmq-actor";
pub const TIMEWARP_INDEXING_ACTOR:&str = "timewarp-actor";

#[derive(Clone, Debug, Deserialize)]
pub struct AppSettings {
    node_settings: NodeSettings,
    timewarp_index_settings: TimewarpIndexSettings,
    cache_settings: CacheSettings
}

#[derive(Clone, Debug, Deserialize)]
pub struct NodeSettings {
    iri_host: String,
    zmq_host: String,
    iri_port: usize,
    zmq_port: usize
}


#[derive(Clone, Debug, Deserialize)]
pub struct CacheSettings {
    local_tangle_max_transactions: usize,
    db_memory_cache: usize
}

#[derive(Clone, Debug, Deserialize)]
pub struct TimewarpIndexSettings {
    detection_threshold_min_timediff_in_seconds: usize,
    detection_threshold_max_timediff_in_seconds: usize,
    time_index_clustering_in_seconds: usize,
    time_index_max_length_in_seconds: usize,
    time_index_database_location: String
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
                    iri_host: lconfig["nodes"]["iri_api_host"].as_string().unwrap(),
                    iri_port: lconfig["nodes"]["iri_api_port"].as_i64().unwrap() as usize,
                    zmq_host: lconfig["nodes"]["zmq_host"].as_string().unwrap(),
                    zmq_port: lconfig["nodes"]["zmq_port"].as_i64().unwrap() as usize
                },
                cache_settings: CacheSettings{
                    local_tangle_max_transactions: lconfig["caching"]["local_tangle_max_transactions"].as_i64().unwrap() as usize,
                    db_memory_cache: 24000
                },
                timewarp_index_settings: TimewarpIndexSettings {
                    detection_threshold_min_timediff_in_seconds: lconfig["timewarp_indexing"]["detection_threshold_min_timediff_in_seconds"].as_i64().unwrap() as usize,
                    detection_threshold_max_timediff_in_seconds: lconfig["timewarp_indexing"]["detection_threshold_max_timediff_in_seconds"].as_i64().unwrap() as usize,
                    time_index_clustering_in_seconds: lconfig["timewarp_indexing"]["time_index_clustering_in_seconds"].as_i64().unwrap() as usize,
                    time_index_max_length_in_seconds: lconfig["timewarp_indexing"]["time_index_max_length_in_seconds"].as_i64().unwrap() as usize,
                    time_index_database_location: lconfig["timewarp_indexing"]["time_index_database_location"].as_string().unwrap()
                }
            }
            
        }
    };
}
fn main() {
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    info!("{:?}", args);
    debug!("test");
    println!("{}", std::env::var("RUST_LOG").unwrap_or_default());
    let mut config_file = "./config.conf";
    if args.len() == 2 {
        config_file = &args[1];
    }
      let ldr = HoconLoader::new();
     let fll = ldr.load_file(&config_file);
        if fll.is_err() {
            let herror = fll.unwrap_err();
            println!("{}", herror);
            return ()
        }
    unsafe{
        let hcon = fll.unwrap().hocon();
         if hcon.is_err() {
            let herror = hcon.unwrap_err();
            println!("{}", herror);
            return ()
        }
        HOCON_CONFIG = Some(hcon.unwrap());
    }

    lazy_static::initialize(&SETTINGS);

    let sys = ActorSystem::new().unwrap();
    
    
    let zmq_actor = sys.actor_of(ZMQListener::props(), ZMQ_LISTENER_ACTOR).unwrap();
    let my_actor1_2 = zmq_actor.clone(); 
     let storage_actor = sys.actor_of(RocksDBProvider::props(), STORAGE_ACTOR).unwrap();


    let indexing_actor = sys.actor_of(TimewarpIndexing::props(BasicActorRef::from(storage_actor.clone())), TIMEWARP_INDEXING_ACTOR).unwrap();
   
    let temp_actor = sys.actor_of(TimewarpWalker::props(BasicActorRef::from(storage_actor.clone())), "timewarp-walking").unwrap();

    storage_actor.tell(Protocol::AddToIndexPersistence(TimewarpIndexEntry{key: 10, values: vec!["Hallo".to_string(), "world".to_string()]}), None);
    storage_actor.tell(Protocol::GetFromIndexPersistence(10), None);
    
    // temp_actor.tell(Protocol::StartTimewarpWalking(StartTimewarpWalking { 
    //     target_hash: "LRYSLAXS9ZJYMQ9ENALNCRNUJBIFNVJTEOILGMJVJAMPYH9EBQBPGDXPTCZUR9ATTYZBANMPQIDTWNNK9".to_string(), 
    //     source_timestamp: 1571333939, 
    //     trunk_or_branch: false})
    //     , None);
    //{ zmq_listener: BasicActorRef::from(my_actor1.clone())}
    let _zmq_listner_result = BasicActorRef::from(zmq_actor).try_tell(Protocol::StartListening(StartListening{host:"Hello my actor!".to_string()}),None);
    indexing_actor.tell(Protocol::RegisterZMQListener(RegisterZMQListener{zmq_listener: BasicActorRef::from(my_actor1_2)}), None);
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