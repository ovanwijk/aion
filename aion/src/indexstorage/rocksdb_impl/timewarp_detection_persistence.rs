

use crate::indexstorage::*;
use crate::indexstorage::rocksdb_impl::*;


impl TimewarpDetectionPersistence for RocksDBProvider {

    fn tw_detection_add_decision_data(&self, tw: crate::timewarping::Timewarp) -> TimewarpData { 
        let previous = self.tw_detection_get_decision_data(tw.target_hash().to_string());
        let tostore = if previous.is_some() {
            let advanced = previous.unwrap().advance(&tw);
            info!("Advancing ID:{} : Index: {} Avg distance {}", &advanced.timewarpid, &advanced.index_since_id, &advanced.avg_distance );
            advanced
        }else{
            info!("New timewarp: {}", tw.source_hash.clone());
            TimewarpData {
                timewarpid: tw.source_tag.clone()[16..24].to_string(),
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
}