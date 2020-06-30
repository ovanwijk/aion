

use crate::SETTINGS;
use serde::{Serialize, Deserialize};
use crate::timewarping::signing::*;
use sha2::{Sha256, Digest};

use std::sync::Arc;
use crate::pathway::PathwayDescriptor;
extern crate base64;
use std::marker::{Send, Sync};
use std::{
    collections::{HashMap, VecDeque}};
use ::rocksdb::{DB, Options, WriteBatch};
use std::sync::Mutex;
use lru_cache::LruCache;

pub const ADDRESS_INDEX_COLUMN:&str = "ADDRESS_INDEX_COLUMN";
pub const TAG_INDEX_COLUMN:&str = "TAG_INDEX_COLUMN";
pub const BUNDLE_INDEX_COLUMN:&str = "BUNDLE_INDEX_COLUMN";
pub const APPROVEE_INDEX_COLUMN:&str = "APPROVEE_INDEX_COLUMN";
pub const PAGING_INDEX_COLUMN:&str = "PAGING_INDEX_COLUMN";
pub const TX_COLUMN:&str = "TX_COLUMN";


#[derive(Debug)]
pub struct RocksDBTXProvider {
    provider: DB,
    ADDRESS_INDEX_CACHE: Mutex<LruCache<String, Vec<String>>>,
    APPROVEE_INDEX_CACHE:  Mutex<LruCache<String, Vec<String>>>,
    TAG_INDEX_CACHE:  Mutex<LruCache<String, Vec<String>>>,
    BUNDLE_INDEX_CACHE:  Mutex<LruCache<String, Vec<String>>>,
    TX_CACHE:  Mutex<LruCache<String, String>>,
    UPDATE_LOCK: Mutex<bool>,
    
}

fn to_vec(bytes:Vec<u8>, chunk_size:usize) -> Vec<String> {  
    bytes.chunks(chunk_size).map(|v| String::from_utf8(v.to_vec()).unwrap()).collect() 
}

fn to_byte_vec(bv:&Vec<String>) -> Vec<u8> {
    let mut to_return: Vec<u8> = vec!();
    for s in bv {
        to_return.append(&mut s.as_bytes().to_vec());
    }
    to_return
}


pub trait TXPersistence: Send + Sync + std::fmt::Debug  {
    fn store_txs(&self, pairs:Vec<(String, String)>) -> Result<(), String>;
    fn get_txs(&self, keys:Vec<String>) -> Result<(Vec<String>, Vec<usize>), String>;
    fn find_bundle(&self, keys:Vec<String>) -> Result<Vec<String>, String>;
    fn find_tag(&self, keys:Vec<String>) -> Result<Vec<String>, String>;
    fn find_address(&self, keys:Vec<String>) -> Result<Vec<String>, String>;
    fn find_approvees(&self, keys:Vec<String>) -> Result<Vec<String>, String>;
}


fn page_ref(page:i64, id:String) -> String {
    if page == 0 { return id;}
    [page.to_string() , String::from("_") , id].concat()
}
impl TXPersistence for RocksDBTXProvider {
    fn store_txs(&self, pairs: Vec<(String, String)>) -> Result<(), String> {
        let _lock = self.UPDATE_LOCK.lock();
        let address_handle = self.provider.cf_handle(ADDRESS_INDEX_COLUMN).unwrap();
        let tag_handle = self.provider.cf_handle(TAG_INDEX_COLUMN).unwrap();
        let bundle_handle = self.provider.cf_handle(BUNDLE_INDEX_COLUMN).unwrap();
        let approvee_handle = self.provider.cf_handle(APPROVEE_INDEX_COLUMN).unwrap();
        let tx_handle = self.provider.cf_handle(TX_COLUMN).unwrap();
        let paging_handle = self.provider.cf_handle(PAGING_INDEX_COLUMN).unwrap();
        
        let mut batch = WriteBatch::default();
        //let mut borrowed_cached = self.TX_CACHE.lock().unwrap();
        //let cached = borrowed_cached.get_mut(key);
        let mut address_map:HashMap<String, Vec<String>> = HashMap::new();
        let mut tag_map:HashMap<String, Vec<String>> = HashMap::new();
        let mut bundle_map:HashMap<String, Vec<String>> = HashMap::new();
        let mut approvee_map:HashMap<String, Vec<String>> = HashMap::new();
        let mut paging_map:HashMap<String, i64> = HashMap::new();
        for (key, value) in pairs {
            let hasKey = self.provider.get_cf(tx_handle, key.as_bytes()).expect("Basic DB functions to work");
            if hasKey.is_none() {
                let tag:String = value[2592..2619].into();         
                let address:String = value[2187..2268].into();
                
                let bundle:String = value[2349..2430].into();
                let trunk_transaction:String = value[2430..2511].into();
                let branch_transaction:String = value[2511..2592].into();                
                batch.put_cf(tx_handle, key.as_bytes(), value.as_bytes());

               
              
                if !address_map.contains_key(&address) {
                    address_map.insert(address.clone(), match self.provider.get_cf(address_handle, address.as_bytes()).expect("DB to work")  {
                        Some(v) => to_vec(v.to_vec(), 81),
                        None => vec!()
                    });
                }
                // Address Vec paging
                /*

                    Paging only creates overhead of there are 50 or more transactions per index.
                    The default page is the address/tag itself, and when more then 50 references are in there
                    a record is recorded in PAGING_INDEX_COLUMN.

                    Paging keys start at 1 and increment as 1_ADDRESS/TAG, 2_ADDRESS/TAG etc
                */
                if address_map.get(&address).unwrap().len() >= 50 {
                    if !paging_map.contains_key(&address) {
                        paging_map.insert(address.clone(), match self.provider.get_cf(paging_handle, address.as_bytes()).expect("DB to work")  {
                            Some(v) => crate::read_be_i64(&mut v.to_vec().as_slice()),
                            None => 1
                        });
                    }
                    let address_paging = paging_map.get(&address).unwrap().clone();
                    let mut page_r = page_ref(address_paging.clone(), address.clone());
                    if !address_map.contains_key(&page_r) {
                        address_map.insert(page_r.clone(), match self.provider.get_cf(address_handle, page_r.as_bytes()).expect("DB to work")  {
                            Some(v) => to_vec(v.to_vec(), 81),
                            None => vec!()
                        });
                    }

                    if !address_map.contains_key(&page_r) {
                        address_map.insert(page_r.clone(), match self.provider.get_cf(address_handle, page_r.as_bytes()).expect("DB to work")  {
                            Some(v) => to_vec(v.to_vec(), 81),
                            None => vec!()
                        });
                    }

                    if address_map.get(&page_r).unwrap().len() >= 50 {
                        info!("New address page {} for {}", (address_paging+1), address.clone());                        
                        paging_map.insert(address.clone(), address_paging.clone() +1);
                        page_r = page_ref(address_paging.clone() + 1, address.clone());
                        address_map.insert(page_r.clone(), vec!());
                    }
                    address_map.get_mut(&page_r).unwrap().push(key.clone());
                }else{
                    address_map.get_mut(&address).unwrap().push(key.clone());
                }
                

                if !tag_map.contains_key(&tag) {
                    tag_map.insert(tag.clone(), match self.provider.get_cf(tag_handle, tag.as_bytes()).expect("DB to work")  {
                        Some(v) => to_vec(v.to_vec(), 27),
                        None => vec!()
                    });
                }
              
                 // Tag Vec paging
                 if tag_map.get(&tag).unwrap().len() >= 50 {
                    if !paging_map.contains_key(&tag) {
                        paging_map.insert(tag.clone(), match self.provider.get_cf(paging_handle, tag.as_bytes()).expect("DB to work")  {
                            Some(v) => crate::read_be_i64(&mut v.to_vec().as_slice()),
                            None => 1
                        });
                    }
                    let tag_paging = paging_map.get(&tag).unwrap().clone();
                    let mut page_r = page_ref(tag_paging.clone(), tag.clone());
                    if !tag_map.contains_key(&page_r) {
                        tag_map.insert(page_r.clone(), match self.provider.get_cf(tag_handle, page_r.as_bytes()).expect("DB to work")  {
                            Some(v) => to_vec(v.to_vec(), 81),
                            None => vec!()
                        });
                    }

                    if !tag_map.contains_key(&page_r) {
                        tag_map.insert(page_r.clone(), match self.provider.get_cf(tag_handle, page_r.as_bytes()).expect("DB to work")  {
                            Some(v) => to_vec(v.to_vec(), 81),
                            None => vec!()
                        });
                    }

                    if tag_map.get(&page_r).unwrap().len() >= 50 {
                        info!("New tag page {} for {}", (tag_paging+1), tag.clone());                        
                        paging_map.insert(tag.clone(), tag_paging.clone() +1);
                        page_r = page_ref(tag_paging.clone() + 1, tag.clone());
                        tag_map.insert(page_r.clone(), vec!());
                    }
                    tag_map.get_mut(&page_r).unwrap().push(key.clone());
                }else{
                    tag_map.get_mut(&tag).unwrap().push(key.clone());
                }

                //Bundle
                if !bundle_map.contains_key(&bundle) {
                    bundle_map.insert(bundle.clone(), match self.provider.get_cf(bundle_handle, bundle.as_bytes()).expect("DB to work")  {
                        Some(v) => to_vec(v.to_vec(), 81),
                        None => vec!()
                    });
                }
                bundle_map.get_mut(&bundle).unwrap().push(key.clone());

                //Approvee
                if !approvee_map.contains_key(&trunk_transaction) {
                    approvee_map.insert(trunk_transaction.clone(), match self.provider.get_cf(approvee_handle, trunk_transaction.as_bytes()).expect("DB to work")  {
                        Some(v) => to_vec(v.to_vec(), 81),
                        None => vec!()
                    });
                }
                approvee_map.get_mut(&trunk_transaction).unwrap().push(key.clone());

                if !approvee_map.contains_key(&branch_transaction) {
                    approvee_map.insert(branch_transaction.clone(), match self.provider.get_cf(approvee_handle, branch_transaction.as_bytes()).expect("DB to work")  {
                        Some(v) => to_vec(v.to_vec(), 81),
                        None => vec!()
                    });
                }
                approvee_map.get_mut(&branch_transaction).unwrap().push(key.clone());
                
            
            }else{
                //Is already stored, just ignore
            }
            
        }
        
        let mut address_cached = self.ADDRESS_INDEX_CACHE.lock().unwrap();
        for (k,v) in address_map.iter() {
            address_cached.remove(k);
            batch.put_cf(address_handle, k.as_bytes(), to_byte_vec(v));
        }

        for (k,v) in paging_map.iter() {
            address_cached.remove(k);
            batch.put_cf(paging_handle, k.as_bytes(), v.to_be_bytes());
        }
        drop(address_cached);
        let mut tag_cached = self.TAG_INDEX_CACHE.lock().unwrap();
        for (k,v) in tag_map.iter() {
            tag_cached.remove(k);
            batch.put_cf(tag_handle, k.as_bytes(), to_byte_vec(v));
        }
        drop(tag_cached);
        let mut bundle_cached = self.BUNDLE_INDEX_CACHE.lock().unwrap();
        for (k,v) in bundle_map.iter() {
            bundle_cached.remove(k);
            batch.put_cf(bundle_handle, k.as_bytes(), to_byte_vec(v));
        }
        drop(bundle_cached);
        let mut approvee_cached = self.APPROVEE_INDEX_CACHE.lock().unwrap();
        for (k,v) in approvee_map.iter() {
            approvee_cached.remove(k);
            batch.put_cf(approvee_handle, k.as_bytes(), to_byte_vec(v));
        }
        drop(approvee_cached);

        let _l = self.provider.write(batch);
       
        if _l.is_err() {
            return Err(format!("Something went wrong writing: {}", _l.unwrap_err().to_string()))
        }

        Ok(())
    }

    fn get_txs(&self, keys: Vec<String>) -> Result<(Vec<String>, Vec<usize>), String> {
       let tx_handle = self.provider.cf_handle(TX_COLUMN).unwrap();
        let mut borrowed_cached = self.TX_CACHE.lock().unwrap();
        let mut to_return:Vec<String> = vec!();
        let mut to_return_not_found:Vec<usize> = vec!();
        let mut not_found_counter:usize = 0;
        for tx in keys {
            let cached = borrowed_cached.get_mut(&tx);
            if cached.is_some() {
                to_return.push(cached.unwrap().clone());
            }else {
                match self.provider.get_cf(tx_handle, tx.as_bytes()).unwrap() {
                    Some(v) => {
                        let ve = String::from_utf8(v.to_vec()).unwrap();
                        borrowed_cached.insert(tx.clone(), ve.clone());
                        to_return.push(ve);
                    },
                    None => {
                        to_return.push("9".repeat(2673));
                        to_return_not_found.push(not_found_counter)}
                }
            }
            not_found_counter += 1;
        }
        Ok((to_return, to_return_not_found))
    }

    fn find_bundle(&self, keys: Vec<String>) -> Result<Vec<String>, String> {
        let tx_handle = self.provider.cf_handle(BUNDLE_INDEX_COLUMN).unwrap();
        let mut borrowed_cached = self.BUNDLE_INDEX_CACHE.lock().unwrap();
        let mut to_return:Vec<String> = vec!();
        for tx in keys {
            let cached = borrowed_cached.get_mut(&tx);
            if cached.is_some() {
                to_return.append(&mut cached.unwrap().clone());
            }else {
                match self.provider.get_cf(tx_handle, tx.as_bytes()).unwrap() {
                    Some(v) => {
                        let mut ve = to_vec(v.to_vec(), 81);
                        borrowed_cached.insert(tx.clone(), ve.clone());
                        to_return.append(&mut ve);
                    },
                    None => {to_return.push(String::new())}
                }
            }
        }
        Ok(to_return)  
    }

    fn find_tag(&self, keys: Vec<String>) -> Result<Vec<String>, String> {
        let tx_handle = self.provider.cf_handle(TAG_INDEX_COLUMN).unwrap();
        let paging_handle = self.provider.cf_handle(PAGING_INDEX_COLUMN).unwrap();
        let mut borrowed_cached = self.TAG_INDEX_CACHE.lock().unwrap();
        let mut to_return:Vec<String> = vec!();
        for tx in keys {
            let cached = borrowed_cached.get_mut(&tx);
            if cached.is_some() {
                to_return.append(&mut cached.unwrap().clone());
            }else {
                let latest_page = match self.provider.get_cf(paging_handle, tx.as_bytes()).unwrap() {
                    Some(v) => page_ref(crate::read_be_i64(&mut v.to_vec().as_slice()), tx.clone()),
                    None => tx.clone()
                };
                match self.provider.get_cf(tx_handle, latest_page.as_bytes()).unwrap() {
                    Some(v) => {
                        let mut ve = to_vec(v.to_vec(), 27);
                        borrowed_cached.insert(tx.clone(), ve.clone());
                        to_return.append(&mut ve);
                    },
                    None => {to_return.push(String::new())}
                }
            }
        }
        Ok(to_return)  
    }

    fn find_approvees(&self, keys: Vec<String>) -> Result<Vec<String>, String> {
        let tx_handle = self.provider.cf_handle(APPROVEE_INDEX_COLUMN).unwrap();
        let mut borrowed_cached = self.APPROVEE_INDEX_CACHE.lock().unwrap();
        let mut to_return:Vec<String> = vec!();
        for tx in keys {
            let cached = borrowed_cached.get_mut(&tx);
            if cached.is_some() {
                to_return.append(&mut cached.unwrap().clone());
            }else {
                match self.provider.get_cf(tx_handle, tx.as_bytes()).unwrap() {
                    Some(v) => {
                        let mut ve = to_vec(v.to_vec(), 81);
                        borrowed_cached.insert(tx.clone(), ve.clone());
                        to_return.append(&mut ve);
                    },
                    None => {to_return.push(String::new())}
                }
            }
        }
        Ok(to_return)  
    }

    fn find_address(&self, keys: Vec<String>) -> Result<Vec<String>, String> {
        let tx_handle = self.provider.cf_handle(ADDRESS_INDEX_COLUMN).unwrap();
        let paging_handle = self.provider.cf_handle(PAGING_INDEX_COLUMN).unwrap();
        let mut borrowed_cached = self.ADDRESS_INDEX_CACHE.lock().unwrap();
        let mut to_return:Vec<String> = vec!();
        for tx in keys {
            let cached = borrowed_cached.get_mut(&tx);
            if cached.is_some() {
                to_return.append(&mut cached.unwrap().clone());
            }else {
                let latest_page = match self.provider.get_cf(paging_handle, tx.as_bytes()).unwrap() {
                    Some(v) => page_ref(crate::read_be_i64(&mut v.to_vec().as_slice()), tx.clone()),
                    None => tx.clone()
                };
                match self.provider.get_cf(tx_handle, latest_page.as_bytes()).unwrap() {
                    Some(v) => {
                        let mut ve = to_vec(v.to_vec(), 81);
                        borrowed_cached.insert(tx.clone(), ve.clone());
                        to_return.append(&mut ve);
                    },
                    None => {to_return.push(String::new())}
                }
            }
        }
        Ok(to_return)  
    }
}

impl RocksDBTXProvider {
    pub fn new() -> RocksDBTXProvider {
        let path = format!("{}/tx", SETTINGS.timewarp_index_settings.time_index_database_location.clone());
        let mut db_opts = Options::default();
        db_opts.create_missing_column_families(true);
        db_opts.create_if_missing(true);      
        let db = DB::open_cf(&db_opts, path, vec![ADDRESS_INDEX_COLUMN,
            TAG_INDEX_COLUMN,
            BUNDLE_INDEX_COLUMN,
            APPROVEE_INDEX_COLUMN,
            TX_COLUMN,
            PAGING_INDEX_COLUMN]).unwrap();
        
        let toReturn = RocksDBTXProvider {
            provider: db,
            ADDRESS_INDEX_CACHE: Mutex::new(LruCache::new(SETTINGS.cache_settings.db_memory_cache as usize)),
            TAG_INDEX_CACHE: Mutex::new(LruCache::new(SETTINGS.cache_settings.db_memory_cache as usize)),
            BUNDLE_INDEX_CACHE: Mutex::new(LruCache::new(SETTINGS.cache_settings.db_memory_cache as usize)),
            APPROVEE_INDEX_CACHE: Mutex::new(LruCache::new(SETTINGS.cache_settings.db_memory_cache as usize)),
            TX_CACHE:  Mutex::new(LruCache::new(SETTINGS.cache_settings.db_memory_cache as usize)),
            UPDATE_LOCK: Mutex::new(true)
        };
        
        toReturn


    }
}
// 

unsafe impl Send for RocksDBTXProvider {}
unsafe impl Sync for RocksDBTXProvider {}