
use std::collections::LinkedList;
use riker::actors::*;
use riker::actors::Context;
use std::convert::TryInto;
use timewarping::Protocol;
 use std::cell::RefCell;
use indexstorage::{TimewarpIndexEntry, IndexPersistence};
use crate::SETTINGS;
use rocksdb::{DB, Options, ColumnFamily};
use std::{
    collections::{HashMap, HashSet}};
use lru_cache::LruCache;

const RANGE_INDEX_COLUMN:&str = "RANGE_INDEX_COLUMN";
const KEY_INDEX_COLUMN:&str = "KEY_INDEX_COLUMN";




impl TimewarpIndexEntry {
    pub fn from_vec_u8(to_parse:String) -> HashSet<String> {
        to_parse.split(',').map(|x| String::from(x)).collect()
    }
}



pub struct RocksDBProvider {
    provider: DB,
    cache: RefCell<LruCache<u64, HashSet<String>> >
}

impl Actor for RocksDBProvider {
      // we used the #[actor] attribute so CounterMsg is the Msg type
    type Msg = Protocol;

    fn recv(&mut self,
                ctx: &Context<Self::Msg>,
                msg: Self::Msg,
                sender: Sender) {

        // Use the respective Receive<T> implementation
        match msg {
            //RemoveFromIndexPersistence
            Protocol::AddToIndexPersistence(__msg) => self.receive_addtoindex(ctx, __msg, sender),
            Protocol::RemoveFromIndexPersistence(__msg) => self.receive_removefromindex(ctx, __msg, sender),
            _ => ()
        }
    }
}

impl  IndexPersistence for RocksDBProvider {
    fn add_to_index(&self, key:u64, values:Vec<String>) {
        let handle = self.provider.cf_handle(RANGE_INDEX_COLUMN).unwrap();
        let mut data = self.get(key);
        for v in values.iter() {
            data.insert(v.to_string());
        }
       // values.iter().map(|v| d;
        if !data.is_empty() {
            let concatted = data.iter().fold(String::from(""), |mut a, b| {a.push_str(","); a.push_str(b ); a})[1..].to_string();
            let _r = self.provider.put_cf(handle, key.to_be_bytes(), concatted.as_bytes());
            self.cache.borrow_mut().insert(key, data);
        }
        
    }
    fn remove_from_index(&self, key:u64, values:Vec<String>){}
    fn get(&self, key:u64) -> HashSet<String> {
        let mut borrowed_cached = self.cache.borrow_mut();
        let cached = borrowed_cached.get_mut(&key);
        if cached.is_some() {
            let result = cached.unwrap();
            result.clone()
        }else { 
            let handle = self.provider.cf_handle(RANGE_INDEX_COLUMN).unwrap();

            match self.provider.get_cf(handle, key.to_be_bytes()) {
                Ok(Some(value)) => TimewarpIndexEntry::from_vec_u8(value.to_utf8().unwrap().to_string()),
                Ok(None) => HashSet::new(),
                Err(e) => {println!("operational problem encountered: {}", e);
                HashSet::new()},
                _ => HashSet::new()
            }
        }
       
    }
    fn get_all(&self, keys:Vec<u64>) -> Vec<String> {vec![]}
}

impl RocksDBProvider {
    fn actor() -> Self {
        let path = SETTINGS.timewarp_index_settings.time_index_database_location.clone();
        let mut db_opts = Options::default();
        db_opts.create_missing_column_families(true);
        db_opts.create_if_missing(true);      
        let db = DB::open_cf(&db_opts, path, vec![RANGE_INDEX_COLUMN, KEY_INDEX_COLUMN]).unwrap();       
        
        RocksDBProvider {
            provider: db,
            cache: RefCell::new(LruCache::new(SETTINGS.cache_settings.db_memory_cache))
        }
    }
    fn receive_addtoindex(&mut self,
                _ctx: &Context<Protocol>,
                _msg: TimewarpIndexEntry,
                _sender: Sender) {

                    
                    self.add_to_index(_msg.key, _msg.values);
      //  println!("Got start listening {}", _msg.host);

      //  self.socket.connect("tcp://127.0.0.1:5556").unwrap();
       // self.socket.set_subscribe("tx ".as_bytes()).unwrap();

        //_ctx.myself.tell(Protocol::PullTxData(PullTxData), None);
      //
    }

     fn receive_removefromindex(&mut self,
                _ctx: &Context<Protocol>,
                _msg: TimewarpIndexEntry,
                _sender: Sender) {
      //  println!("Got start listening {}", _msg.host);

      //  self.socket.connect("tcp://127.0.0.1:5556").unwrap();
       // self.socket.set_subscribe("tx ".as_bytes()).unwrap();

        //_ctx.myself.tell(Protocol::PullTxData(PullTxData), None);
      //
    }


    pub fn props() -> BoxActorProd<RocksDBProvider> {
        Props::new(RocksDBProvider::actor)
    }
}