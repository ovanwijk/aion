

//use std::str;
use std::{
    collections::{HashMap, VecDeque, hash_map::DefaultHasher},
    hash::{Hash, Hasher, BuildHasherDefault, BuildHasher},
};
use std::collections::LinkedList;
use crate::SETTINGS;
use riker::actors::*;
use riker::actors::Context;
use iota_lib_rs::iota_model::Transaction;
use aionmodel::tangle::*;
use indexstorage::Persistence;
use timewarping::zmqlistener::*;
use timewarping::Protocol;
use timewarping::Timewarp;
use timewarping::signing;
use timewarping::timewarpwalker::*;
//use std::collections::HashMap;
use indexstorage::*;
#[macro_use]
use log;
use std::sync::Arc;


pub struct TimewarpSelecting {
    picked_timewarp: TimewarpIssuingState,
    available_timewarps: HashMap<String, TimewarpData>,
    
    timeout_in_seconds: i64,
    promote_timeout_in_seconds: i64,
    storage:Arc<dyn Persistence>,
    node: String
}