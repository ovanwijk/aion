
use crate::SETTINGS;
use serde::{Serialize, Deserialize};
use crate::timewarping::signing::*;
pub mod rocksdb;
use std::sync::Arc;

use std::marker::{Send, Sync};
use std::{
    collections::{HashMap, HashSet}};

pub fn get_time_key(timestamp:&i64) -> i64 {    
    timestamp - (timestamp % SETTINGS.timewarp_index_settings.time_index_clustering_in_seconds as i64)    
}
use iota_lib_rs::iota_conversion;


pub const TIMEWARP_ID_PREFIX:&str = "TW_ID";

pub fn get_time_key_range(start:&i64, end:&i64) -> Vec<i64> {
    let mut to_return:Vec<i64> = vec![];
    let mut next = get_time_key(start);
    while next < *end {
        to_return.push(next + SETTINGS.timewarp_index_settings.time_index_clustering_in_seconds as i64);
        next = next + SETTINGS.timewarp_index_settings.time_index_clustering_in_seconds as i64;
    }
    to_return
}

pub fn get_n_timewarp_transactions(timewarp_id:String, n:i32, st: Arc<dyn Persistence>) -> Vec<String> {

    let mut result:Vec<String> = vec!();
    let mut counter = 0;
    result.push(timewarp_id.to_string());
    while counter < n {
        let r = st.tw_detection_get_decision_data(result.last().unwrap().to_string());
        if r.is_some() {
            result.push(r.unwrap().target_hash());
            counter +=1 ;
        }else{
            counter = n;
        }
    }
    result
}


pub fn get_lastest_known_timewarps(st: Arc<dyn Persistence>) -> Vec<TimewarpData> {
    let timewindow_key = get_time_key(&crate::now());
    let a  = st.tw_detection_get(&timewindow_key);
    let mut to_return:Vec<TimewarpData> = vec!();
    for (_k, v) in a.iter().filter(|&(k, _v)| k.starts_with(&TIMEWARP_ID_PREFIX)) {
        let t_result = st.tw_detection_get_decision_data(v.to_string());
        if t_result.is_some() {
            to_return.push(t_result.unwrap());
        }
    }
    to_return.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    to_return
}


pub trait Persistence: Send + Sync + std::fmt::Debug {    
    fn tw_detection_add_to_index(&self, key:i64, values:Vec<(String, String)>);
    fn tw_detection_remove_from_index(&self, key:i64, values:HashMap<String, String>);
    fn tw_detection_get(&self, key:&i64) -> HashMap<String, String>;
    fn tw_detection_get_all(&self, keys:Vec<&i64>) -> HashMap<String, String>;
    fn tw_detection_add_decision_data(&self, tw:  crate::timewarping::Timewarp) -> TimewarpData;
    fn tw_detection_get_decision_data(&self, key: String) -> Option<TimewarpData>;

    fn get_picked_tw_index(&self, key:i64) -> HashMap<String, String>;
    fn get_last_picked_tw(&self) -> Option<TimewarpData>;
    fn add_last_picked_tw(&self, timewarps:Vec<TimewarpData>) -> Result<(), String>; 

    fn save_timewarp_state(&self, state: TimewarpIssuingState);
    fn get_timewarp_state(&self) -> Option<TimewarpIssuingState>;
   
}



#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimewarpIssuingState {
    pub seed: String,
    pub original_tx: String,
    pub latest_tx: String,
    pub latest_timestamp: i64,
    pub is_confirmed:bool,
    pub latest_index: String,
    pub latest_private_key: Vec<i8>
}

impl TimewarpIssuingState {
    pub fn random_id(&self) -> &str {
        if &self.original_tx.len() >= &(9 as usize) {
            &self.original_tx[0..9]
        }else{
            "999999999"
        }
        
    }
    pub fn latest_index_num(&self) -> i64 {
        trytes_to_number(&self.latest_index)
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimewarpData {
    pub timewarpid: String,
    pub hash: String,
    pub trunk: String,
    pub branch: String,
    pub distance: i64,
    pub trunk_or_branch: bool,
    pub timestamp: i64,
    pub timestamp_deviation_factor:f64,
    pub avg_distance: i64,
    pub index_since_id: i64,
}

// #[derive(Clone, Debug)]
// pub struct WarpWalk {
//     pub from: String,
//     pub timestamp: i64,
//     pub distance: i64,
//     pub trunk_or_branch:bool,
//     pub target: String
// }


impl TimewarpData {
    pub fn advance(&self, new_data: &crate::timewarping::Timewarp) -> TimewarpData {
        TimewarpData {
            timewarpid: self.timewarpid.clone(),
            hash: new_data.source_hash.clone(),
            branch: new_data.source_branch.clone(),
            trunk: new_data.source_trunk.clone(),
            timestamp: new_data.source_timestamp,
            timestamp_deviation_factor: -1.0,
            distance: new_data.distance,
            index_since_id: self.index_since_id + 1,
            avg_distance: ((self.distance * 9) + new_data.distance) / 10,
            trunk_or_branch: self.trunk_or_branch
        }
     }
     pub fn target_hash(&self) -> String {
        if self.trunk_or_branch {
            self.trunk.clone()
        }else{
            self.branch.clone()
        }
    }
 }




// #[derive(Serialize, Deserialize, Clone, Debug)]
// pub struct RangeTxIDLookup {
//     pub key: i64,
//     pub values: HashMap<String, String>
// }

// #[derive(Serialize, Deserialize, Clone, Debug)]
// pub struct TwIndexEntry { 
//     pub timestamp: Vec<String>
// }

// #[derive(Serialize, Deserialize, Clone, Debug)]
// pub struct TwIndex {
//     //Always store the key as Big Endian to preserve default byte ordering
//     pub key: i64,
//     pub values: HashMap<String, TwIndexEntry>
// }
