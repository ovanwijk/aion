use crate::timewarping::{Protocol, WebRequestType};
//use std::future::Future;
use actix_web::{
    web, Error, HttpRequest, HttpResponse,// HttpServer, Responder,
    Result,
};
use crate::APIActors;
use crate::SETTINGS;
use crate::webapi::ReturnData;
use crate::indexstorage::{PullJob, PinDescriptor};
use iota_lib_rs::prelude::*;
use iota_model::Transaction;
use serde::{Serialize, Deserialize};
extern crate base64;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateStorageRequest {
    hashes: Vec<String>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateStorageReponse {
    pub start: String,
    pub pathway: crate::pathway::PathwayDescriptor,
    pub node: String,
    pub endpoints: Vec<String>,
    pub is_lifeline: bool,
    pub dependant: String
}

impl CreateStorageReponse {
    pub fn to_pin_descriptor(&self) -> PinDescriptor {
        PinDescriptor{
            lifeline_tx: self.start.clone(),
            pathway: self.pathway.clone(),
            endpoints: self.endpoints.clone(),
          //  is_pinned: false,
            timestamp: crate::now(),
            metadata: String::default(),
            pathway_index_splits: vec!(),
            is_lifeline: self.is_lifeline.clone(),
            dependant: self.dependant.clone()
        }
    }

   
}

impl Default for CreateStorageReponse {
    fn default() -> CreateStorageReponse {
        CreateStorageReponse{
            start: String::from(""),
            pathway: crate::pathway::PathwayDescriptor::new(),
            node: String::from(""),
            endpoints: vec!(),
            is_lifeline: false,
            dependant: String::from("")  
        }
    }
}


#[post("/store/connect_storage_object")]
pub async fn connect_storage_object_fn(info: web::Json<CreateStorageReponse>, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    let r = data.storage.get_lifeline_tx(&info.start.clone());
    //if it connects to a lifeline already just return the object.
    if r.is_some() {
        return Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(serde_json::to_string_pretty(&info.into_inner()).unwrap()));
    }

    let mut t = std::time::Instant::now();
    let node = if info.node == "" { SETTINGS.node_settings.iri_connection() } else {info.node.clone()};
    let mut iota = iota_client::Client::new(&node); //TODO get from settings  

    let iota_trytes = iota.get_trytes(&[info.start.clone()]);
    let tx_trytes = &iota_trytes.unwrap_or_default().take_trytes().unwrap_or_default()[0];
    let tx:Transaction = crate::aionmodel::transaction::parse_tx_trytes(&tx_trytes, &info.start.clone());
    println!("Getting transaction took: {}", t.elapsed().as_millis());
    t = std::time::Instant::now();
    //crate::iota_api::find_paths(&node, start: String, endpoints: Vec<String>);
    let r = if crate::now() - 180 > tx.timestamp { data.storage.get_lifeline_ts(&(tx.timestamp + 180)) }else {data.storage.get_last_lifeline()};
    println!("Getting lifeline took: {}", t.elapsed().as_millis());
    t = std::time::Instant::now();
    if r.is_some() {

        let start = r.unwrap().timewarp_tx;
        let object_found = crate::iota_api::find_paths_async(node.to_string(), start.clone(), vec!(info.start.clone())).await;
        println!("Find path took: {}", t.elapsed().as_millis());
        t = std::time::Instant::now();
        if !object_found.is_err() {
            let mut pathway =  object_found.unwrap().to_pathway(start.clone());
            //extend the pathway found to the one given.
            pathway.extend(info.pathway.clone());

            println!("Pathway transform took: {}", t.elapsed().as_millis());
            t = std::time::Instant::now();
        
            
            let json_text = serde_json::to_string_pretty(&ReturnData {
                data: CreateStorageReponse{
                    start: start,
                    pathway: pathway,
                    node: String::from(""),
                    endpoints: info.endpoints.clone(),
                    is_lifeline: info.is_lifeline.clone(),
                    dependant: info.dependant.clone()
                }});
                println!("Json took: {}", t.elapsed().as_millis());

            return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(json_text.unwrap()));
        }
    }
    Ok(HttpResponse::NotFound().body(""))
   
}


#[get("/store/{pinid}")]
pub async fn get_storage_object_fn(info: web::Path<String>, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    let inner = info.into_inner();
    let key = base64::decode_config(&inner, base64::URL_SAFE).unwrap();
    let pull_job = data.storage.get_pull_job(&inner);
    
    if pull_job.is_none() {
        let pin_desc = data.storage.get_pin_descriptor(key);
        if pin_desc.is_none() {
            return Ok(HttpResponse::NotFound()
            .content_type("application/json")
            .body("{\"status\":\"not found\"}"));
        }else{
            return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(format!("{{\"status\":\"success\", \"data\": {}}}", serde_json::to_string_pretty(&pin_desc.unwrap()).unwrap())));
        }
    }else {
        return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(serde_json::to_string_pretty(&pull_job).unwrap()));
    }
   
}


#[post("/store")]
pub async fn store_storage_object_fn(info: web::Json<CreateStorageReponse>, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    let r = data.storage.get_lifeline_tx(&info.start.clone());
    //if it connects to a lifeline already just return the object.
    if r.is_none() {
        return Ok(HttpResponse::NotFound()
        .content_type("application/json")
        .body("{\"error\":\"First transaction is not a known lifeline transaction\"}"));
    }

    let mut t = std::time::Instant::now();
    // let is_local_node = info.node == SETTINGS.node_settings.iri_connection();
    let node = if info.node == "" { SETTINGS.node_settings.iri_connection() } else { info.node.clone() };
    let pin_descriptor = info.to_pin_descriptor();
    data.storage.store_pin_descriptor(pin_descriptor.clone());
    let pulljob = pin_descriptor.to_pull_job(node);//TODO 
   // info!("Pulljob ID: ", )
    data.storage.add_pull_job(&pulljob);
    return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(format!("{{ \"pinid\":\"{}\"}}", base64::encode_config(&pin_descriptor.id(), base64::URL_SAFE))));
    
   
   
}



// #[get("/store")]
// pub async fn get_pin_job(info: web::Json<CreateStorageReponse>, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
//     let r = data.storage.get_lifeline_tx(&info.start.clone());
//     //if it connects to a lifeline already just return the object.
//     if r.is_none() {
//         return Ok(HttpResponse::NotFound()
//         .content_type("application/json")
//         .body("{\"error\":\"First transaction is not a known lifeline transaction\"}"));
//     }

//     let mut t = std::time::Instant::now();
//     // let is_local_node = info.node == SETTINGS.node_settings.iri_connection();
//     let node = if info.node == "" { SETTINGS.node_settings.iri_connection() } else { info.node.clone() };
//     //let mut iota = iota_client::Client::new(node); //TODO get from settings  
//     let pin_descriptor = info.to_pin_descriptor();
//     data.storage.store_pin_descriptor(pin_descriptor.clone());
//     let pulljob = pin_descriptor.to_pull_job(node);
//    // info!("Pulljob ID: ", )
//     data.storage.add_pull_job(&pulljob);
//     return Ok(HttpResponse::Ok()
//             .content_type("application/json")
//             .body(format!("{{ \"pinid\":\"{}\"}}", base64::encode_config(&pin_descriptor.id(), base64::URL_SAFE))));
    
   
// }


#[post("/store/create_storage_object")]
pub async fn create_storage_object_fn(info: web::Json<CreateStorageRequest>, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    //let r = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
    let mut t = std::time::Instant::now();
    let node = &SETTINGS.node_settings.iri_connection();
    let mut iota = iota_client::Client::new(node); //TODO get from settings  

    let iota_trytes = iota.get_trytes(&info.hashes);
    let mut transactions:Vec<Transaction> = vec!();
    let tx_trytes = &iota_trytes.unwrap_or_default().take_trytes().unwrap_or_default();
    let mut min:i64 = 9999999999999999;
    let mut max:i64 = 0;
    for i in 0..tx_trytes.len() {
        transactions.push(crate::aionmodel::transaction::parse_tx_trytes(&tx_trytes[i], &info.hashes[i]));
        min = std::cmp::min(min, transactions[i].attachment_timestamp);
        max = std::cmp::max(max, transactions[i].attachment_timestamp);
    }

    println!("Getting transaction took: {}", t.elapsed().as_millis());
    t = std::time::Instant::now();
    if max - min > 180 {
        return Ok(HttpResponse::BadRequest().body("{\"error\": \"Transaction timestamps are more then 3 minutes apart. Call this method multiple times if needed for different times.\"}"))
    }
    //TODO fix magic numer 180
    let r = if crate::now() - 180 > min { data.storage.get_lifeline_ts(&(min + 180)) }else {data.storage.get_last_lifeline()};
    println!("Getting lifeline took: {}", t.elapsed().as_millis());
    t = std::time::Instant::now();
    if r.is_some() {

        let start = r.unwrap().timewarp_tx;
        let object_found = crate::iota_api::find_paths_async(node.to_string(), start.clone(), info.hashes.clone()).await;
        println!("Find path took: {}", t.elapsed().as_millis());
        t = std::time::Instant::now();
        if !object_found.is_err() {
            let pathway =  object_found.unwrap().to_pathway(start.clone());
            println!("Pathway transform took: {}", t.elapsed().as_millis());
            t = std::time::Instant::now();

            let json_text = serde_json::to_string_pretty(&ReturnData {
                data: CreateStorageReponse{
                    start: start,
                    pathway: pathway,
                    node: String::from(""),
                    endpoints: info.hashes.clone(),
                    is_lifeline: false,
                    dependant: String::from("")
                }});
                println!("Json took: {}", t.elapsed().as_millis());

            return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(json_text.unwrap()));
        }
    }
    Ok(HttpResponse::NotFound().body("{\"error\": \"No lifeline found to connect to.\"}"))
   
}