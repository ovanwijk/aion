
// use std::collections::LinkedList;
// use serde_json::json;
// use serde::{Serialize, Deserialize};
use std::marker::{Send, Sync};
// use riker::actors::*;
// use riker::actors::Context;
//use std::convert::TryInto;
use std::sync::Mutex;
//use crate::timewarping::Protocol;
use crate::timewarping::timewarpselecting::TimewarpSelectionState;
use crate::indexstorage::*;
use crate::SETTINGS;
use ::rocksdb::{DB, Options, WriteBatch};
use std::{
    collections::{HashMap}};
use lru_cache::LruCache;

const TW_DETECTION_RANGE_TX_INDEX_COLUMN:&str = "TW_DETECTION_RANGE_TX_INDEX_COLUMN";
const TW_DETECTION_RANGE_GROUP_INDEX_COLUMN:&str = "TW_DETECTION_RANGE_GROUP_INDEX_COLUMN";
const FOLLOWED_TW_INDEX_COLUMN:&str = "FOLLOWED_TW_INDEX_COLUMN";
const FOLLOWED_TW_INDEX_RANGE_COLUMN:&str = "FOLLOWED_TW_INDEX_RANGE_COLUMN";

const LIFELINE_INDEX_COLUMN:&str = "LIFELINE_INDEX_COLUMN";
const LIFELINE_INDEX_RANGE_COLUMN:&str = "LIFELINE_INDEX_RANGE_COLUMN";

const PULLJOB_ID_COLUMN:&str = "PULLJOB_ID_COLUMN";
const PINNED_TX_COUNTER:&str = "PINNED_TX_COUNTER";
const PATHWAY_DESCRIPTORS:&str = "PATHWAY_DESCRIPTORS";
const FLEXIBLE_ZERO:&str = "FLEXIBLE_ZERO";
const PERSISTENT_CACHE:&str = "PERSISTENT_CACHE";



#[derive(Debug)]
pub struct RocksDBProvider {
    provider: DB,
    TW_DETECTION_RANGE_TX_INDEX_COLUMN_cache: Mutex<LruCache<i64, HashMap<String, String>>>,
    FOLLOWED_TW_INDEX_RANGE_COLUMN_cache:  Mutex<LruCache<i64, HashMap<String, String>>>,
    FOLLOWED_TW_INDEX_COLUMN_cache:  Mutex<LruCache<String, TimewarpData>>,
    LAST_PICKED_TW_cache: Mutex<Option<TimewarpSelectionState>>
}


impl Persistence for RocksDBProvider {

    fn clean_db(&self, timestamp:i64) {
        //TODO implement
    }

    fn store_pin_descriptor(&self, pin_descriptor:PinDescriptor) -> Result<(), String> {
        let handle = self.provider.cf_handle(PATHWAY_DESCRIPTORS).unwrap();
        let _r = self.provider.put_cf(handle, pin_descriptor.id(), serde_json::to_vec(&pin_descriptor).unwrap());
        if _r.is_err() {
            Err(_r.unwrap_err().to_string())
        }else{
            Ok(())
        }
    }
    fn get_pin_descriptor(&self, id:Vec<u8>) -> Option<PinDescriptor> {
        let handle = self.provider.cf_handle(PATHWAY_DESCRIPTORS).unwrap();
        
        match self.provider.get_cf(handle,  id) {                
                Ok(Some(value)) => Some(serde_json::from_slice(&*value).expect("get_pin_descriptor")),
                Ok(None) => None,
                Err(e) => {println!("operational problem encountered: {}", e);
                None}
        }
    }

    fn tw_detection_add_decision_data(&self, tw:  crate::timewarping::Timewarp) -> TimewarpData { 
        let previous = self.tw_detection_get_decision_data(tw.target_hash().to_string());
        let tostore = if previous.is_some() {
            let advanced = previous.unwrap().advance(&tw);
            info!("Advancing ID:{} : Index: {} Avg distance {}", &advanced.timewarpid, &advanced.index_since_id, &advanced.avg_distance );
            advanced
        }else{
            info!("New timewarp: {}", tw.source_hash.clone());
            TimewarpData {
                timewarpid: tw.source_hash.clone()[0..9].to_string(),
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
                None}
            };

            if result.is_some() {
                info!("Obtained from storage");
                borrowed_cached.insert(key.clone(), result.clone().unwrap().clone());
            }
            result
        }
    }

    fn get_lifeline(&self, key: &i64) -> Vec<(String, i64)> {
        let handle = self.provider.cf_handle(LIFELINE_INDEX_RANGE_COLUMN).unwrap();   
        let to_return:Option<Vec<(String, i64)>> = match self.provider.get_cf(handle,  key.to_be_bytes()) {                
                Ok(Some(value)) => Some(serde_json::from_slice(&*value).expect("get_lifeline_tx")),
                Ok(None) => None,
                Err(e) => {println!("operational problem encountered: {}", e);
                None}
        };
        if to_return.is_some() {
            let mut sorted = to_return.unwrap();
            sorted.sort_unstable_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            sorted
        }else{
            vec!()
        }
    }

    fn get_lifeline_ts(&self, timestamp:&i64) -> Option<LifeLineData> {

        let timewindow_key = get_time_key(&timestamp);
        //let mut it = self.get_lifeline(timewindow_key);
        let mut counter = 0;
        //let mut found = false;
        let mut found: Option<LifeLineData> = None;
        while counter < 15 && found.is_none() {
            let it = self.get_lifeline(&(timewindow_key + (SETTINGS.timewarp_index_settings.time_index_clustering_in_seconds * counter)));
            for ll_tx in it  {                
                if ll_tx.1 > *timestamp {                  
                    found = Some(self.get_lifeline_tx(&ll_tx.0).unwrap());
                    break;
                }
            }

            counter += 1;
        }
       // let it = self.get_lifeline(get_time_key(&timestamp));
        if found.is_none() {
            warn!("No lifeline found for {}", timestamp);
        }
      
        found
    }

    fn get_unpinned_lifeline(&self) -> Vec<String> {
        let handle = self.provider.cf_handle(PERSISTENT_CACHE).unwrap();        
        let to_return:Option<Vec<String>> = match self.provider.get_cf(handle,  P_CACHE_UNPINNED_LIFELIFE.as_bytes()) {                
            Ok(Some(value)) => Some(serde_json::from_slice(&*value).expect("get_unpinned_lifeline")),
            Ok(None) => None,
            Err(e) => {println!("operational problem encountered: {}", e);
            None}
        };
        if to_return.is_some() {
            to_return.unwrap()
        }else{
            vec!()
        }
    }


    fn get_lifeline_tx(&self, key: &String) -> Option<LifeLineData> {
        let handle = self.provider.cf_handle(LIFELINE_INDEX_COLUMN).unwrap();
        
        match self.provider.get_cf(handle,  key.as_bytes()) {                
                Ok(Some(value)) => Some(serde_json::from_slice(&*value).expect("get_lifeline_tx")),
                Ok(None) => None,
                Err(e) => {println!("operational problem encountered: {}", e);
                None}
        }
    }

    fn get_last_lifeline(&self) -> Option<LifeLineData> {
        let handle = self.provider.cf_handle(PERSISTENT_CACHE).unwrap();
        
        match self.provider.get_cf(handle,  P_CACHE_LAST_LIFELIFE.as_bytes()) {                
                Ok(Some(value)) => Some(serde_json::from_slice(&*value).expect("get_last_lifeline")),
                Ok(None) => None,
                Err(e) => {println!("operational problem encountered: {}", e);
                None}
        }
    }

    fn update_lifeline_tx(&self, data:LifeLineData) -> Result<(), String> {
        let handle = self.provider.cf_handle(LIFELINE_INDEX_COLUMN).unwrap();
        let _r = self.provider.put_cf(handle, data.timewarp_tx.as_bytes(), serde_json::to_vec(&data).unwrap());
        if _r.is_err() {
            return Err(format!("Error occured {:?}", _r.unwrap_err()));
        }
        Ok(())
    }

    fn set_unpinned_lifeline(&self, data:Vec<String>) -> Result<(), String> {
       
        self.set_generic_cache(P_CACHE_UNPINNED_LIFELIFE, serde_json::to_vec(&data).unwrap())
        
    }
   
    fn save_timewarp_state(&self, state: TimewarpIssuingState) {
        self.set_generic_cache(TW_ISSUING_STATE, serde_json::to_vec(&state).unwrap());
        // let handle = self.provider.cf_handle(PERSISTENT_CACHE).unwrap();
        // let _r = self.provider.put_cf(handle, TW_ISSUING_STATE.as_bytes(), serde_json::to_vec(&state).unwrap()).unwrap();
    }
    fn get_timewarp_state(&self) -> Option<TimewarpIssuingState> {
        let handle = self.provider.cf_handle(PERSISTENT_CACHE).unwrap();
        
        match self.provider.get_cf(handle,  TW_ISSUING_STATE.as_bytes()) {                
                Ok(Some(value)) => Some(serde_json::from_slice(&*value).expect("get_timewarp_state")),
                Ok(None) => None,
                Err(e) => {println!("operational problem encountered: {}", e);
                None}
        }
    }
    fn set_generic_cache(&self, key:&str, value:Vec<u8>) -> Result<(), String> {
        let handle = self.provider.cf_handle(PERSISTENT_CACHE).unwrap();
        let _r = self.provider.put_cf(handle, key.as_bytes(), value);
        if _r.is_ok(){
            Ok(())
        }else{
            Err(format!("{}", _r.unwrap_err()))
        }
        
    }
    fn get_generic_cache(&self, key:&str) -> Option<Vec<u8>> {
        let handle = self.provider.cf_handle(PERSISTENT_CACHE).unwrap();        
        match self.provider.get_cf(handle,  key.as_bytes()) {                
            Ok(Some(value)) => Some(value.to_vec()),
            Ok(None) => None,
            Err(e) => {println!("operational problem encountered: {}", e);
            None}
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
                HashMap::new()}
            };
            if result.len() > 0 {
                borrowed_cached.insert(key.clone(), result.clone());
            }
            result

        }
       
    }
    fn tw_detection_get_all(&self, keys:Vec<&i64>) ->HashMap<String, String> {HashMap::new()}

    fn set_last_picked_tw(&self,  state: TimewarpSelectionState) -> Result<(), String> {

        self.set_generic_cache(crate::indexstorage::TIME_SINCE_LL_CONNECT, crate::now().to_be_bytes().to_vec());
        self.set_generic_cache(LAST_PICKED_TW_ID,serde_json::to_vec(&state).unwrap())
    }


    fn get_last_picked_tw(&self) -> Option<TimewarpSelectionState> {
        // let cached = self.LAST_PICKED_TW_cache.lock().unwrap();
        // if cached.is_some() {
        //     return Some(cached.as_ref().unwrap().clone());
        // }
        //TODO recheck persistent cache, where is this stores ?
        let cache_handle = self.provider.cf_handle(PERSISTENT_CACHE).unwrap();
        let handle = self.provider.cf_handle(FOLLOWED_TW_INDEX_COLUMN).unwrap();

        let result = match self.provider.get_cf(cache_handle, LAST_PICKED_TW_ID.as_bytes()) {
            Ok(Some(value)) => {
                let res: Option<TimewarpSelectionState> = serde_json::from_slice(&*value).unwrap();
                res
            },
            Ok(None) => None,
            Err(e) => {println!("operational problem encountered: {}", e);
            None}
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
                HashMap::new()}
            };
            if result.len() > 0 {
                borrowed_cached.insert(key.clone(), result.clone());
            }
            result

        }
    }

    //TODO implement
    fn prepend_to_lifeline(&self, ll_data: LifeLineData) -> Result<(), String> {
        let _r = self.get_lifeline_tx(&ll_data.timewarp_tx);
        if _r.is_none() && ll_data.connecting_timewarp.is_none() {
            return Err(String::from("Referncing transaction is not a lifeline transaction or the first lifeline data is not the referencing transaction."));
        }
        let mut previous_ll = _r.unwrap();
        let handle = self.provider.cf_handle(LIFELINE_INDEX_COLUMN).unwrap();
        let range_handle = self.provider.cf_handle(LIFELINE_INDEX_RANGE_COLUMN).unwrap();
        let mut batch = WriteBatch::default();
        let mut current_time_key = get_time_key(&ll_data.timestamp);
        let mut current_ll_time_index =self.get_lifeline(&current_time_key);

        current_ll_time_index.insert(0, (ll_data.connecting_timewarp.clone().unwrap(), ll_data.connecting_timestamp.clone().unwrap()));
        
        //TODO handle errors
        batch.put(ll_data.connecting_timewarp.clone().unwrap().as_bytes(), serde_json::to_vec(&ll_data.connecting_empty_ll_data()).unwrap());
        batch.put(current_time_key.to_be_bytes(), serde_json::to_vec(&current_ll_time_index).unwrap());
        batch.put(ll_data.timewarp_tx.clone().as_bytes(), serde_json::to_vec(&ll_data).unwrap());
        
        let _l = self.provider.write(batch);
       
        if _l.is_err() {
            return Err(format!("Something went wrong pre-pending lifeline: {}", _l.unwrap_err().to_string()))
        }
        
        Ok(())
    }
    //TODO implement
    fn find_tx_distance_between_lifelines(&self, start: &String, end: &String) -> i64 {
        let start_tx = self.get_lifeline_tx(start);
        let end_tx = self.get_lifeline_tx(end);
        match (start_tx, end_tx) {
            (None, _) => -1,
            (_, None) => -1,
            (Some(st),Some(en)) => {                
                0
            }
        }
    }

    
    /// This function assumes the first data-point to be the 'oldest'
    /// Uses the 
    fn add_to_lifeline(&self, lifeline_data: Vec<LifeLineData>) -> Result<(), String> {
        let handle = self.provider.cf_handle(LIFELINE_INDEX_COLUMN).unwrap();
        let range_handle = self.provider.cf_handle(LIFELINE_INDEX_RANGE_COLUMN).unwrap();
        
        let mut batch = WriteBatch::default();
        let mut unpinned: Vec<String> = vec!();

        let mut ll_graph_events: Vec<GraphEntryEvent> = vec!();
        
        let mut last_lifeline = if lifeline_data.first().unwrap().connecting_timewarp.is_some() {                
            self.get_lifeline_tx(&lifeline_data.first().unwrap().connecting_timewarp.as_ref().unwrap().to_string())
        }else {
            None
        };
        let mut oldest_tx_ts_cnt: (String, i64, i64) = (String::from(""), 0, 0);
       
        for lifeline in lifeline_data {            
            if last_lifeline.is_some() {
                let mut unwrapped_last_ll = last_lifeline.unwrap();
                if &unwrapped_last_ll.timestamp > &lifeline.timestamp {
                    return Err("Given timewarp is older then the one provided".to_string());
                }
               
                oldest_tx_ts_cnt = (unwrapped_last_ll.oldest_tx.clone(), 
                    unwrapped_last_ll.oldest_timestamp.clone(), 
                    unwrapped_last_ll.transactions_till_oldest.clone() + (if unwrapped_last_ll.connecting_pathway.is_none() { 1 }else {
                        unwrapped_last_ll.connecting_pathway.clone().unwrap().size as i64
                    } ));
                unwrapped_last_ll.transactions_till_oldest = oldest_tx_ts_cnt.2;
                unwrapped_last_ll = if unwrapped_last_ll.timestamp - oldest_tx_ts_cnt.1 > SETTINGS.lifeline_settings.subgraph_section_split_in_seconds {
                    // TODO Call lifeline adjustment, create new event
                    oldest_tx_ts_cnt = (unwrapped_last_ll.timewarp_tx.clone(), unwrapped_last_ll.timestamp.clone(), 0);
                    LifeLineData{                       
                        oldest_tx: oldest_tx_ts_cnt.0,
                        oldest_timestamp: oldest_tx_ts_cnt.1,
                        ..unwrapped_last_ll
                    }
                }else { unwrapped_last_ll };
                if &unwrapped_last_ll.timewarp_tx == lifeline.connecting_timewarp.as_ref().expect("Connecting lifeline data") {
                    
                    for time_key in get_time_key_range(&unwrapped_last_ll.timestamp, &lifeline.timestamp ) {
                        unpinned.push(lifeline.timewarp_tx.clone());
                        let _1 = &batch.put_cf(handle, &lifeline.timewarp_tx.as_bytes(), serde_json::to_vec(&lifeline).unwrap());
                        let mut range_map = self.get_lifeline(&time_key);
                        range_map.push((lifeline.timewarp_tx.clone(), lifeline.timestamp.clone()));
                        let _2 = &batch.put_cf(range_handle, time_key.to_be_bytes(),serde_json::to_vec(&range_map).unwrap());                        
                    }                   
                } else {
                    return Err("Not a connecting lifeline. Connecting_timewarp does not match latest transaction ID.".to_string());
                }
            } else {
                info!("Life line initialisation");
                // TODO create event 0;
                unpinned.push(lifeline.timewarp_tx.clone());
                let _l = &batch.put_cf(handle, lifeline.timewarp_tx.as_bytes(), serde_json::to_vec(&lifeline).unwrap());                
                let mut range_map = self.get_lifeline(&get_time_key(&lifeline.timestamp));
                range_map.push((lifeline.timewarp_tx.clone(), lifeline.timestamp.clone()));
                let _2 = &batch.put_cf(range_handle, get_time_key(&lifeline.timestamp).to_be_bytes(),serde_json::to_vec(&range_map).unwrap());    
                    
                //cache_updates.insert(get_time_key(&timewarp.timestamp), range_map);
            }
            last_lifeline = Some(lifeline.clone());
            
        }
        let _l = self.provider.write(batch);
       
        if _l.is_ok() {
            if last_lifeline.is_some() {
                let last_lifeline_handle = self.provider.cf_handle(PERSISTENT_CACHE).unwrap();
                let _l = self.provider.put_cf(last_lifeline_handle, P_CACHE_LAST_LIFELIFE.as_bytes(), serde_json::to_vec(&last_lifeline.unwrap()).unwrap());
                let mut last_unpinned =  self.get_unpinned_lifeline();
                last_unpinned.append(&mut unpinned);
                
                let _l2 = self.provider.put_cf(last_lifeline_handle, P_CACHE_UNPINNED_LIFELIFE.as_bytes(), serde_json::to_vec(&last_unpinned).unwrap());
                if _l.is_err() {
                    return Err("Something went wrong inserting last_lifeline".to_string());
                }
            }
           
        }else{
            return Err("Something went wrong writing batches".to_string());
        }
      
        Ok(())
    }

    fn add_pull_job(&self, job:&PullJob) {
        let handle = self.provider.cf_handle(PULLJOB_ID_COLUMN).unwrap();
        let index_handle = self.provider.cf_handle(PERSISTENT_CACHE).unwrap();
        
        let mut result:Vec<String> = match self.provider.get_cf(index_handle, P_CACHE_PULLJOB.as_bytes()) {                
            Ok(Some(value)) => serde_json::from_slice(&*value).unwrap(),
            Ok(None) => vec!(),
            Err(e) => {println!("operational problem encountered: {}", e);
           vec!()}
        };
        result.push(job.id.clone());
        let _r = self.provider.put_cf(handle, job.id.as_bytes(), serde_json::to_vec(&job).unwrap());
        let _r = self.provider.put_cf(index_handle, P_CACHE_PULLJOB.as_bytes(), serde_json::to_vec(&result).unwrap());
    }

     fn update_pull_job(&self, job:&PullJob) {
        let handle = self.provider.cf_handle(PULLJOB_ID_COLUMN).unwrap();
        let _r = self.provider.put_cf(handle, job.id.as_bytes(), serde_json::to_vec(&job).unwrap());
        if job.status.as_str() == PIN_STATUS_PIN_ERROR || job.status.as_str() == PIN_STATUS_NODE_ERROR {

            //Add to faulty list
            let mut failed_pull_jobs:Vec<String> = match self.get_generic_cache(P_CACHE_FAULTY_PULLJOB) {                
                Some(value) => serde_json::from_slice(&*value).unwrap(),
                None => vec!()
               };
                      
            
            failed_pull_jobs.push(job.id.to_string());
            self.set_generic_cache(P_CACHE_FAULTY_PULLJOB, serde_json::to_vec(&failed_pull_jobs).unwrap());  


            //Remove from standard list
           
            let mut pull_jobs:Vec<String> = match self.get_generic_cache(P_CACHE_PULLJOB) {                
                Some(value) => serde_json::from_slice(&*value).unwrap(),
                None => vec!()
               };
                    
            
            pull_jobs.retain(|a| a != &job.id);
            self.set_generic_cache(P_CACHE_PULLJOB, serde_json::to_vec(&pull_jobs).unwrap());             
                      
        }
        
     }

     fn list_pull_jobs(&self) -> Vec<String> {
        match self.get_generic_cache(P_CACHE_PULLJOB) {                
            Some(value) => serde_json::from_slice(&*value).unwrap(),
            None => vec!()
           }
     }
    fn list_faulty_pull_jobs(&self) -> Vec<String> {
        match self.get_generic_cache(P_CACHE_FAULTY_PULLJOB) {                
            Some(value) => serde_json::from_slice(&*value).unwrap(),
            None => vec!()
           }
    }

     fn get_pull_job(&self, id: &String) -> Option<PullJob> {
        let handle = self.provider.cf_handle(PULLJOB_ID_COLUMN).unwrap();
        return  match self.provider.get_cf(handle, id.as_bytes()) {                
            Ok(Some(value)) => Some(serde_json::from_slice(&*value).unwrap()),
            Ok(None) => None,
            Err(e) => {println!("operational problem encountered: {}", e);
            None}
        };
     }
     fn pop_pull_job(&self, id: String) {
        let handle = self.provider.cf_handle(PULLJOB_ID_COLUMN).unwrap();
        let index_handle = self.provider.cf_handle(PERSISTENT_CACHE).unwrap();
        let mut pull_jobs:Vec<String> = match self.provider.get_cf(index_handle, P_CACHE_PULLJOB.as_bytes()) {                
            Ok(Some(value)) => serde_json::from_slice(&*value).unwrap(),
            Ok(None) => vec!(),
            Err(e) => {println!("operational problem encountered: {}", e);
           vec!()}
        };
        let _r = self.provider.delete_cf(handle, id.as_bytes());
        if !_r.is_err() {
            pull_jobs.retain(|a| a != &id);
            let _r = self.provider.put_cf(index_handle, P_CACHE_PULLJOB.as_bytes(), serde_json::to_vec(&pull_jobs).unwrap());          
        }
        

     }
     fn next_pull_job(&self, offset:&usize) -> Option<PullJob> {
        
        //return None;
        let handle = self.provider.cf_handle(PULLJOB_ID_COLUMN).unwrap();
        let index_handle = self.provider.cf_handle(PERSISTENT_CACHE).unwrap();
        let pull_jobs:Vec<String> = match self.provider.get_cf(index_handle, P_CACHE_PULLJOB.as_bytes()) {                
            Ok(Some(value)) => serde_json::from_slice(&*value).unwrap(),
            Ok(None) => vec!(),
            Err(e) => {println!("operational problem encountered: {}", e);
           vec!()}
        };
        let mut picked:Option<String> = if pull_jobs.is_empty() { None } else { Some(pull_jobs.get(std::cmp::min(pull_jobs.len()-1, 0 + offset)).unwrap().clone()) };
        if picked.is_none(){
            None
        } else {
            let mut to_return: Option<PullJob> = None;
            loop{
                to_return = match self.provider.get_cf(handle, picked.unwrap().as_bytes()) {
                    Ok(Some(value)) => Some(serde_json::from_slice(&*value).unwrap()),
                    Ok(None) => return None,
                    Err(e) => {println!("operational problem encountered: {}", e); return None}
                };
                let unwarpped = to_return.unwrap();
                if self.provider.get_cf(handle, unwarpped.dependant.as_bytes()).unwrap().is_none() {
                    return Some(unwarpped);
                }else {
                    let borrowed = unwarpped.dependant.clone();
                    picked = Some(borrowed);
                }
                
            };
        }
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
            LIFELINE_INDEX_COLUMN,
            LIFELINE_INDEX_RANGE_COLUMN,
            PINNED_TX_COUNTER,
            PATHWAY_DESCRIPTORS,
            FLEXIBLE_ZERO,
            PULLJOB_ID_COLUMN,
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