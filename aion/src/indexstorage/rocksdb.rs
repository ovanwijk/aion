
use std::collections::LinkedList;
use serde::{Serialize, Deserialize};
use std::marker::{Send, Sync};
use riker::actors::*;
use riker::actors::Context;
use std::convert::TryInto;
use std::sync::Mutex;
use timewarping::Protocol;

use indexstorage::*;
use crate::SETTINGS;
use rocksdb::{DB, Options, WriteBatch};
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
const PERSISTENT_CACHE:&str = "PERSISTENT_CACHE";
//Persistent cache keys:

const LAST_PICKED_TW_ID:&str = "LAST_PICKED_TW_ID";


#[derive(Debug)]
pub struct RocksDBProvider {
    provider: DB,
    TW_DETECTION_RANGE_TX_INDEX_COLUMN_cache: Mutex<LruCache<i64, HashMap<String, String>>>,
    FOLLOWED_TW_INDEX_RANGE_COLUMN_cache:  Mutex<LruCache<i64, HashMap<String, String>>>,
    LAST_PICKED_TW_cache: Mutex<Option<TimewarpDetectionData>>
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
            self.TW_DETECTION_RANGE_TX_INDEX_COLUMN_cache.lock().unwrap().insert(key, data);
        }
        
    }
    fn tw_detection_remove_from_index(&self, key:i64, values:HashMap<String, String>){}
    fn tw_detection_get(&self, key:&i64) -> HashMap<String, String> {
        let mut borrowed_cached = self.TW_DETECTION_RANGE_TX_INDEX_COLUMN_cache.lock().unwrap();
        let cached = borrowed_cached.get_mut(key);
        if cached.is_some() {
            let result = cached.unwrap();
            result.clone()
        }else { 
            let handle = self.provider.cf_handle(TW_DETECTION_RANGE_TX_INDEX_COLUMN).unwrap();

            let result = match self.provider.get_cf(handle, key.to_be_bytes()) {                
                Ok(Some(value)) => bincode::deserialize(&*value).unwrap(),
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

    fn last_picked_tw(&self) -> Option<TimewarpDetectionData> {
        let cached = self.LAST_PICKED_TW_cache.lock().unwrap();
        if cached.is_some() {
            return Some(cached.as_ref().unwrap().clone());
        }

        let cache_handle = self.provider.cf_handle(PERSISTENT_CACHE).unwrap();
        let handle = self.provider.cf_handle(FOLLOWED_TW_INDEX_COLUMN).unwrap();

        let result = match self.provider.get_cf(cache_handle, LAST_PICKED_TW_ID.as_bytes()) {                
            Ok(Some(value)) => {
                let local_result = match self.provider.get_cf(handle, &*value) {
                    Ok(Some(value_two)) => {
                        let res: Option<TimewarpDetectionData> = bincode::deserialize(&*value_two).unwrap();
                        res
                    }
                    Ok(None) => None,
                    Err(e) => {println!("operational problem encountered: {}", e);
                    None},
                    _ => None
                };
                local_result
            },
            Ok(None) => None,
            Err(e) => {println!("operational problem encountered: {}", e);
            None},
            _ => None
        };        
        result
    }

    fn get_picked_tw_range(&self, key:i64) -> HashMap<String, String> {
        let mut borrowed_cached = self.FOLLOWED_TW_INDEX_RANGE_COLUMN_cache.lock().unwrap();     
        let cached = borrowed_cached.get_mut(&key);
        if cached.is_some() {
            let result = cached.unwrap();
            result.clone()
        }else { 
            let handle = self.provider.cf_handle(FOLLOWED_TW_INDEX_RANGE_COLUMN).unwrap();

            let result = match self.provider.get_cf(handle, key.to_be_bytes()) {                
                Ok(Some(value)) => bincode::deserialize(&*value).unwrap(),
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

    fn add_last_picked_tw(&self, timewarps: Vec<TimewarpDetectionData>) -> Result<(), String> {
        let handle = self.provider.cf_handle(FOLLOWED_TW_INDEX_COLUMN).unwrap();
        let range_handle = self.provider.cf_handle(FOLLOWED_TW_INDEX_RANGE_COLUMN).unwrap();
        let mut batch = WriteBatch::default();
        let mut cache_updates:HashMap<i64, HashMap<String, String>> = HashMap::new();
        for timewarp in timewarps {
            let last_tw = self.last_picked_tw();
            if last_tw.is_some() {
                let unwrapped = last_tw.unwrap();
                if unwrapped.timestamp > timewarp.timestamp {
                    return Err("Given timewarp is older then the one provided".to_string());
                }
                if unwrapped.source == timewarp.target {
                   
                    let _1 = &batch.put_cf(handle, timewarp.source.as_bytes(), bincode::serialize(&timewarp).unwrap());
                    let mut range_map = self.get_picked_tw_range(get_time_key(&timewarp.timestamp));
                    range_map.insert(timewarp.source, timewarp.target);
                    let _2 = &batch.put_cf(range_handle, get_time_key(&timewarp.timestamp).to_be_bytes(), bincode::serialize(&range_map).unwrap());
                    
                    cache_updates.insert(get_time_key(&timewarp.timestamp), range_map);
                } else {
                    return Err("Not a connecting timewarp. Source and target transaction are not matching.".to_string());
                }
            } else {
                let _l = &batch.put_cf(handle, timewarp.source.as_bytes(), bincode::serialize(&timewarp).unwrap());
                let mut range_map = self.get_picked_tw_range(get_time_key(&timewarp.timestamp));
                range_map.insert(timewarp.source, timewarp.target);
                let _2 = &batch.put_cf(range_handle, get_time_key(&timewarp.timestamp).to_be_bytes(), bincode::serialize(&range_map).unwrap());
                    
                cache_updates.insert(get_time_key(&timewarp.timestamp), range_map);
            }
        }
        let _l = self.provider.write(batch);
        if _l.is_ok() {
           let mut borrowed_cache = self.FOLLOWED_TW_INDEX_RANGE_COLUMN_cache.lock().unwrap();
           for cupdate in cache_updates {
               borrowed_cache.insert(cupdate.0, cupdate.1);
           }
        }else{
            return Err("Something went wrong writing batches".to_string());
        }
      
        Ok(())
    }

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
            FLEXIBLE_ZERO,
            PERSISTENT_CACHE]).unwrap();
        
        RocksDBProvider {
            provider: db,
            TW_DETECTION_RANGE_TX_INDEX_COLUMN_cache: Mutex::new(LruCache::new(SETTINGS.cache_settings.db_memory_cache as usize)),
            FOLLOWED_TW_INDEX_RANGE_COLUMN_cache: Mutex::new(LruCache::new(SETTINGS.cache_settings.db_memory_cache as usize)),
            LAST_PICKED_TW_cache: Mutex::new(None)
        }
    }
    
}