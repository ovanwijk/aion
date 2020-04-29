use crate::indexstorage::*;
use crate::indexstorage::rocksdb_impl::*;


impl GenericCachePersistence for RocksDBProvider {
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

}