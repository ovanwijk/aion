
// use std::collections::LinkedList;
// use serde_json::json;
// use serde::{Serialize, Deserialize};
use std::marker::{Send, Sync};
// use riker::actors::*;
// use riker::actors::Context;
//use std::convert::TryInto;
use std::sync::Mutex;
use std::rc::Rc;
//use crate::timewarping::Protocol;
use crate::timewarping::timewarpselecting::TimewarpSelectionState;
use crate::indexstorage::*;
use crate::SETTINGS;
use crate::aionmodel::lifeline_subgraph::*;
use ::rocksdb::{DB, Options, WriteBatch};
use std::{
    collections::{HashMap}};
use lru_cache::LruCache;
pub mod timewarp_detection_persistence;
pub mod generic_storage_persistence;
pub mod lifeline_persistence;
pub mod timewarpissueing_persistence;
pub mod pulljobs_persistence;
pub mod subgraph_persistence;

pub const TW_DETECTION_RANGE_TX_INDEX_COLUMN:&str = "TW_DETECTION_RANGE_TX_INDEX_COLUMN";
pub const TW_DETECTION_RANGE_GROUP_INDEX_COLUMN:&str = "TW_DETECTION_RANGE_GROUP_INDEX_COLUMN";
pub const FOLLOWED_TW_INDEX_COLUMN:&str = "FOLLOWED_TW_INDEX_COLUMN";
pub const FOLLOWED_TW_INDEX_RANGE_COLUMN:&str = "FOLLOWED_TW_INDEX_RANGE_COLUMN";

pub const LIFELINE_INDEX_COLUMN:&str = "LIFELINE_INDEX_COLUMN";
pub const LIFELINE_INDEX_RANGE_COLUMN:&str = "LIFELINE_INDEX_RANGE_COLUMN";

pub const PULLJOB_ID_COLUMN:&str = "PULLJOB_ID_COLUMN";
pub const PINNED_TX_COUNTER:&str = "PINNED_TX_COUNTER";
pub const PATHWAY_DESCRIPTORS:&str = "PATHWAY_DESCRIPTORS";
pub const FLEXIBLE_ZERO:&str = "FLEXIBLE_ZERO";
pub const PERSISTENT_CACHE:&str = "PERSISTENT_CACHE";



#[derive(Debug)]
pub struct RocksDBProvider {
    provider: DB,
    TW_DETECTION_RANGE_TX_INDEX_COLUMN_cache: Mutex<LruCache<i64, HashMap<String, String>>>,
    FOLLOWED_TW_INDEX_RANGE_COLUMN_cache:  Mutex<LruCache<i64, HashMap<String, String>>>,
    FOLLOWED_TW_INDEX_COLUMN_cache:  Mutex<LruCache<String, TimewarpData>>,
    LAST_PICKED_TW_cache: Mutex<Option<TimewarpSelectionState>>,
    LIFELINE_SUBGRAPH: Mutex<LifelineSubGraph>
}

impl CleanDB for RocksDBProvider {
    fn clean_db(&self, timestamp:i64) {
        //TODO implement
    }
}

impl Persistence for RocksDBProvider {

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
        
        let mut toReturn = RocksDBProvider {
            provider: db,
            TW_DETECTION_RANGE_TX_INDEX_COLUMN_cache: Mutex::new(LruCache::new(SETTINGS.cache_settings.db_memory_cache as usize)),
            FOLLOWED_TW_INDEX_RANGE_COLUMN_cache: Mutex::new(LruCache::new(SETTINGS.cache_settings.db_memory_cache as usize)),
            LAST_PICKED_TW_cache: Mutex::new(None),
            FOLLOWED_TW_INDEX_COLUMN_cache: Mutex::new(LruCache::new(SETTINGS.cache_settings.db_memory_cache as usize)),
            LIFELINE_SUBGRAPH: Mutex::new(LifelineSubGraph::empty())
        };
        toReturn.load_subgraph();
        toReturn
       
    }
    
}