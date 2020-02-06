
use crate::SETTINGS;
use serde::{Serialize, Deserialize};
use crate::timewarping::signing::*;
use sha2::{Sha256, Digest};
pub mod rocksdb;
use std::sync::Arc;
use crate::pathway::PathwayDescriptor;
extern crate base64;
use std::marker::{Send, Sync};
use std::{
    collections::{HashMap, VecDeque}};


/// Function that takes a seconds-based timestamp and returns an interger time-key index
/// This index is based on SETTINGS.timewarp_index_settings.time_index_clustering_in_seconds
pub fn get_time_key(timestamp:&i64) -> i64 {    
    timestamp - (timestamp % SETTINGS.timewarp_index_settings.time_index_clustering_in_seconds as i64)    
}
use crate::timewarping::timewarpselecting::TimewarpSelectionState;


pub const TIMEWARP_ID_PREFIX:&str = "TW_ID";

pub const LAST_PICKED_TW_ID:&str = "LAST_PICKED_TW_ID";
pub const TW_ISSUING_STATE:&str = "TW_ISSUING_STATE";

/// Given a start and end timestamp returns a range of time-indexes.
pub fn get_time_key_range(start:&i64, end:&i64) -> Vec<i64> {
    let mut to_return:Vec<i64> = vec![];
    let mut next = get_time_key(start);
    while next < *end {
        to_return.push(next + SETTINGS.timewarp_index_settings.time_index_clustering_in_seconds as i64);
        next = next + SETTINGS.timewarp_index_settings.time_index_clustering_in_seconds as i64;
    }
    to_return
}


pub fn storage_hash(start:String, endpoints:&Vec<String>) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.input(start);
    //we sort the endpoints so we get a standarized result.
    let mut it = endpoints.clone();
    it.sort();
    for r in it {
        hasher.input(r);
    }
    
    hasher.result().to_vec()

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


/// Uses the persistence layer to obtain a list of latest kown timewarps.
pub fn get_lastest_known_timewarps(st: Arc<dyn Persistence>) -> Vec<TimewarpData> {
    let timewindow_key = get_time_key(&crate::now());
    let mut a  = st.tw_detection_get(&timewindow_key);
    let mut counter = 1;
    while a.len() == 0 && counter < 100 {
        a = st.tw_detection_get(&(timewindow_key - (SETTINGS.timewarp_index_settings.time_index_clustering_in_seconds * counter)));
        counter += 1;
    }
    
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

/// Persistence trait described all local persistence functions required for AION.
/// There are multiple domains described here:
/// 
/// tw_detection: Relates to all information that is being stored in order to detect
/// possible timewarps.
/// 
/// last picked timewarps: this domain describes timewarps are that are 'followed' and detected
/// this is the basis to select a lifeline on.
/// 
/// Lifeline: a collection of selected timewarps and connecting transaction ID's that are being pinned
/// on the managed IOTA node.
/// 
/// Timewarp-state: All data related to issuing your own timewarps.
/// 
/// Pull jobs: These are persisted jobs that are pulling data from other nodes.
pub trait Persistence: Send + Sync + std::fmt::Debug {

    ///cleans all tracking of timewarps and indexes up till the timewarp
    fn clean_db(&self, timestamp:i64);
    

    /// Adds values to a time-index key-block.
    fn tw_detection_add_to_index(&self, key:i64, values:Vec<(String, String)>);
    /// Removes values from a time-index key-block
    fn tw_detection_remove_from_index(&self, key:i64, values:HashMap<String, String>);
    /// Gets a time-index key-block
    fn tw_detection_get(&self, key:&i64) -> HashMap<String, String>;
    /// Gets all key-blocks given a list of time-keys.
    fn tw_detection_get_all(&self, keys:Vec<&i64>) -> HashMap<String, String>;
    /// Adds data to the observed timewarp set. This must be done in correct order to populate
    /// the TimewarpData statistics.
    fn tw_detection_add_decision_data(&self, tw:  crate::timewarping::Timewarp) -> TimewarpData;
    /// Given a timewarp transaction ID return its related TimewarpData.
    fn tw_detection_get_decision_data(&self, key: String) -> Option<TimewarpData>;

    /// Gets followed timewarp indexes related to a time key. The map equals TimewarpID -> TransactionID
    /// This means only the latest timewarp transaction in relation to the TimewarpID is returned. You
    /// have to follow it manually to obtain the other timewarp transactions within the given time-index block.
    fn get_picked_tw_index(&self, key:i64) -> HashMap<String, String>;
    /// Returnes the latest kown followed timewarp.
    fn get_last_picked_tw(&self) -> Option<TimewarpSelectionState>;
    fn set_last_picked_tw(&self,  state: TimewarpSelectionState) -> Result<(), String>;

    /// When adding transactions to the lifeline there might be a path of hundreds of transactions that require pinning
    /// Therefore lifeline pinning is asynchronous. This function gets lifeline data that still requires pinning.
    fn get_unpinned_lifeline(&self) -> Vec<String>;
    fn set_unpinned_lifeline(&self, ll_data:Vec<String>) -> Result<(), String>;
    /// Appends to the lifeline, this should always be a live process
    fn add_to_lifeline(&self, ll_data:Vec<LifeLineData>) -> Result<(), String>;
    /// Prepends to the known lifeline. The transaction ID of the split + connecting transactions will be added to the
    /// lifeline dataset with a postfix of '_N' where N should mostly be 1 and in rare occations more then 1.
    fn prepend_to_lifeline(&self, ll_data:Vec<LifeLineData>) -> Result<(), String>;
    /// Get lifeline data given en time-index key
    fn get_lifeline(&self, key:&i64) -> Vec<String>;
    /// Gets the closest lifeline transactions to the given timestamp.
    fn get_lifeline_ts(&self, timestamp:&i64) -> Option<LifeLineData>;
    /// Gets a specific lifeline data point given the transaction ID. Note that this might sometimes be post-fixed with '_N'
    fn get_lifeline_tx(&self, key:&String) -> Option<LifeLineData>;
    fn update_lifeline_tx(&self, data:LifeLineData) -> Result<(), String>;
    /// Gets the head of the lifeline.
    fn get_last_lifeline(&self) -> Option<LifeLineData>;

    fn save_timewarp_state(&self, state: TimewarpIssuingState);
    fn get_timewarp_state(&self) -> Option<TimewarpIssuingState>;

    fn store_pin_descriptor(&self, pin_descriptor:PinDescriptor) -> Result<(), String>;
    fn get_pin_descriptor(&self, id:Vec<u8>) -> Option<PinDescriptor>;

    fn add_pull_job(&self, job:&PullJob);
    fn update_pull_job(&self, job:&PullJob);
    fn pop_pull_job(&self, id: String);
    fn get_pull_job(&self, id: &String) -> Option<PullJob>;
    fn next_pull_job(&self, offset:&usize) -> Option<PullJob>;
   
}



#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimewarpIssuingState {
    pub seed: String,
    pub original_tx: String,
    pub unconfirmed_txs: Vec<String>,
    pub tx_timestamp: HashMap<String, i64>,
    pub latest_confimed_tx: String,
    pub latest_timestamp: i64,
    pub latest_confimed_timestamp: i64,
    pub latest_index: String,
    pub latest_private_key: Vec<i8>
}

impl TimewarpIssuingState {
    pub fn latest_tx(&self) -> String {
        if self.unconfirmed_txs.len() == 0 {
            self.latest_confimed_tx.clone()
        }else{
            self.unconfirmed_txs.last().unwrap().clone()
        }
            
    }
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LifeLineData {
    pub timewarp_tx: String,    
    pub trunk_or_branch: bool,
    pub timestamp: i64,
    pub transactions_till_oldest: i64,
    pub oldest_tx: String,
    pub timewarp_id: String,
    pub oldest_timestamp: i64,
    pub unpinned_connecting_txs: Vec<String>, 
    pub connecting_pathway: Option<PathwayDescriptor>,
    pub connecting_timestamp: Option<i64>,
    pub connecting_timewarp: Option<String>
}

impl Default for LifeLineData {
    fn default() -> LifeLineData {
        LifeLineData{ 
            timewarp_tx: String::new(),
            oldest_tx: String::new(),
            timewarp_id: String::new(),
            timestamp: 0,
            oldest_timestamp: 0,
            trunk_or_branch: true,
            transactions_till_oldest: 0, 
            unpinned_connecting_txs: vec!(),
            connecting_pathway: None,
            connecting_timestamp: None,
            connecting_timewarp: None

        }
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PinDescriptor {
    pub timestamp: i64,
    //pub is_pinned: bool,
    pub lifeline_tx: String,
    pub pathway: PathwayDescriptor,
    pub endpoints: Vec<String>,
    pub pathway_index_splits: Vec<isize>,    
    pub metadata: String
}

impl PinDescriptor {
    pub fn id(&self) -> Vec<u8> {
        storage_hash(self.lifeline_tx.clone(), &self.endpoints)
    }

    pub fn to_pull_job(&self, node: String) -> PullJob {
        PullJob{
            id: base64::encode_config(&self.id(), base64::URL_SAFE),
            current_tx: self.lifeline_tx.clone(),
            current_index: 0,
            history: vec!(),
            node: node.clone(),
            pathway: self.pathway.clone(),
            validity_pre_check_tx: vec!(),
            status: String::from("await")



            
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AwaitPinning {
    pub id: String,
    pub transaction_id: String,
    pub transaction_trytes: Option<String>
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PullJob {
    pub id: String,
    pub node: String,
    pub status: String,
    pub current_tx: String,
    pub current_index: usize,
    pub history: Vec<String>,
    pub validity_pre_check_tx: Vec<String>,
    pub pathway: PathwayDescriptor
}

impl PullJob {
    pub fn max_steps(&self) -> usize {
        &self.pathway.size - &self.current_index
    }
}


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
            avg_distance: ((self.avg_distance * self.index_since_id) + new_data.distance) / (self.index_since_id+1),
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

    pub fn score(&self) -> isize {
        (self.index_since_id * self.avg_distance) as isize
    }
 }
use serde::{Serializer, Deserializer};

//  #[derive(Serialize, Deserialize)]
// struct Config {
//     #[serde(serialize_with = "as_base64", deserialize_with = "from_base64")]
//     key: [u8],
// }




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
