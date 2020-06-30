use crate::timewarping::{Protocol, WebRequestType};
//use std::future::Future;
use actix_web::{
    web, Error, HttpRequest, HttpResponse,// HttpServer, Responder,
    Result,
};
use crate::APIActors;
use crate::SETTINGS;
use crate::webapi::ReturnData;
use serde::{Serialize, Deserialize};



#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IotaHashesRequest {
    pub hashes: Vec<String>
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IotaFindRequest {
    pub addresses: Vec<String>,
    pub tags: Vec<String>,
    pub bundles: Vec<String>,
    pub approvees: Vec<String>
}

async fn actix_iota_call(post_data:String, req: HttpRequest) ->  Result<HttpResponse, Error> {
    let c = actix_web::client::Client::new();
    let mut r = c.post(SETTINGS.node_settings.iri_connection());
     for h in req.headers().iter() {
         r = r.header(h.0, h.1.to_str().unwrap());
     };

     let result = r.send_body(post_data).await;
     let mut unwrapped = result.unwrap();
     let mut res = HttpResponse::build(unwrapped.status());

     for h in unwrapped.headers().iter() {
         res.header(h.0, h.1.to_str().unwrap());
     };
     let body = unwrapped.body().await;
    Ok(res.body(body.unwrap()))
}


#[post("/")]
pub async fn iota_proxy(postData: String, data: web::Data<APIActors>, req: HttpRequest) ->  Result<HttpResponse, Error>   {
    if postData.find("getTrytes").is_some() {
        let node = SETTINGS.node_settings.iri_connection();
        let request_data:IotaHashesRequest = serde_json::from_str(postData.as_str()).expect("");
        let mut local_result = data.tx_storage.get_txs(request_data.hashes.clone()).unwrap();
        if local_result.1.len() == 0 {
            return  Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body( serde_json::to_string_pretty(&crate::iota_api::APITryteResponse {
                trytes: local_result.0
            }).unwrap()));
        }

        if local_result.1.len() == request_data.hashes.len() {
            return actix_iota_call(postData, req).await;
        }        

        let get_trytes_result = (crate::iota_api::get_trytes_async(node, request_data.hashes).await).unwrap();

        for missing_tx in local_result.1 {
            std::mem::replace(&mut local_result.0[missing_tx], get_trytes_result.trytes[missing_tx].clone());
        }
        
        return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body( serde_json::to_string_pretty(&crate::iota_api::APITryteResponse {
                trytes: local_result.0
        }).unwrap()))
         
    }
  
    info!("Requested");
    return actix_iota_call(postData, req).await;
}