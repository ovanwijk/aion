
use std::collections::LinkedList;
use serde::{Serialize, Deserialize};
use std::marker::{Send, Sync};
use riker::actors::*;
use riker::actors::Context;
use std::convert::TryInto;
use std::sync::Mutex;
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



#[derive(Debug)]
pub struct RocksDBProvider {
    provider: DB,
    cache: Mutex<LruCache<i64, HashMap<String, String>> >
}


impl Persistence for RocksDBProvider {
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
            self.cache.lock().unwrap().insert(key, data);
        }
        
    }
    fn tw_detection_remove_from_index(&self, key:i64, values:HashMap<String, String>){}
    fn tw_detection_get(&self, key:&i64) -> HashMap<String, String> {
        let mut borrowed_cached = self.cache.lock().unwrap();
        let cached = borrowed_cached.get_mut(key);
        if cached.is_some() {
            let result = cached.unwrap();
            result.clone()
        }else { 
            let handle = self.provider.cf_handle(TW_DETECTION_RANGE_TX_INDEX_COLUMN).unwrap();

            let result = match self.provider.get_cf(handle, key.to_be_bytes()) {                
                Ok(Some(value)) => bincode::deserialize(&*value).unwrap(),//TimewarpIndexEntry::from_vec_u8(value.to_utf8().unwrap().to_string()),
                Ok(None) => HashMap::new(),
                Err(e) => {println!("operational problem encountered: {}", e);
                HashMap::new()},
                _ => HashMap::new()
            };
            if result.len() > 0 {
                borrowed_cached.insert(key.clone(), result.clone());
            }
            result

        }
       
    }
    fn tw_detection_get_all(&self, keys:Vec<&i64>) ->HashMap<String, String> {HashMap::new()}
}
unsafe impl Send for RocksDBProvider {}
unsafe impl Sync for RocksDBProvider {}
impl RocksDBProvider {
  
    pub fn new() -> RocksDBProvider {
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
            cache: Mutex::new(LruCache::new(SETTINGS.cache_settings.db_memory_cache as usize))
        }
    }
    
}