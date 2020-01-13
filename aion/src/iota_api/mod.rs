use reqwest;
use std::collections::{HashMap, HashSet};
//use futures::future;
use serde_json;
//use log;
use reqwest::StatusCode;
use crate::pathway::*;
use serde::{Serialize, Deserialize};



fn iota_api_call(node:&str, data: String) ->  Result<String, reqwest::Error>{
    let client = reqwest::blocking::Client::new();
    let res = client.post(node)
        .header("ContentType", "application/json")
        .header("X-IOTA-API-Version", "1")
        .body(data)
        .send();
    if res.is_err() {
        return Err(res.unwrap_err())
    }
    let res = res.unwrap().error_for_status();
    
    if res.is_err() {
        return Err(res.unwrap_err())
    }
    let result = res.unwrap();
    
    Ok(result.text().unwrap())
      
    
    
}


#[derive(Serialize, Deserialize, Clone, Debug)]
struct APIResponse {
    result: Vec<bool>
}

#[derive(Clone, Debug)]
pub struct BTtx {
    pub branch: Option<String>,
    pub trunk: Option<String>,
    pub id: String
}

impl BTtx {
    pub fn mask(&self) -> u8 {
        match (self.branch.as_ref(), self.trunk.as_ref()) {
            (Some(_), Some(_)) => crate::pathway::_Y,
            (None, None) => crate::pathway::_E,
            (Some(_), None) => crate::pathway::_B,
            (None, Some(_)) => crate::pathway::_T,
        }
    } 
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PathfindingResult {
    pub txIDs: Box<Vec<String>>,
    pub branches: Box<Vec<Vec<usize>>>,
    pub trunks: Box<Vec<Vec<usize>>>,  
}

impl PathfindingResult {
    pub fn to_pathway( self) -> crate::pathway::PathwayDescriptor {
        let txIDs = self.txIDs.clone();
        let mut no_target:Box<HashMap<String, BTtx>> = Box::new(txIDs.clone().into_iter().map(|v| (v.clone(), BTtx{branch:None, trunk:None, id: v})).collect());
        for t in self.trunks.into_iter() {
            no_target.get_mut(&txIDs[t[0]]).unwrap().trunk = Some(txIDs[t[1]].clone());
        };
        for b in self.branches.into_iter() {
            no_target.get_mut(&txIDs[b[0]]).unwrap().branch = Some(txIDs[b[1]].clone());
        };
        let mut to_return = PathwayDescriptor::new();
        let mut y_memory: Vec<String> = vec!();
        let mut visited_memory: HashSet<String> = HashSet::new();
        let mut current: BTtx = no_target.get_mut(txIDs.last().unwrap()).expect("The last tx id to be always in the map").clone();
        while current.branch.is_some() || current.trunk.is_some() {
            match current.mask() {
                _Y => {
                    y_memory.push(current.branch.as_ref().unwrap().clone());
                    if visited_memory.contains(&current.id) {
                        let last_result = y_memory.pop();
                        to_return.add_to_path(_E);
                        if last_result.is_none() {
                            //Exit;
                        }else{
                            let tref = last_result.as_ref().unwrap();
                            current = no_target.get_mut(&tref.clone()).unwrap().clone();
                        }
                    } else {
                        visited_memory.insert(current.id.clone());
                        to_return.add_to_path(_Y);
                    }
                    let tref = current.trunk.as_ref().unwrap();
                    current = no_target.get_mut(&tref.clone()).unwrap().clone();
                },
                _E => {
                    let last_result = y_memory.pop();
                    to_return.add_to_path(_E);
                    if last_result.is_none() {
                        //Exit;
                    }else{
                        let tref = last_result.as_ref().unwrap();
                        current = no_target.get_mut(&tref.clone()).unwrap().clone();
                    }
                },
                _T => {
                    if visited_memory.contains(&current.id) {
                        let last_result = y_memory.pop();
                        to_return.add_to_path(_E);
                        if last_result.is_none() {
                            //Exit;
                        }else{
                            let tref = last_result.as_ref().unwrap();
                            current = no_target.get_mut(&tref.clone()).unwrap().clone();
                        }
                    } else {
                        visited_memory.insert(current.id.clone());
                        to_return.add_to_path(_T);
                    }
                    let tref = current.trunk.as_ref().unwrap();
                    current = no_target.get_mut(&tref.clone()).unwrap().clone();
                },
                _B => {
                    if visited_memory.contains(&current.id) {
                        let last_result = y_memory.pop();
                        to_return.add_to_path(_E);
                        if last_result.is_none() {
                            //Exit;
                        }else{
                            let tref = last_result.as_ref().unwrap();
                            current = no_target.get_mut(&tref.clone()).unwrap().clone();
                        }
                    } else {
                        visited_memory.insert(current.id.clone());
                        to_return.add_to_path(_B);
                    }
                    let tref = current.branch.as_ref().unwrap();
                    current = no_target.get_mut(&tref.clone()).unwrap().clone();
                }
                _ => {}
            };
        }


        to_return
    }
}

pub fn pin_transaction_hashes(node:String, tx_hashes:Vec<String> ) -> Result<Vec<bool>, reqwest::Error> {

    let data = format!("{{\"command\": \"pinTransactionHashes\", \"hashes\": {}}}", serde_json::to_string(&tx_hashes).unwrap());
    let string_res = iota_api_call(node.as_str(), data);
    if string_res.is_err() {
        return Err(string_res.unwrap_err())
    }
    let response: APIResponse = serde_json::from_str(&string_res.expect("Normal text to be available")).unwrap();
    Ok(response.result)  
}
pub fn is_pinned(node:String, tx_hashes:Vec<String> ) -> Result<Vec<bool>, reqwest::Error> {

    let data = format!("{{\"command\": \"isPinned\", \"hashes\": {}}}", serde_json::to_string(&tx_hashes).unwrap());
    let string_res = iota_api_call(node.as_str(), data);
    if string_res.is_err() {
        return Err(string_res.unwrap_err())
    }
    let response: APIResponse = serde_json::from_str(&string_res.expect("Normal text to be available")).unwrap();
    Ok(response.result)  
}
pub fn pin_transaction_trytes(node:String, tx_hashes:Vec<String> ) -> Result<Vec<bool>, reqwest::Error> {

    let data = format!("{{\"command\": \"pinTransactionsTrytes\", \"trytes\": {}}}", serde_json::to_string(&tx_hashes).unwrap());
   
    let string_res = iota_api_call(node.as_str(), data);
    if string_res.is_err() {
        return Err(string_res.unwrap_err())
    }
    let response: APIResponse = serde_json::from_str(&string_res.expect("Normal text to be available")).unwrap();
    Ok(response.result)    
}


pub fn find_paths(node:String, start:String,  endpoints:Vec<String> ) -> Result<PathfindingResult, reqwest::Error> {
    //info!("Find paths on: {}", node);
    let data = format!("{{\"command\": \"findPaths\", \"start\": \"{}\" , \"endpoints\": {}}}",start , serde_json::to_string(&endpoints).unwrap());
    let string_res = iota_api_call(node.as_str(), data);
    if string_res.is_err() {
        return Err(string_res.unwrap_err())
    }
    let response: PathfindingResult = serde_json::from_str(&string_res.expect("Normal text to be available")).unwrap();
    Ok(response)    
}

//TODO handle unpinning
pub async fn unpin_transaction_hashes(node:String, tx_hashes:Vec<String> ) -> Result<(), reqwest::Error> {

    let data = format!("{{\"command\": \"unpinTransactions\", \"hashes\": {}}}", serde_json::to_string(&tx_hashes).unwrap());
    //let string_res = iota_api_call(node.as_str(), data).await.expect("Normal text to be available");
    let string_res = iota_api_call(node.as_str(), data);
    if string_res.is_err() {
        return Err(string_res.unwrap_err())
    }
    //let response: APIResponse = serde_json::from_str(&string_res).unwrap();
    Ok(())    
}