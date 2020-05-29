use crate::indexstorage::*;
use crate::indexstorage::rocksdb_impl::*;


impl LifelinePersistence for RocksDBProvider {
   

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
            to_return.unwrap().iter().filter(|a| a.len() == 81).cloned().collect()
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


    fn prepend_to_lifeline(&self, ll_data: LifeLineData) -> Result<(), String> {
        let _r = self.get_lifeline_tx(&ll_data.timewarp_tx);
        if _r.is_none() {
            error!("Referncing transaction is not a lifeline transaction or the first lifeline data is not the referencing transaction. {}", ll_data.timewarp_tx);
            return Err(String::from("Referncing transaction is not a lifeline transaction or the first lifeline data is not the referencing transaction."));
        }
        let mut previous_ll = _r.unwrap();       
        let handle = self.provider.cf_handle(LIFELINE_INDEX_COLUMN).unwrap();
        let range_handle = self.provider.cf_handle(LIFELINE_INDEX_RANGE_COLUMN).unwrap();
        let mut batch = WriteBatch::default();
        for path in &ll_data.paths {
            previous_ll.paths.push(path.clone());
            let current_time_key = get_time_key(&path.connecting_timestamp);
            let mut current_ll_time_index =self.get_lifeline(&current_time_key);
            
            current_ll_time_index.insert(0, (path.connecting_timewarp.clone(), path.connecting_timestamp.clone()));
            
            if self.get_lifeline_tx(&path.connecting_timewarp).is_none(){
                match batch.put_cf(handle, path.connecting_timewarp.clone().as_bytes(), serde_json::to_vec(&path.connecting_empty_ll_data()).unwrap()) {
                    Err(e) => return Err(format!(" path.connecting_timewarp.clone(): {}", e.to_string())),
                    _ => {}
                };
            }
            //TODO handle errors
        
            match batch.put_cf(range_handle, current_time_key.to_be_bytes(), serde_json::to_vec(&current_ll_time_index).unwrap()){
                Err(e) => return Err(format!(" current_time_key.to_be_bytes(): {}", e.to_string())),
                _ => {}
            };
            match batch.put_cf(handle, ll_data.timewarp_tx.clone().as_bytes(), serde_json::to_vec(&ll_data).unwrap()){
                Err(e) => return Err(format!(" ll_data.timewarp_tx.clone(): {}", e.to_string())),
                _ => {}
            };
        }
        
        match batch.put_cf(handle, previous_ll.timewarp_tx.clone().as_bytes(), serde_json::to_vec(&previous_ll).unwrap()) {
            Err(e) => return Err(format!(" previous_ll.timewarp_tx.clone(): {}", e.to_string())),
            _ => {}
        };
        
        
        let _l = self.provider.write(batch);
       
        if _l.is_err() {
            return Err(format!("Something went wrong pre-pending lifeline: {}", _l.unwrap_err().to_string()))
        }
        //info!("Prepended lifeline tx {}", &ll_data.timewarp_tx);
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
    /// The function should only be used to add live data.
    fn add_to_lifeline(&self, lifeline_data: Vec<LifeLineData>) -> Result<(), String> {
        let handle = self.provider.cf_handle(LIFELINE_INDEX_COLUMN).unwrap();
        let range_handle = self.provider.cf_handle(LIFELINE_INDEX_RANGE_COLUMN).unwrap();
        
        let mut batch = WriteBatch::default();
        let mut unpinned: Vec<String> = vec!();
        let mut ll_index_map: HashMap<i64, Vec<(String, i64)>> = HashMap::new();

        let mut ll_graph_events: Vec<GraphEntryEvent> = vec!();
        
        let mut last_lifeline = if !lifeline_data.first().unwrap().paths.is_empty() {                
            self.get_lifeline_tx(&lifeline_data.first().unwrap().paths[0].connecting_timewarp.to_string())
        }else {
            None
        };
        let mut oldest_tx_ts_cnt: (String, i64, i64) = (String::new(), 0,0);
       
        for lifeline in lifeline_data {
            let mut ll_clone = lifeline.clone();
            let timekey = get_time_key(&ll_clone.timestamp);
            if !ll_index_map.contains_key(&timekey) {
                ll_index_map.insert(timekey.clone(), self.get_lifeline(&timekey).clone());
            }
            
            if last_lifeline.is_some() {
                let mut unwrapped_last_ll = last_lifeline.unwrap();              
                if &unwrapped_last_ll.timestamp > &ll_clone.timestamp {
                    return Err("Given timewarp is older then the one provided".to_string());
                }
                if unwrapped_last_ll.paths.is_empty() {
                     oldest_tx_ts_cnt = (unwrapped_last_ll.timewarp_tx.clone(), 
                     unwrapped_last_ll.timestamp.clone(), 0);                    
                }else {
                    oldest_tx_ts_cnt = (unwrapped_last_ll.paths[0].oldest_tx.clone(), 
                        unwrapped_last_ll.paths[0].oldest_timestamp.clone(), 
                        unwrapped_last_ll.paths[0].transactions_till_oldest.clone() +
                            unwrapped_last_ll.paths[0].connecting_pathway.clone().size as i64
                        );
                    unwrapped_last_ll = if unwrapped_last_ll.timestamp - unwrapped_last_ll.paths[0].oldest_timestamp.clone() > SETTINGS.lifeline_settings.subgraph_section_split_in_seconds {
                                       
                    // TODO Call lifeline adjustment, create new event
                    ll_graph_events.push(GraphEntryEvent {
                        between_start: None,
                        between_end: Some(oldest_tx_ts_cnt.0.clone()),
                        between_tx_split: None,
                        target_tx_id: oldest_tx_ts_cnt.0.clone(),
                        tx_distance_count: oldest_tx_ts_cnt.2, //TODO check if this is correct
                        index: self.new_index(),
                        target_timestamp: oldest_tx_ts_cnt.1.clone(),
                        txid: unwrapped_last_ll.timewarp_tx.clone(),
                        timestamp: unwrapped_last_ll.timestamp

                    });
                    oldest_tx_ts_cnt = (unwrapped_last_ll.timewarp_tx.clone(), unwrapped_last_ll.timestamp.clone(), 0);
                   
                    LifeLineData{        
                        paths: vec!(LifeLinePathData {
                            oldest_tx: oldest_tx_ts_cnt.0.clone(),
                            oldest_timestamp: oldest_tx_ts_cnt.1,
                            transactions_till_oldest: oldest_tx_ts_cnt.2,
                            ..unwrapped_last_ll.paths[0].clone()
                        }),
                        ..unwrapped_last_ll
                    }
                }else { unwrapped_last_ll };
                }
                ll_clone.paths[0].oldest_tx = oldest_tx_ts_cnt.0.clone();
                ll_clone.paths[0].oldest_timestamp = oldest_tx_ts_cnt.1.clone();
                ll_clone.paths[0].transactions_till_oldest = oldest_tx_ts_cnt.2.clone();
                if &unwrapped_last_ll.timewarp_tx == &ll_clone.paths[0].connecting_timewarp {
                    
                    //for time_key in get_time_key_range(&ll_clone.timestamp, &ll_clone.timestamp ) {
                        unpinned.push(ll_clone.timewarp_tx.clone());
                        let _1 = &batch.put_cf(handle, &ll_clone.timewarp_tx.as_bytes(), serde_json::to_vec(&ll_clone).unwrap());
                        let mut range_map = ll_index_map.get_mut(&timekey).unwrap();
                        range_map.push((ll_clone.timewarp_tx.clone(), ll_clone.timestamp.clone()));
                                             
                    //}                   
                } else {
                    return Err("Not a connecting lifeline. Connecting_timewarp does not match latest transaction ID.".to_string());
                }
            } else {
                info!("Life line initialisation");
                // TODO create event 0;
                unpinned.push(lifeline.timewarp_tx.clone());
                let _l = &batch.put_cf(handle, lifeline.timewarp_tx.clone().as_bytes(), serde_json::to_vec(&lifeline.clone()).unwrap());                
                let mut range_map = self.get_lifeline(&get_time_key(&lifeline.timestamp));
                range_map.push((lifeline.timewarp_tx.clone(), lifeline.timestamp.clone()));
                let _2 = &batch.put_cf(range_handle, get_time_key(&lifeline.timestamp).to_be_bytes(),serde_json::to_vec(&range_map).unwrap());    
                ll_graph_events.push(GraphEntryEvent {
                    between_end: None,
                    between_start: None,
                    between_tx_split: None,
                    index: 0,
                    target_tx_id: "".to_string(),
                    txid: lifeline.timewarp_tx.clone(),
                    timestamp: lifeline.timestamp.clone(),
                    target_timestamp: 0,
                    tx_distance_count: 0,

                });
                //cache_updates.insert(get_time_key(&timewarp.timestamp), range_map);
            }
            last_lifeline = Some(ll_clone.clone());
            
        }
        for (timekey, lst) in &ll_index_map {
            let _2 = &batch.put_cf(range_handle, timekey.to_be_bytes(),serde_json::to_vec(&lst).unwrap());    
        }
        let _l = self.provider.write(batch);
       
        if _l.is_ok() {
            if last_lifeline.is_some() {
                let last_lifeline_handle = self.provider.cf_handle(PERSISTENT_CACHE).unwrap();
                let _l = self.provider.put_cf(last_lifeline_handle, P_CACHE_LAST_LIFELIFE.as_bytes(), serde_json::to_vec(&last_lifeline.unwrap()).unwrap());
                let mut last_unpinned =  self.get_unpinned_lifeline();
                last_unpinned.append(&mut unpinned);
                
                let _l2 = self.provider.put_cf(last_lifeline_handle, P_CACHE_UNPINNED_LIFELIFE.as_bytes(), serde_json::to_vec(&last_unpinned).unwrap());

                for sll in ll_graph_events {
                    let _pe = self.process_event(sll);
                    if _pe.is_err() {
                        return Err("Something went wrong processings the event".to_string());
                    }
                }
                if _l.is_err() {
                    return Err("Something went wrong inserting last_lifeline".to_string());
                }
            }
           
        }else{
            return Err("Something went wrong writing batches".to_string());
        }
      
        Ok(())
    }

}