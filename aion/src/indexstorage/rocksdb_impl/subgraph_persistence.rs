use crate::indexstorage::*;
use crate::indexstorage::rocksdb_impl::*;


impl SubgraphPersistence for RocksDBProvider {
    
    fn new_index(&self) -> i64 {
        self.LIFELINE_SUBGRAPH.lock().unwrap().current_index + 1
    }

    fn clone_state(&self) -> LifelineSubGraph {
        self.LIFELINE_SUBGRAPH.lock().unwrap().clone()
    }

    fn store_state(&self) -> Result<(), String> {
        let _r = self.set_generic_cache(crate::indexstorage::P_CACHE_LIFELINE_SUBGRAPH, serde_json::to_vec(&self.LIFELINE_SUBGRAPH.lock().unwrap().clone()).unwrap());
        _r
    }
    fn process_event(&self, event: GraphEntryEvent) -> Result<(), String> {
        let mut subgraph = self.LIFELINE_SUBGRAPH.lock().unwrap();
        if subgraph.vertices.is_empty() {
            if event.index != 0 {
                return Err("First event index should be 0".to_string());
            }
            subgraph.top_level = event.txid.clone();
            subgraph.vertices.insert(event.txid.clone(), GraphVertex {
                reference_me: HashMap::new(),
                i_reference: HashMap::new(),
                timestamp: event.timestamp
            });
            drop(subgraph);
            let _r = self.store_state();
            info!("Subgraph initiated.");
            return Ok(());
        }

        let to_return = match (&event.between_start, &event.between_end) {
            //TODO after a split edge update lifeline transactions to reference new split
            (Some(_), Some(_)) => {info!("Splitting subgraph");LifelineSubGraph::split_edge(event.clone(), &mut subgraph); Ok(())},
            (Some(_), None) => { info!("Prepend subgraph");LifelineSubGraph::prepend(event.clone(), &mut subgraph);Ok(())}, //Prepend
            (None, Some(_)) => {info!("Append subgraph"); LifelineSubGraph::append(event.clone(), &mut subgraph);Ok(())}, // Append
            (None, None) => Err("Must have start or end".to_string())
        };
        subgraph.current_index += 1;
        //manually drop the lock
        drop(subgraph);

        let _r = self.store_state();


        to_return
    }


    fn load_subgraph(&mut self) -> Result<(), String>  {
        let _r = self.get_generic_cache(crate::indexstorage::P_CACHE_LIFELINE_SUBGRAPH);
        if _r.is_none() {
            return Ok(());
        }
        let result:LifelineSubGraph = serde_json::from_slice(&_r.unwrap()).expect("Lifeline data to be correct");
        let mut a = self.LIFELINE_SUBGRAPH.lock().expect("Could not lock");
        std::mem::replace(&mut *a, result);
        return Ok(());


    }


}