use crate::indexstorage::*;
use crate::indexstorage::rocksdb_impl::*;


impl TimewarpIssueingPersistence for RocksDBProvider {
  
   
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

}