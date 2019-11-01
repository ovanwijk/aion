
use std::collections::LinkedList;
use serde::{Serialize, Deserialize};

use riker::actors::*;
use riker::actors::Context;
use std::convert::TryInto;
use timewarping::Protocol;
 use std::cell::RefCell;
use indexstorage::*;
use crate::SETTINGS;
use rocksdb::{DB, Options, ColumnFamily};
use std::{
    collections::{HashMap, HashSet}};
use lru_cache::LruCache;

const TW_DETECTION_RANGE_TX_INDEX_COLUMN:&str = "TW_DETECTION_RANGE_TX_INDEX_COLUMN";
const TW_DETECTION_RANGE_GROUP_INDEX_COLUMN:&str = "TW_DETECTION_RANGE_GROUP_INDEX_COLUMN";
const FOLLOWED_TW_INDEX_COLUMN:&str = "FOLLOWED_TW_INDEX_COLUMN";
const FOLLOWED_TW_INDEX_RANGE_COLUMN:&str = "FOLLOWED_TW_INDEX_RANGE_COLUMN";
const PINNED_TX_COUNTER:&str = "PINNED_TX_COUNTER";
const PATHWAY_DESCRIPTORS:&str = "PATHWAY_DESCRIPTORS";
const FLEXIBLE_ZERO:&str = "FLEXIBLE_ZERO";



// impl TimewarpIndexEntry {
//     pub fn from_vec_u8(to_parse:String) -> HashSet<String> {
//         to_parse.split(',').map(|x| String::from(x)).collect()
//     }
// }



pub struct RocksDBProvider {
    provider: DB,
    cache: RefCell<LruCache<i64, HashMap<String, String>> >
}

impl Actor for RocksDBProvider {
      // we used the #[actor] attribute so CounterMsg is the Msg type
    type Msg = Protocol;

    fn recv(&mut self,
                ctx: &Context<Self::Msg>,
                msg: Self::Msg,
                sender: Sender) {

        // Use the respective Receive<T> implementation
        match msg {
            //RemoveFromIndexPersistence
            Protocol::AddToIndexPersistence(key, values) => self.receive_addtoindex(ctx, key, values, sender),
            Protocol::RemoveFromIndexPersistence(__msg) => self.receive_removefromindex(ctx, __msg, sender),
            Protocol::GetFromIndexPersistence(__msg) => self.receive_getfromindex(ctx, __msg, sender),
            _ => ()
        }
    }
}

impl  TwDetectionPersistence for RocksDBProvider {
    fn tw_detection_add_to_index(&self, key:i64, values:Vec<(String, String)>) {
        let handle = self.provider.cf_handle(TW_DETECTION_RANGE_TX_INDEX_COLUMN).unwrap();
        let mut data = self.tw_detection_get(&key);
        for v in values.iter() {            
            data.insert(v.0.to_string(), v.1.to_string());
        }
       // values.iter().map(|v| d;
        if !data.is_empty() {
            //let concatted = data.iter().fold(String::from(""), |mut a, b| {a.push_str(","); a.push_str(b ); a})[1..].to_string();
            let _r = self.provider.put_cf(handle, key.to_be_bytes(), bincode::serialize(&data).unwrap());
            self.cache.borrow_mut().insert(key, data);
        }
        
    }
    fn tw_detection_remove_from_index(&self, key:i64, values:HashMap<String, String>){}
    fn tw_detection_get(&self, key:&i64) -> HashMap<String, String> {
        let mut borrowed_cached = self.cache.borrow_mut();
        let cached = borrowed_cached.get_mut(key);
        if cached.is_some() {
            let result = cached.unwrap();
            result.clone()
        }else { 
            let handle = self.provider.cf_handle(TW_DETECTION_RANGE_TX_INDEX_COLUMN).unwrap();

            match self.provider.get_cf(handle, key.to_be_bytes()) {                
                Ok(Some(value)) => bincode::deserialize(&*value).unwrap(),//TimewarpIndexEntry::from_vec_u8(value.to_utf8().unwrap().to_string()),
                Ok(None) => HashMap::new(),
                Err(e) => {println!("operational problem encountered: {}", e);
                HashMap::new()},
                _ => HashMap::new()
            }
        }
       
    }
    fn tw_detection_get_all(&self, keys:Vec<&i64>) ->HashMap<String, String> {HashMap::new()}
}

impl RocksDBProvider {
    fn actor() -> Self {
        let path = SETTINGS.timewarp_index_settings.time_index_database_location.clone();
        let mut db_opts = Options::default();
        db_opts.create_missing_column_families(true);
        db_opts.create_if_missing(true);      
        let db = DB::open_cf(&db_opts, path, vec![TW_DETECTION_RANGE_TX_INDEX_COLUMN,
            TW_DETECTION_RANGE_GROUP_INDEX_COLUMN,
            FOLLOWED_TW_INDEX_COLUMN,
            FOLLOWED_TW_INDEX_RANGE_COLUMN,
            PINNED_TX_COUNTER,
            PATHWAY_DESCRIPTORS,
            FLEXIBLE_ZERO]).unwrap();
        
        RocksDBProvider {
            provider: db,
            cache: RefCell::new(LruCache::new(SETTINGS.cache_settings.db_memory_cache as usize))
        }
    }
    fn receive_addtoindex(&mut self,
                _ctx: &Context<Protocol>,
                key: i64,
                values: Vec<(String, String)>,
                _sender: Sender) {

                    
                    self.tw_detection_add_to_index(key, values);
    
    }
    fn receive_getfromindex(&mut self,
                _ctx: &Context<Protocol>,
                index: i64,
                _sender: Sender){

            if _sender.is_some(){
                _sender.unwrap().try_tell(Protocol::GetFromIndexPersistenceResponse(index, self.tw_detection_get(&index)), None);
            }
    }

     fn receive_removefromindex(&mut self,
                _ctx: &Context<Protocol>,
                _msg: RangeTxIDLookup,
                _sender: Sender) {
      //  println!("Got start listening {}", _msg.host);

      //  self.socket.connect("tcp://127.0.0.1:5556").unwrap();
       // self.socket.set_subscribe("tx ".as_bytes()).unwrap();

        //_ctx.myself.tell(Protocol::PullTxData(PullTxData), None);
      //
    }


    pub fn props() -> BoxActorProd<RocksDBProvider> {
        Props::new(RocksDBProvider::actor)
    }
}