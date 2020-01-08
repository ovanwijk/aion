use reqwest;
use std::collections::HashMap;
use futures::future;
use serde_json;

use serde::{Serialize, Deserialize};



async fn iota_api_call(node:&str, data: String) ->  Result<String, reqwest::Error>{
    let client = reqwest::Client::new();
    let res = client.post(node)
        .header("ContentType", "application/json")
        .header("X-IOTA-API-Version", "1")
        .body(data)
        .send().await;
    if res.is_err() {
        return Err(res.unwrap_err())
    }
    let result = res.unwrap();
   
    Ok(result.text().await.unwrap())
}


#[derive(Serialize, Deserialize, Clone, Debug)]
struct APIResponse {
    result: Vec<bool>
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PathfindingResult {
    txIDs: Vec<String>,
    branches: Vec<Vec<usize>>,
    trunks: Vec<Vec<usize>>,  
}

pub async fn pin_transaction_hashes(node:String, tx_hashes:Vec<String> ) -> Result<Vec<bool>, reqwest::Error> {

    let data = format!("{{\"command\": \"pinTransactionHashes\", \"hashes\": {}}}", serde_json::to_string(&tx_hashes).unwrap());
    let string_res = iota_api_call(node.as_str(), data).await.expect("Normal text to be available");
    let response: APIResponse = serde_json::from_str(&string_res).unwrap();
    Ok(response.result)    
}
pub async fn is_pinned(node:String, tx_hashes:Vec<String> ) -> Result<Vec<bool>, reqwest::Error> {

    let data = format!("{{\"command\": \"isPinned\", \"hashes\": {}}}", serde_json::to_string(&tx_hashes).unwrap());
    let string_res = iota_api_call(node.as_str(), data).await.expect("Normal text to be available");
    let response: APIResponse = serde_json::from_str(&string_res).unwrap();
    Ok(response.result)    
}
pub async fn pin_transaction_trytes(node:String, tx_hashes:Vec<String> ) -> Result<Vec<bool>, reqwest::Error> {

    let data = format!("{{\"command\": \"pinTransactionsTrytes\", \"trytes\": {}}}", serde_json::to_string(&tx_hashes).unwrap());
    let string_res = iota_api_call(node.as_str(), data).await.expect("Normal text to be available");
    let response: APIResponse = serde_json::from_str(&string_res).unwrap();
    Ok(response.result)    
}


pub async fn pin_transaction_trytes(node:String, tx_hashes:Vec<String> ) -> Result<Vec<bool>, reqwest::Error> {

    let data = format!("{{\"command\": \"pinTransactionsTrytes\", \"trytes\": {}}}", serde_json::to_string(&tx_hashes).unwrap());
    let string_res = iota_api_call(node.as_str(), data).await.expect("Normal text to be available");
    let response: APIResponse = serde_json::from_str(&string_res).unwrap();
    Ok(response.result)    
}

//TODO handle unpinning
pub async fn unpin_transaction_hashes(node:String, tx_hashes:Vec<String> ) -> Result<(), reqwest::Error> {

    let data = format!("{{\"command\": \"unpinTransactions\", \"hashes\": {}}}", serde_json::to_string(&tx_hashes).unwrap());
    let string_res = iota_api_call(node.as_str(), data).await.expect("Normal text to be available");
    //let response: APIResponse = serde_json::from_str(&string_res).unwrap();
    Ok(())    
}