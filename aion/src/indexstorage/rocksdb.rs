
use std::collections::LinkedList;
use serde_json::json;
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
const TW_ISSUING_STATE:&str = "TW_ISSUING_STATE";


#[derive(Debug)]
pub struct RocksDBProvider {
    provider: DB,
    TW_DETECTION_RANGE_TX_INDEX_COLUMN_cache: Mutex<LruCache<i64, HashMap<String, String>>>,
    FOLLOWED_TW_INDEX_RANGE_COLUMN_cache:  Mutex<LruCache<i64, HashMap<String, String>>>,
    FOLLOWED_TW_INDEX_COLUMN_cache:  Mutex<LruCache<String, TimewarpData>>,
    LAST_PICKED_TW_cache: Mutex<Option<TimewarpData>>
}


impl Persistence for RocksDBProvider {


    fn tw_detection_add_decision_data(&self, tw:  crate::timewarping::Timewarp) -> TimewarpData { 
        let previous = self.tw_detection_get_decision_data(tw.target_hash().to_string());
        let tostore = if previous.is_some() {
            let advanced = previous.unwrap().advance(&tw);
            info!("Advancing {}: Index: {} Avg distance {}", &advanced.timewarpid, &advanced.index_since_id, &advanced.avg_distance );
            advanced
        }else{
            info!("New timewarp: {}", tw.source_hash.clone());
            TimewarpData {
                timewarpid: tw.source_hash.clone(),
                hash: tw.source_hash.clone(),
                branch: tw.source_branch,
                trunk: tw.source_trunk,
                timestamp: tw.source_timestamp,
                timestamp_deviation_factor: -1.0,
                distance: tw.distance,
                index_since_id: 0,
                avg_distance: tw.distance,
                trunk_or_branch: tw.trunk_or_branch
            }
        };
        let handle = self.provider.cf_handle(FOLLOWED_TW_INDEX_COLUMN).unwrap();
        let _r = self.provider.put_cf(handle, tw.source_hash.as_bytes(), serde_json::to_vec(&tostore).unwrap());
        
        let mut borrowed_cached = self.FOLLOWED_TW_INDEX_COLUMN_cache.lock().unwrap();
        borrowed_cached.insert(tostore.hash.clone(), tostore.clone());
       
        tostore
    }
    fn tw_detection_get_decision_data(&self, key: String) -> Option<TimewarpData> {
        let mut borrowed_cached = self.FOLLOWED_TW_INDEX_COLUMN_cache.lock().unwrap();
        let cached = borrowed_cached.get_mut(&key);
        if cached.is_some() {
            return Some(cached.unwrap().clone());
        }else {
            let handle = self.provider.cf_handle(FOLLOWED_TW_INDEX_COLUMN).unwrap();
            let result: Option<TimewarpData> = match self.provider.get_cf(handle,  key.as_bytes()) {                
                Ok(Some(value)) => Some(serde_json::from_slice(&*value).expect("Deserialisation to work")),
                Ok(None) => None,
                Err(e) => {println!("operational problem encountered: {}", e);
                None},
                _ => None
            };

            if result.is_some() {
                info!("Obtained from storage");
                borrowed_cached.insert(key.clone(), result.clone().unwrap().clone());
            }
            result
        }


    }

   
    fn save_timewarp_state(&self, state: TimewarpIssuingState) {
        
        let handle = self.provider.cf_handle(PERSISTENT_CACHE).unwrap();
        let _r = self.provider.put_cf(handle, TW_ISSUING_STATE.as_bytes(), serde_json::to_vec(&state).unwrap()).unwrap();
    }
    fn get_timewarp_state(&self) -> Option<TimewarpIssuingState> {
        let handle = self.provider.cf_handle(PERSISTENT_CACHE).unwrap();
        
        match self.provider.get_cf(handle,  TW_ISSUING_STATE.as_bytes()) {                
                Ok(Some(value)) => Some(serde_json::from_slice(&*value).unwrap()),
                Ok(None) => None,
                Err(e) => {println!("operational problem encountered: {}", e);
                None},
                _ => None
        }
    }



    fn tw_detection_add_to_index(&self, key:i64, values:Vec<(String, String)>) {
        let handle = self.provider.cf_handle(TW_DETECTION_RANGE_TX_INDEX_COLUMN).unwrap();
        let mut data = self.tw_detection_get(&key);
        for v in values.iter() {            
            data.insert(v.0.to_string(), v.1.to_string());
        }
       
        if !data.is_empty() {
            
            let _r = self.provider.put_cf(handle, key.to_be_bytes(), serde_json::to_vec(&data).unwrap());
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
                Ok(Some(value)) => serde_json::from_slice(&*value).unwrap(),
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

    fn get_last_picked_tw(&self) -> Option<TimewarpData> {
        let cached = self.LAST_PICKED_TW_cache.lock().unwrap();
        if cached.is_some() {
            return Some(cached.as_ref().unwrap().clone());
        }
        //TODO recheck persistent cache, where is this stores ?
        let cache_handle = self.provider.cf_handle(PERSISTENT_CACHE).unwrap();
        let handle = self.provider.cf_handle(FOLLOWED_TW_INDEX_COLUMN).unwrap();

        let result = match self.provider.get_cf(cache_handle, LAST_PICKED_TW_ID.as_bytes()) {
            Ok(Some(value)) => {
                let local_result = match self.provider.get_cf(handle, &*value) {
                    Ok(Some(value_two)) => {
                        let res: Option<TimewarpData> = serde_json::from_slice(&*value_two).unwrap();
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

    fn get_picked_tw_index(&self, key:i64) -> HashMap<String, String> {
        let mut borrowed_cached = self.FOLLOWED_TW_INDEX_RANGE_COLUMN_cache.lock().unwrap();     
        let cached = borrowed_cached.get_mut(&key);
        if cached.is_some() {
            let result = cached.unwrap();
            result.clone()
        }else { 
            let handle = self.provider.cf_handle(FOLLOWED_TW_INDEX_RANGE_COLUMN).unwrap();

            let result = match self.provider.get_cf(handle, key.to_be_bytes()) {                
                Ok(Some(value)) => serde_json::from_slice(&*value).unwrap(),
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

    fn add_last_picked_tw(&self, timewarps: Vec<TimewarpData>) -> Result<(), String> {
        let handle = self.provider.cf_handle(FOLLOWED_TW_INDEX_COLUMN).unwrap();
        let range_handle = self.provider.cf_handle(FOLLOWED_TW_INDEX_RANGE_COLUMN).unwrap();
        let mut batch = WriteBatch::default();
        let mut cache_updates:HashMap<i64, HashMap<String, String>> = HashMap::new();
        for timewarp in timewarps {
            let last_tw = self.get_last_picked_tw();
            if last_tw.is_some() {
                let unwrapped = last_tw.unwrap();
                if unwrapped.timestamp > timewarp.timestamp {
                    return Err("Given timewarp is older then the one provided".to_string());
                }
                if unwrapped.hash == *timewarp.target_hash() {
                   
                    let _1 = &batch.put_cf(handle, timewarp.hash.as_bytes(), serde_json::to_vec(&timewarp).unwrap());
                    let mut range_map = self.get_picked_tw_index(get_time_key(&timewarp.timestamp));
                    range_map.insert(timewarp.hash.clone(), timewarp.target_hash());
                    let _2 = &batch.put_cf(range_handle, get_time_key(&timewarp.timestamp).to_be_bytes(),serde_json::to_vec(&range_map).unwrap());
                    
                    cache_updates.insert(get_time_key(&timewarp.timestamp), range_map);
                } else {
                    return Err("Not a connecting timewarp. Source and target transaction are not matching.".to_string());
                }
            } else {
                let _l = &batch.put_cf(handle, timewarp.hash.as_bytes(), serde_json::to_vec(&timewarp).unwrap());
                let mut range_map = self.get_picked_tw_index(get_time_key(&timewarp.timestamp));
                range_map.insert(timewarp.hash.clone(), timewarp.target_hash());
                let _2 = &batch.put_cf(range_handle, get_time_key(&timewarp.timestamp).to_be_bytes(), serde_json::to_vec(&range_map).unwrap());
                    
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
            LAST_PICKED_TW_cache: Mutex::new(None),
            FOLLOWED_TW_INDEX_COLUMN_cache: Mutex::new(LruCache::new(SETTINGS.cache_settings.db_memory_cache as usize)),
        }
    }
    
}