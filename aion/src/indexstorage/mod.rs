
use crate::SETTINGS;
use serde::{Serialize, Deserialize};
pub mod rocksdb;
use std::marker::{Send, Sync};
use std::{
    collections::{HashMap, HashSet}};

pub fn get_time_key(timestamp:&i64) -> i64 {    
    timestamp - (timestamp % SETTINGS.timewarp_index_settings.time_index_clustering_in_seconds as i64)    
}


pub fn get_time_key_range(start:&i64, end:&i64) -> Vec<i64> {
    let mut to_return:Vec<i64> = vec![];
    let mut next = get_time_key(start);
    while next < *end {
        to_return.push(next + SETTINGS.timewarp_index_settings.time_index_clustering_in_seconds as i64);
        next = next + SETTINGS.timewarp_index_settings.time_index_clustering_in_seconds as i64;
    }
    to_return
}


pub trait Persistence: Send + Sync + std::fmt::Debug {    
    fn tw_detection_add_to_index(&self, key:i64, values:Vec<(String, String)>);
    fn tw_detection_remove_from_index(&self, key:i64, values:HashMap<String, String>);
    fn tw_detection_get(&self, key:&i64) -> HashMap<String, String>;
    fn tw_detection_get_all(&self, keys:Vec<&i64>) -> HashMap<String, String>;
    fn get_picked_tw_range(&self, key:i64) -> HashMap<String, String>;
    fn last_picked_tw(&self) -> Option<TimewarpDetectionData>;
    fn add_last_picked_tw(&self, timewarps:Vec<TimewarpDetectionData>) -> Result<(), String>;


}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimewarpDetectionData {
    timewarpid: String,
    source: String,
    target: String,
    distance: i64,
    trunk_or_branch: bool,
    timestamp: i64,
    timestamp_deviation_factor:f64,
    avg_distance: i64,
    index_since_id: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RangeTxIDLookup {
    pub key: i64,
    pub values: HashMap<String, String>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TwIndexEntry { 
    pub timestamp: Vec<String>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TwIndex {
    //Always store the key as Big Endian to preserve default byte ordering
    pub key: i64,
    pub values: HashMap<String, TwIndexEntry>
}
