
use crate::SETTINGS;
pub mod rocksdb;
use std::{
    collections::{HashMap, HashSet}};

pub fn get_time_key(timestamp:usize) -> usize {    
    timestamp - (timestamp % SETTINGS.timewarp_index_settings.time_index_clustering_in_seconds)    
}


pub fn get_time_key_range(start:usize, end:usize) -> Vec<usize> {
    let mut to_return:Vec<usize> = vec![];
    let mut next = get_time_key(start);
    while next < end {
        to_return.push(next + SETTINGS.timewarp_index_settings.time_index_clustering_in_seconds);
        next = next + SETTINGS.timewarp_index_settings.time_index_clustering_in_seconds;
    }
    to_return
}


pub trait IndexPersistence {    
    fn add_to_index(&self, key:u64, values:Vec<String>);
    fn remove_from_index(&self, key:u64, values:Vec<String>);
    fn get(&self, key:u64) -> HashSet<String>;
    fn get_all(&self, keys:Vec<u64>) -> Vec<String>;
}



#[derive(Clone, Debug)]
pub struct TimewarpIndexEntry {
    //Always store the key as Big Endian to preserve default byte ordering
    pub key: u64,
    pub values: Vec<String>
}

