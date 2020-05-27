use crate::indexstorage::*;
use crate::indexstorage::rocksdb_impl::*;
use crate::pathway::*;
use crate::petgraph::visit::*;
use crate::petgraph::graph::*;
use crate::petgraph::prelude::*;

impl RocksDBProvider { 


    fn get_pull_job_cache(&self, start: String, end: String) -> Option<PinDescriptor> {
        let handle = self.provider.cf_handle(PULLJOB_CACHE_COLUMN).unwrap();
        //let r = self.provider.get_cf(handle, );
        match self.provider.get_cf(handle,  format!("{}{}", start, end).as_bytes()) {                
            Ok(Some(value)) => Some(serde_json::from_slice(&*value).expect("get_pull_job_cache")),
            Ok(None) => None,
            Err(e) => {println!("operational problem encountered: {}", e);
            None}
        }
    }

    fn retarget_paths(&self, event: &GraphEntryEvent) -> Result<(), String> {
        match (&event.between_start, &event.between_end) {
            //TODO after a split edge update lifeline transactions to reference new split
            
            (Some(_), None) => { return Ok(()) }, //Prepend
            (None, Some(_)) => { return Ok(())}, // Append
            (None, None) => return Ok(()),
            (Some(bs), Some(es)) => {/* Just continue the function */},
        };
        let between_start = event.between_start.clone().unwrap();
        let old_end = event.between_end.clone().unwrap();
        let new_end = event.txid.clone();

        let between_start_ll = self.get_lifeline_tx(&between_start).unwrap();
        let split_tx_ll = self.get_lifeline_tx(&event.txid).unwrap();

        let between_start_path = between_start_ll.walk_towards(&old_end).expect("It to be correct");
        let split_tx_path = split_tx_ll.walk_towards(&old_end).expect("It to be correct");
        let tx_difference = between_start_path.transactions_till_oldest - split_tx_path.transactions_till_oldest;
        let mut total_edits = 0;
        let mut current_tx = between_start.clone();
        while current_tx != new_end {
            let mut tx = self.get_lifeline_tx(&current_tx).expect("The lifeline transaction to exists");
            let mut tx_path = tx.walk_towards(&old_end).expect("The lifeline transaction to exists");
            let mut filtered_path:Vec<LifeLinePathData> = tx.paths.iter().filter(|x| &x.oldest_tx != &old_end).cloned().collect();
            current_tx = tx_path.connecting_timewarp.clone();
            tx_path.oldest_tx = new_end.clone();
            tx_path.transactions_till_oldest -= tx_difference;
            filtered_path.push(tx_path);
            tx.paths = filtered_path;            
            match self.update_lifeline_tx(tx) {
                Err(e) => return Err(e.to_string()),
                _ => {total_edits += 1;}
            };
        }
        info!("Altered {} transactions.", total_edits);
        Ok(())
    }

    fn set_pull_job_cache(&self, event: &GraphEntryEvent) -> Result<(), String> {

        let entries: Vec<(String, String)> = match (&event.between_start, &event.between_end) {
            //TODO after a split edge update lifeline transactions to reference new split
            (Some(bs), Some(es)) => {
                vec!((bs.clone(), event.txid.clone()),
                    (event.txid.clone(), es.clone()),
                    (event.txid.clone(), event.target_tx_id.clone()))},
            (Some(_), None) => { vec!((event.txid.clone(), event.target_tx_id.clone())) }, //Prepend
            (None, Some(_)) => {vec!((event.txid.clone(), event.target_tx_id.clone()))}, // Append
            (None, None) => return Err("Must have start or end".to_string())
        };
        for (start, end) in entries.iter() {
            let subgraph = self.LIFELINE_SUBGRAPH.lock().unwrap();
            if !(subgraph.vertices.contains_key(&start.clone()) && subgraph.vertices.contains_key(&end.clone())) {
                return Err(String::from("start and/or end transaction do not appear in subgraph"));
            }
            let start_info = subgraph.vertices.get(&start.clone()).unwrap().clone();
            let end_info = subgraph.vertices.get(&end.clone()).unwrap().clone();
            drop(subgraph);
            let mut pathway = PathwayDescriptor::new();
            let mut finished = false;
            let mut latest = start.clone();
            let mut jump_points:HashMap<i64, i64> = HashMap::new();
            while !finished {
                let ll_data = self.get_lifeline_tx(&latest).expect("Reference to exist");
                //let next_data = ;
                match ll_data.walk_towards(&end) {
                    None => return Err(String::from("No path to end")),
                    Some(data) => {
                        
                        if data.connecting_pathway.tx_count > 1 {
                            jump_points.insert(pathway.tx_count as i64, data.connecting_pathway.tx_count as i64);
                        }
                        pathway.append(data.connecting_pathway);
                        if &data.connecting_timewarp == end {
                            finished = true;
                        }
                        latest = data.connecting_timewarp;
                    }
                }
            }
            let tx_count = pathway.tx_count.clone();
            let to_return = PinDescriptor{
                lifeline_tx: start.clone(),
                timestamp: 0,
                pathway: pathway,
                metadata: String::new(),
                dependant: String::new(),
                endpoints: vec!(),
                lifeline_component:Some(PullJobLifeline {
                    between_end: None,
                    between_start: None,
                    lifeline_start_tx: start.clone(),
                    lifeline_start_ts: start_info.timestamp,
                    lifeline_end_tx: end.clone(),
                    lifeline_end_ts: end_info.timestamp,
                    lifeline_prev: None,
                    lifeline_prev_index: None,
                    lifeline_transitions: jump_points.clone()

                })
            };
            let handle = self.provider.cf_handle(PULLJOB_CACHE_COLUMN).unwrap();
            let r = self.provider.put_cf(handle, format!("{}{}", start, end).as_bytes(), serde_json::to_vec(&to_return).unwrap());
            if r.is_err() {
                return Err(r.unwrap_err().to_string());
            }
            info!("Cached path {}->{} with {} transactions",  start.clone(), end.clone(), tx_count);
        }
        Ok(())
    }


    fn find_include(&self, start:&String, target: &String, exit: &String) -> bool {
            let mut latest = start.clone();
            loop {
                let ll_data = match self.get_lifeline_tx(&latest) {
                    Some(v) => v,
                    None => {error!("Expect reference to exist");return false;}
                };
                //let next_data = ;
                match ll_data.walk_towards(&target) {
                    None =>  {error!("Expect reference to exist");return false;},
                    Some(data) => {
                        if &data.connecting_timewarp == exit {
                            return true;
                        }
                        if &data.connecting_timewarp == target {
                            return false;
                        }
                        latest = data.connecting_timewarp;
                    }
                }
            }
        
    }

    fn get_newer_subgraph_tx(&self, ll:&LifeLineData, subgraph: &std::sync::MutexGuard<'_, LifelineSubGraph>) -> Option<String> {
      
        let closest = match ll.walk_to_closest() {
            Some(v) => v.clone(),
            None => return None
        };
      
        //let subgraph = self.LIFELINE_SUBGRAPH.lock().unwrap();
        let closest_sg_node = match subgraph.vertices.get(&closest.oldest_tx) {
            Some(v) => {
                v.clone()
            },
            None => return None
        };
         
        //No need to do walks if there is only a single reference
        if closest_sg_node.reference_me.len() == 1 {
            return closest_sg_node.reference_me.keys().cloned().next();
        }
        info!("Walking to check TX include in path.");
        //Note: might require a rework, now it tries to walk each time. Might be worth updating life line entries between each new subgraph entry.
        for (k_source, _edge) in closest_sg_node.sorted_reference_me().iter() {
            match subgraph.vertices.get(k_source) {
                Some(v) => {
                    for (k_target, _t_edge) in v.sorted_i_reference().iter() {
                        if k_target == &closest.oldest_tx {
                            if self.find_include(k_source, &closest.oldest_tx, &ll.timewarp_tx) {
                                return Some(k_source.clone());
                            }

                        }
                    }                    
                },
                None => return None
            };
        };
        None
    }
  

}

impl SubgraphPersistence for RocksDBProvider {
    
    fn new_index(&self) -> i64 {
        self.LIFELINE_SUBGRAPH.lock().unwrap().current_index + 1
    }

    fn clone_state(&self) -> LifelineSubGraph {
        self.LIFELINE_SUBGRAPH.lock().unwrap().clone()
    }

    fn is_in_graph(&self, start: &String) -> bool {
        self.LIFELINE_SUBGRAPH.lock().unwrap().vertices.contains_key(start)
    }

    fn store_state(&self) -> Result<(), String> {
        let _r = self.set_generic_cache(crate::indexstorage::P_CACHE_LIFELINE_SUBGRAPH, serde_json::to_vec(&self.LIFELINE_SUBGRAPH.lock().unwrap().clone()).unwrap());
        _r
    }
    fn get_between_subgraph(&self, start:&String) -> Result<(Option<String>, Option<String>), String> {
        let subgraph = self.LIFELINE_SUBGRAPH.lock().unwrap(); 
        let ll_start = match self.get_lifeline_tx(start) {
            Some(v) => v,
            None => return Err(String::from("Start is not a lifeline transaction"))
        };
      
        let oldest = match subgraph.node_indexes.get(start) {
            Some(end_i) => return Ok((Some(start.clone()), None)),
            None => match ll_start.walk_to_closest() {
                Some(v) => (v.oldest_tx.clone(), subgraph.vertices.get(&v.oldest_tx).expect("Subgraph node to exists")),
                None => return Err(String::from("End does not walk to any subgraph TX"))
            }
        };
        if oldest.1.reference_me.is_empty() {
            return Err(String::from("Lifeline is still in 'live' phase. Use a lifeline that is reachable from the top node of "));
        }
        match self.get_newer_subgraph_tx(&ll_start, &subgraph) {
                Some(v) => return Ok(
                    (Some(v.clone()), Some(oldest.0.clone()))
                ),
                None => return Err(String::from("Shouldn't happen"))
        }            
        
    }

    fn get_subgraph_path(&self, start:String, end:String) -> Result<Vec<PinDescriptor>, String> {
        let subgraph = self.LIFELINE_SUBGRAPH.lock().unwrap(); 
        //subgraph.
   
        let ll_start = match self.get_lifeline_tx(&start) {
            Some(v) => v,
            None => return Err(String::from("Start is not a lifeline transaction"))
        };
 
        let ll_end = match self.get_lifeline_tx(&start) {
            Some(v) => v,
            None => return Err(String::from("End is not a lifeline transaction"))
        };
      
        let end_index = match subgraph.node_indexes.get(&end) {
            Some(end_i) => end_i,
            None => match ll_end.walk_to_closest() {
                Some(v) => subgraph.node_indexes.get(&v.oldest_tx).expect("Subgraph node to exists"),
                None => return Err(String::from("End does not walk to any subgraph TX"))
            }
        };
   
        let start_index = match subgraph.node_indexes.get(&start) {
            Some(start_i) => start_i,
            None => {  match self.get_newer_subgraph_tx(&ll_start, &subgraph) {
                Some(v) => subgraph.node_indexes.get(&v).expect(""),
                None => return Err(String::from("End is not a lifeline transaction"))
                }
            }
        };
        
         // let end_index = subgraph.node_indexes.get(&end).unwrap();
        // let start_index = subgraph.node_indexes.get(&start).unwrap().clone();
        //let b:&Graph<String, i64> = &subgraph.petgraph.clone();
        
        //let result =  // petgraph::algo::dijkstra(&subgraph.petgraph, start_index , Some(*end_index), |e| *e.weight());
        let result = petgraph::algo::astar(&subgraph.petgraph, start_index.clone() ,
          |finsihed| &finsihed == end_index, |e| *e.weight(),  |_| 0);
        if result.is_some() {
            let ab = result.unwrap();
            let res:Vec<String> =  ab.1.iter().map(|a| subgraph.node_indexes_reverse.get(a).unwrap().clone()).collect();
            let mut to_return:Vec<PinDescriptor> = vec!();
            let mut iter = res.iter();
            let mut prev = iter.next().unwrap().clone();
            for s in iter {
                let mut pin_desc = self.get_pull_job_cache(prev.clone(), s.clone()).expect("The cache to be filled");
                match to_return.last() {
                    Some(v) => pin_desc.dependant = v.id(),
                    None => {}
                };
                to_return.push(pin_desc);
                prev = s.clone();
            }
            // if to_return.len() == 1 {
            //     return Ok(vec!());
            // }
            return Ok(to_return);
        }
        Err(String::from("This shouldn't happen"))
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
           
            self.reload_pathfinding();
            info!("Subgraph initiated.");
            return Ok(());
        }

        let to_return = match (&event.between_start, &event.between_end) {
            //TODO after a split edge update lifeline transactions to reference new split
            (Some(_), Some(_)) => {info!("Splitting subgraph");LifelineSubGraph::split_edge(&event, &mut subgraph); Ok(())},
            (Some(_), None) => { info!("Prepend subgraph");LifelineSubGraph::prepend(&event, &mut subgraph);Ok(())}, //Prepend
            (None, Some(_)) => {info!("Append subgraph"); LifelineSubGraph::append(&event, &mut subgraph);Ok(())}, // Append
            (None, None) => Err("Must have start or end".to_string())
        };
        subgraph.current_index += 1;
        let prev_toplevel = subgraph.vertices.get(&subgraph.top_level).unwrap();
        let latest_add = subgraph.vertices.get(&event.txid).unwrap();
        if prev_toplevel.timestamp < latest_add.timestamp && latest_add.reference_me.is_empty() {
            subgraph.top_level = event.txid.clone();
        }

        
        //manually drop the lock
        drop(subgraph);
        match self.retarget_paths(&event) {
            Err(e) => error!("Error occured in pull-job caching {}", e.to_string()),
            Ok(()) => {}
        };

        match self.set_pull_job_cache(&event) {
            Err(e) => error!("Error occured in pull-job caching {}", e.to_string()),
            Ok(()) => {}
        };


        match self.store_state() {
            Err(e) => error!("Error occured in storing state {}", e.to_string()),
            Ok(()) => {}
        };
        //TODO don't do a full reload;
        self.reload_pathfinding();
        to_return
    }

    fn reload_pathfinding(&self) {
        let mut subgraph = self.LIFELINE_SUBGRAPH.lock().unwrap();
        let mut tempMap: HashMap<String, petgraph::graph::NodeIndex> = HashMap::new();
        let mut tempMap_reverse: HashMap<petgraph::graph::NodeIndex, String> = HashMap::new();
        let mut tempGraph: petgraph::graph::Graph<String, i64, petgraph::Directed> = petgraph::graph::DiGraph::new();
        for (k, v) in subgraph.vertices.iter() {
            if !tempMap.contains_key(k) {
                let n =  tempGraph.add_node(k.clone());
                    tempMap.insert(k.clone(),n.clone());
                    tempMap_reverse.insert(n.clone(), k.clone());
            }

            for (ref_a, v_2) in v.i_reference.iter() {
                if !tempMap.contains_key(ref_a) {
                    let n =  tempGraph.add_node(ref_a.clone());
                    tempMap.insert(ref_a.clone(),n.clone());
                    tempMap_reverse.insert(n.clone(), ref_a.clone());
                }
                tempGraph.add_edge(tempMap.get(k).unwrap().clone(), tempMap.get(ref_a).unwrap().clone(), v_2.score());
            }
        }
        subgraph.petgraph = tempGraph;
        subgraph.node_indexes = tempMap;
        subgraph.node_indexes_reverse = tempMap_reverse;
    }   

    fn load_subgraph(&mut self) -> Result<(), String>  {
        let _r = self.get_generic_cache(crate::indexstorage::P_CACHE_LIFELINE_SUBGRAPH);
        if _r.is_none() {
            return Ok(());
        }
        let result:LifelineSubGraph = serde_json::from_slice(&_r.unwrap()).expect("Lifeline data to be correct");
        let mut a = self.LIFELINE_SUBGRAPH.lock().expect("Could not lock");
        std::mem::replace(&mut *a, result);
        drop(a);
        self.reload_pathfinding();
        return Ok(());


    }


}