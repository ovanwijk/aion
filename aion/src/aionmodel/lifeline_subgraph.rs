use std::{
    collections::{HashMap, HashSet, VecDeque},
   // hash::{Hash, Hasher, BuildHasherDefault, BuildHasher},
};
use std::iter::FromIterator;
use crate::indexstorage::*;
#[macro_use]
use crate::log;
use std::rc::Rc;
use serde::{Serialize, Deserialize};


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphEntryEvent {
    pub index: i64,
    pub txid: String,
    pub timestamp: i64,
    pub tx_distance_count: i64,
    pub target_tx_id: String,
    pub target_timestamp: i64,
    pub between_start: Option<String>,
    pub between_end: Option<String>,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphVertex {    
    pub reference_me: HashMap<String, GraphEdge>,
    pub i_reference: HashMap<String, GraphEdge>,
    pub timestamp:i64
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphEdge {   
    pub tx_distance: i64,
    pub time_distance: i64
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LifelineSubGraph {
    pub top_level_txcount_cutoff:i64,
    pub lastest_and_top_connected: bool,
    pub top_level: String,
    pub current_index: i64,
    pub vertices: HashMap<String, GraphVertex>  
}


impl LifelineSubGraph {
    /**
     *    A          A
     *    |          |
     *    | becomes  X (event describes X-Y relations)
     *    |         /| 
     *    D        Y D
     */
    fn split_edge__(&mut self, event: GraphEntryEvent, storage:Rc<dyn Persistence>) {
        let start = self.vertices.get_mut(&event.between_start.clone().unwrap()).expect("");
        
        //Update top node A to reference X
        let _old_end = start.i_reference.remove(&event.between_end.clone().unwrap()).unwrap();
        start.i_reference.insert(event.txid.clone(), GraphEdge {
            tx_distance: 0, //TODO, implement tx distance calculation event.tx_distance_count.clone(),
            time_distance: event.timestamp - event.target_timestamp
        });
        // Insert X (reference_me: A)
        let mut ref_me = HashMap::new();
        ref_me.insert(event.between_start.clone().unwrap(), GraphEdge{
            tx_distance:0, // event.tx_distance_count.clone(),
            time_distance: start.timestamp - event.timestamp
        });
        //  (i_reference, X & D)
        //X
        let mut i_ref = HashMap::new();
        i_ref.insert(event.target_tx_id.clone(), GraphEdge{
            tx_distance:0, // event.tx_distance_count.clone(),
            time_distance: event.timestamp - event.target_timestamp
        });

        {//Mutable borrow scope
            let end = self.vertices.get_mut(&event.between_end.clone().unwrap()).expect("");
            //D
            i_ref.insert(event.between_end.clone().unwrap(), GraphEdge{
                tx_distance:0, // event.tx_distance_count.clone(),
                time_distance: event.timestamp - end.timestamp
            });
        }
        self.vertices.insert(event.txid.clone(), GraphVertex {
            reference_me: ref_me,
            i_reference: i_ref,
            timestamp: event.timestamp        
        });

        let end = self.vertices.get_mut(&event.between_end.clone().unwrap()).expect("");

        // Update D to have reference_me as X instead of A
        let _old_end = end.reference_me.remove(&event.between_start.unwrap()).unwrap();
        end.reference_me.insert(event.txid.clone(), GraphEdge {
            tx_distance: 0, //TODO, implement tx distance calculation event.tx_distance_count.clone(),
            time_distance: event.timestamp - end.timestamp
        });

        
    }

     pub fn split_edge(event: GraphEntryEvent, subgraph:&mut LifelineSubGraph) {
        let start = subgraph.vertices.get_mut(&event.between_start.clone().unwrap()).expect("");
        
        //Update top node A to reference X
        let _old_end = start.i_reference.remove(&event.between_end.clone().unwrap()).unwrap();
        start.i_reference.insert(event.txid.clone(), GraphEdge {
            tx_distance: 0, //TODO, implement tx distance calculation event.tx_distance_count.clone(),
            time_distance: event.timestamp - event.target_timestamp
        });
        // Insert X (reference_me: A)
        let mut ref_me = HashMap::new();
        ref_me.insert(event.between_start.clone().unwrap(), GraphEdge{
            tx_distance: 0, // event.tx_distance_count.clone(),
            time_distance: start.timestamp - event.timestamp
        });
        //  (i_reference, X & D)
        //X
        let mut i_ref = HashMap::new();
        i_ref.insert(event.target_tx_id.clone(), GraphEdge{
            tx_distance:0, // event.tx_distance_count.clone(),
            time_distance: event.timestamp - event.target_timestamp
        });

        {//Mutable borrow scope
            let end = subgraph.vertices.get_mut(&event.between_end.clone().unwrap()).expect("");
            //D
            i_ref.insert(event.between_end.clone().unwrap(), GraphEdge{
                tx_distance:0, // event.tx_distance_count.clone(),
                time_distance: event.timestamp - end.timestamp
            });
        }
        subgraph.vertices.insert(event.txid.clone(), GraphVertex {
            reference_me: ref_me,
            i_reference: i_ref,
            timestamp: event.timestamp        
        });

        let end = subgraph.vertices.get_mut(&event.between_end.clone().unwrap()).expect("");

        // Update D to have reference_me as X instead of A
        let _old_end = end.reference_me.remove(&event.between_start.unwrap()).unwrap();
        end.reference_me.insert(event.txid.clone(), GraphEdge {
            tx_distance: 0, //TODO, implement tx distance calculation event.tx_distance_count.clone(),
            time_distance: event.timestamp - end.timestamp
        });

        
    }
    /**
     *    A   X
     *    |   |
     *    | > A
     *    |   |
     *    B   B
     */
    pub fn append(event: GraphEntryEvent, subgraph:&mut LifelineSubGraph) {
        let end = subgraph.vertices.get_mut(&event.between_end.clone().unwrap()).expect("");
        
        //Update top node A to reference X
        
        end.reference_me.insert(event.txid.clone(), GraphEdge {
            tx_distance: event.tx_distance_count, //TODO, implement tx distance calculation event.tx_distance_count.clone(),
            time_distance: event.timestamp - event.target_timestamp
        });
        // X will not be referenced, new tip
        let ref_me = HashMap::new();
     
        let mut i_ref = HashMap::new();
        i_ref.insert(event.target_tx_id.clone(), GraphEdge{
            tx_distance: event.tx_distance_count, // event.tx_distance_count.clone(),
            time_distance: event.timestamp - event.target_timestamp
        });

        subgraph.vertices.insert(event.txid.clone(), GraphVertex {
            reference_me: ref_me,
            i_reference: i_ref,
            timestamp: event.timestamp        
        });
    }


     /**
     *    A   A
     *    |   |
     *    | > B
     *    |   |
     *    B   X
     */
    pub fn prepend(event: GraphEntryEvent, subgraph:&mut LifelineSubGraph) {

        // event.txid should be equal to event.between_start

        let start = subgraph.vertices.get_mut(&event.between_start.clone().unwrap()).expect("");
        
        //Update top node A to reference X
        
        start.i_reference.insert(event.target_tx_id.clone(), GraphEdge {
            tx_distance: event.tx_distance_count, //TODO, implement tx distance calculation event.tx_distance_count.clone(),
            time_distance: event.timestamp - event.target_timestamp
        });
        // X will not be referenced, new tip
        let mut ref_me = HashMap::new();
     
        let i_ref = HashMap::new();
        ref_me.insert(event.txid.clone(), GraphEdge{
            tx_distance: event.tx_distance_count, // event.tx_distance_count.clone(),
            time_distance: event.timestamp - event.target_timestamp
        });

        subgraph.vertices.insert(event.target_tx_id.clone(), GraphVertex {
            reference_me: ref_me,
            i_reference: i_ref,
            timestamp: event.timestamp        
        });
    }

    fn store_state(&self, event: GraphEntryEvent, storage:Rc<dyn Persistence>) -> Result<(), String> {
        let _r = storage.set_generic_cache(crate::indexstorage::P_CACHE_LIFELINE_SUBGRAPH, serde_json::to_vec(&self).unwrap());
        _r
    }
    pub fn process_event__(&mut self, event: GraphEntryEvent, storage:Rc<dyn Persistence>) -> Result<(), String> {
        if self.vertices.is_empty() {
            if event.index != 0 {
                return Err("First event index should be one".to_string());
            }
            self.top_level = event.txid.clone();
            self.vertices.insert(event.txid.clone(), GraphVertex {
                reference_me: HashMap::new(),
                i_reference: HashMap::new(),
                timestamp: event.timestamp
            });
            return Ok(());
        }

        let to_return = match (&event.between_start, &event.between_end) {
            //This means s
            (Some(_), Some(_)) => {self.split_edge__(event.clone(), storage.clone()); Ok(())},
            (Some(_), None) => return Err("Must have end".to_string()),
            (None, _) => Err("Must have start".to_string())
        };

        self.store_state(event, storage.clone());

        to_return
    }

    pub fn empty() -> LifelineSubGraph {
        LifelineSubGraph {
            top_level_txcount_cutoff: 0,
            top_level: String::from(""),
            current_index: -1,
            lastest_and_top_connected: false,
            vertices: HashMap::new()
        } 
    }

    pub fn load(storage:Rc<dyn Persistence>) -> Result<LifelineSubGraph, String>  {
        let _r = storage.get_generic_cache(crate::indexstorage::P_CACHE_LIFELINE_SUBGRAPH);
        if _r.is_none() {
            return Ok(LifelineSubGraph {
                top_level_txcount_cutoff: 0,
                top_level: String::from(""),
                current_index: -1,
                lastest_and_top_connected: false,
                vertices: HashMap::new()
            } );
        }
        let result:LifelineSubGraph = serde_json::from_slice(&_r.unwrap()).expect("Lifeline data to be correct");
        return Ok(result);


    }

}