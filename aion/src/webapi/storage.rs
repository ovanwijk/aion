use crate::timewarping::{Protocol, WebRequestType};
//use std::future::Future;
use actix_web::{
    web, Error, HttpRequest, HttpResponse,// HttpServer, Responder,
    Result,
};
use crate::APIActors;
use crate::SETTINGS;
use crate::webapi::ReturnData;
use iota_lib_rs::prelude::*;
use iota_model::Transaction;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateStorageRequest {
    hashes: Vec<String>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateStorageReponse {
    pub start: String,
    pub pathway: crate::pathway::PathwayDescriptor,
    pub node: String
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
    let node = &SETTINGS.node_settings.iri_connection();
    let mut iota = iota_client::Client::new(node); //TODO get from settings  

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
                    node: String::from("")
                }});
                println!("Json took: {}", t.elapsed().as_millis());

            return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(json_text.unwrap()));
        }
    }
    Ok(HttpResponse::NotFound().body(""))
   
}


#[post("/store")]
pub async fn store_storage_object_fn(info: web::Json<CreateStorageReponse>, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    let r = data.storage.get_lifeline_tx(&info.start.clone());
    //if it connects to a lifeline already just return the object.
    if r.is_some() {
        return Ok(HttpResponse::NotFound()
        .content_type("application/json")
        .body("{\"error\":\"First transaction is not a known lifeline transaction\"}"));
    }

    let mut t = std::time::Instant::now();
    let node = &SETTINGS.node_settings.iri_connection();
    let mut iota = iota_client::Client::new(node); //TODO get from settings  

    let iota_trytes = iota.get_trytes(&[info.start.clone()]);
    let tx_trytes = &iota_trytes.unwrap_or_default().take_trytes().unwrap_or_default()[0];
    let tx:Transaction = crate::aionmodel::transaction::parse_tx_trytes(&tx_trytes, &info.start.clone());
    println!("Getting transaction took: {}", t.elapsed().as_millis());
    t = std::time::Instant::now();
    //crate::iota_api::find_paths(&node, start: String, endpoints: Vec<String>);
    let r = data.storage.get_lifeline_ts(&(tx.timestamp + 180));
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
                    node: String::from("")
                }});
                println!("Json took: {}", t.elapsed().as_millis());

            return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(json_text.unwrap()));
        }
    }
    Ok(HttpResponse::NotFound().body(""))
   
}


#[post("/store/create_storage_object")]
pub async fn create_storage_object_fn(info: web::Json<CreateStorageRequest>, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    //let r = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
    let mut t = std::time::Instant::now();
    let node = &SETTINGS.node_settings.iri_connection();
    let mut iota = iota_client::Client::new(node); //TODO get from settings  

    let iota_trytes = iota.get_trytes(&info.hashes);
    let tx_trytes = &iota_trytes.unwrap_or_default().take_trytes().unwrap_or_default()[0];
    let tx:Transaction = crate::aionmodel::transaction::parse_tx_trytes(&tx_trytes, &info.hashes[0]);
    println!("Getting transaction took: {}", t.elapsed().as_millis());
    t = std::time::Instant::now();
    
    let r = if crate::now() - 180 > tx.timestamp { data.storage.get_lifeline_ts(&(tx.timestamp + 180)) }else {data.storage.get_last_lifeline()};
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
                    node: String::from("")
                }});
                println!("Json took: {}", t.elapsed().as_millis());

            return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(json_text.unwrap()));
        }
    }
    Ok(HttpResponse::NotFound().body(""))
   
}