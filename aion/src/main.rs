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
//extern crate bincode;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate failure;
extern crate futures;

//type Result<T> = ::std::result::Result<T, failure::Error>;
mod timewarping;
mod aionmodel;
mod indexstorage;
mod pathway;
mod webapi;
mod iota_api;

use std::time::{SystemTime, UNIX_EPOCH};
use lazy_static::*;
use iota_lib_rs::*;
use actix_web::{
     App, HttpServer,
};
use riker::actors::*;
use hocon::HoconLoader;
use std::*;
use std::sync::Arc;
use indexstorage::rocksdb::RocksDBProvider;
use indexstorage::{Persistence};
use timewarping::zmqlistener::*;
use timewarping::timewarpindexing::*;

use timewarping::timewarpissuing::*;
use timewarping::timewarpselecting::*;
use timewarping::transactionpinning::*;

use timewarping::Protocol;
use serde_derive::{Deserialize};

// use actix_web::{
//     error, guard, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder,
//     Result,
// };

//fn index(info: web::Path<(u32, String)>) -> impl Responder {
//    format!("Hello {}! id:{}", info.1, info.0)
//}

pub const STORAGE_ACTOR:&str = "storage-actor";
pub const ZMQ_LISTENER_ACTOR:&str = "zmq-actor";
pub const TIMEWARP_INDEXING_ACTOR:&str = "timewarp-actor";
pub const TIMEWARP_ISSUER_ACTOR:&str = "timewarp-issuer-actor";
pub const TIMEWARP_SELECTION_ACTOR:&str = "timewarp-selection-actor";
pub const PINNING_ACTOR:&str = "pinning-actor";

#[derive(Clone, Debug, Deserialize)]
pub struct AppSettings {
    node_settings: NodeSettings,
    timewarp_index_settings: TimewarpIndexSettings,
    cache_settings: CacheSettings,
    timewarp_issuing_settings: TimewarpIssuingSettings,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NodeSettings {
    iri_host: String,
    zmq_host: String,
    iri_port: usize,
    zmq_port: usize,
    zmq_protocol: String,
    iri_protocol: String
}

impl NodeSettings {
    pub fn zmq_connection(&self) -> String {
        format!("{}://{}:{}", self.zmq_protocol,self.zmq_host, self.zmq_port)
    }
    pub fn iri_connection(&self) -> String {
        format!("{}://{}:{}", self.iri_protocol, self.iri_host, self.iri_port)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct CacheSettings {
    local_tangle_max_transactions: i64,
    db_memory_cache: i64
}


#[derive(Clone, Debug, Deserialize)]
pub struct TimewarpIssuingSettings {
    interval_in_seconds: i64,
    promote_interval_in_seconds: i64,
    tip_selection_depth: i64,
    minimum_weight_magnitude: i64,
    trunk_or_branch: bool
}

#[derive(Clone, Debug, Deserialize)]
pub struct TimewarpIndexSettings {
    detection_threshold_min_timediff_in_seconds: i64,
    detection_threshold_max_timediff_in_seconds: i64,
    detection_threshold_switch_timewarp_in_seconds: i64,
    time_index_clustering_in_seconds: i64,
    time_index_max_length_in_seconds: i64,
    time_index_database_location: String,
    selection_idle_timeout_in_seconds: i64,
    selection_switch_delay_in_seconds: i64,
    selection_better_option_reference_delay_in_seconds: i64,
    selection_xor_key: String
}


pub fn now() -> i64 {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH);
    since_the_epoch.unwrap().as_secs() as i64
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
                    zmq_port: lconfig["nodes"]["zmq_port"].as_i64().unwrap() as usize,
                    iri_protocol: lconfig["nodes"]["iri_api_protocol"].as_string().unwrap(),
                    zmq_protocol: lconfig["nodes"]["zmq_protocol"].as_string().unwrap(),
                },
                cache_settings: CacheSettings{
                    local_tangle_max_transactions: lconfig["caching"]["local_tangle_max_transactions"].as_i64().unwrap() ,
                    db_memory_cache: 24000
                },
                timewarp_index_settings: TimewarpIndexSettings {
                    detection_threshold_min_timediff_in_seconds: lconfig["timewarp_indexing"]["detection_threshold_min_timediff_in_seconds"].as_i64().unwrap() ,
                    detection_threshold_max_timediff_in_seconds: lconfig["timewarp_indexing"]["detection_threshold_max_timediff_in_seconds"].as_i64().unwrap() ,
                    detection_threshold_switch_timewarp_in_seconds: lconfig["timewarp_indexing"]["detection_threshold_switch_timewarp_in_seconds"].as_i64().unwrap() ,
                    time_index_clustering_in_seconds: lconfig["timewarp_indexing"]["time_index_clustering_in_seconds"].as_i64().unwrap(),
                    time_index_max_length_in_seconds: lconfig["timewarp_indexing"]["time_index_max_length_in_seconds"].as_i64().unwrap(),
                    time_index_database_location: lconfig["timewarp_indexing"]["time_index_database_location"].as_string().unwrap(),
                    selection_idle_timeout_in_seconds: 60,
                    selection_switch_delay_in_seconds: 60,
                    selection_better_option_reference_delay_in_seconds: 60,
                    selection_xor_key: "".to_string()
                },
                timewarp_issuing_settings: TimewarpIssuingSettings {
                    interval_in_seconds: lconfig["timewarp_issuing"]["interval_in_seconds"].as_i64().unwrap(),
                    promote_interval_in_seconds: lconfig["timewarp_issuing"]["promote_interval_in_seconds"].as_i64().unwrap(),
                    tip_selection_depth: lconfig["timewarp_issuing"]["tip_selection_depth"].as_i64().unwrap(), 
                    minimum_weight_magnitude: lconfig["timewarp_issuing"]["minimum_weight_magnitude"].as_i64().unwrap(),
                    trunk_or_branch: lconfig["timewarp_issuing"]["trunk_or_branch"].as_bool().unwrap()
                }
            }
            
        }
    };
}

// struct AskActor {
//     future: Future<>
// }

pub struct APIActors {
    storage: Arc<dyn Persistence>,
    actor_system: Arc<ActorSystem>,
    tw_selecting: riker::actor::ActorRef<timewarping::Protocol>
}

// impl APIActors {
//     pub async fn ask_pattern (&self, actor:riker::actor::ActorRef<timewarping::Protocol>) -> timewarping::Protocol {
//         self.actor_system.
//     }
// }


#[actix_rt::main]
async fn main() -> io::Result<()> {
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    info!("{:?}", args);
    debug!("test");
    println!("{}", std::env::var("RUST_LOG").unwrap_or_default());
    let mut config_file = "./config.conf";
    if args.len() >= 2 {
        config_file = &args[1];
    }
    
    let do_timewarp = &args.contains(&String::from("--timewarp"));
    let only_timewarp = &args.contains(&String::from("--only-timewarp"));
    let no_api = &args.contains(&String::from("--no-api"));
    if *do_timewarp {info!("Timewarping");}
    if *only_timewarp {info!("only_timewarp");}
    let ldr = HoconLoader::new();
    let fll = ldr.load_file(&config_file);
    if fll.is_err() {
        let herror = fll.unwrap_err();
        println!("{}", herror);
        return Ok(());
    }
    unsafe{
        let hcon = fll.unwrap().hocon();
         if hcon.is_err() {
            let herror = hcon.unwrap_err();
            println!("{}", herror);
            return Ok(());
        }
        HOCON_CONFIG = Some(hcon.unwrap());
    }

    lazy_static::initialize(&SETTINGS);

    let storage:Arc<dyn Persistence> = Arc::new(RocksDBProvider::new());

    let sys = ActorSystem::new().unwrap();
   
    // let _a = crate::iota::pin_transaction_hash(SETTINGS.node_settings.iri_connection(),
    //      vec!("AQEXQMGPTUARFAYMMHNJMNKQGQCRSTZGSUOMGG9CIOOMTHP99KMYVUHJTEGZKXLCVBBFLEMTUIMCAQFG9".to_string())).await;
    let zmq_actor = sys.actor_of(ZMQListener::props(), ZMQ_LISTENER_ACTOR).unwrap();
    let tw_selection_actor = sys.actor_of(TimewarpSelecting::props(storage.clone()), TIMEWARP_SELECTION_ACTOR).unwrap();

    
    let transactionpinning_actor = sys.actor_of(TransactionPinning::props(storage.clone()), PINNING_ACTOR).unwrap();
    transactionpinning_actor.clone().tell(Protocol::Timer, None);
   //  let storage_actor = sys.actor_of(RocksDBProvider::props(), STORAGE_ACTOR).unwrap();

    if !only_timewarp {

        let indexing_actor = sys.actor_of(TimewarpIndexing::props((storage.clone(), tw_selection_actor.clone())), TIMEWARP_INDEXING_ACTOR).unwrap();
        indexing_actor.tell(Protocol::RegisterZMQListener(RegisterZMQListener{zmq_listener: BasicActorRef::from(zmq_actor.clone())}), None);
        indexing_actor.tell(Protocol::Timer, None);
        //let temp_actor = sys.actor_of(TimewarpWalker::props(storage.clone()), "timewarp-walking").unwrap();
    }
    //storage_actor.tell(Protocol::AddToIndexPersistence(TimewarpIndexEntry{key: 10, values: vec!["Hallo".to_string(), "world".to_string()]}), None);
   // storage_actor.tell(Protocol::GetFromIndexPersistence(10), None);
    
    // temp_actor.tell(Protocol::StartTimewarpWalking(StartTimewarpWalking { 
    //     target_hash: "LRYSLAXS9ZJYMQ9ENALNCRNUJBIFNVJTEOILGMJVJAMPYH9EBQBPGDXPTCZUR9ATTYZBANMPQIDTWNNK9".to_string(), 
    //     source_timestamp: 1571333939, 
    //     trunk_or_branch: false})
    //     , None);
    //{ zmq_listener: BasicActorRef::from(my_actor1.clone())}
    
   
    let tw_issuing_actor = if *do_timewarp || *only_timewarp {
        info!("Start timewarping");
        let timewarp_actor = &sys.actor_of(TimewarpIssuer::props(storage.clone()), TIMEWARP_ISSUER_ACTOR).unwrap();
        &timewarp_actor.tell(Protocol::RegisterZMQListener(RegisterZMQListener{zmq_listener: BasicActorRef::from(zmq_actor.clone())}), None);
        &timewarp_actor.tell(Protocol::Start, None);
        Some(timewarp_actor.clone())
    }else{ None };
    let _zmq_listner_result = BasicActorRef::from(zmq_actor).try_tell(Protocol::StartListening(StartListening{host:"Hello my actor!".to_string()}),None);
  //  my_actor1_2.tell(ZMQListenerMsg::StartListening(StartListening{host:"Hello my actor!".to_string()}), None);

    //std::thread::sleep(time::Duration::from_millis(2500));

    println!("Going to wait...");
   // io::stdin().read_line(&mut String::new()).unwrap();
    //timewarping::start();


    

    // HttpServer::new(|| 
    //     App::new()
    //     .service(
    //         web::resource("/test").to(|req: HttpRequest| match *req.method() {
    //             Method::GET => HttpResponse::Ok(),
    //             Method::POST => HttpResponse::MethodNotAllowed(),
    //             _ => HttpResponse::NotFound(),
    //         })))
    //     .bind("127.0.0.1:0")?
    //     .run()
    //     .await
    if *no_api {
        println!("Going to wait...");
        io::stdin().read_line(&mut String::new()).unwrap();
    }
    let arc_system = Arc::new(sys);
    HttpServer::new(
        move || App::new().data(APIActors {
            storage: storage.clone(),
            actor_system: arc_system.clone(),
            tw_selecting: tw_selection_actor.clone()
        })
        .service(webapi::lifelineUnpinnedF)
        .service(webapi::lifelineIdFn)
        .service(webapi::timewarpPickedFn)
        .service(webapi::timewarpstateFn)
        .service(webapi::timewarpsFn)
        .service(webapi::timewarpIdFn)
        .service(webapi::timewarpIdMaxFn)
        

        
            // .service(
            //   web::resource("/test").route(web::get().to(index)))
            )
        .bind("0.0.0.0:8080")?
        .run().await


}