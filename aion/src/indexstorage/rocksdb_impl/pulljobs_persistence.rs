use crate::indexstorage::*;
use crate::indexstorage::rocksdb_impl::*;


impl PullJobsPersistence for RocksDBProvider {
   
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
        if  PIN_STATUS_ERROR.contains(&job.status.as_str()) {

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