use std::{
    collections::{HashMap, HashSet, VecDeque},
   // hash::{Hash, Hasher, BuildHasherDefault, BuildHasher},
};

struct WeightedEdge {
    pub target: String,
    pub timestamp: i64,
    pub transaction_count: i64
}

struct LifelineEdge {

}

pub struct LifelineSubGraph {
    pub current_latest: String,
    pub 

}