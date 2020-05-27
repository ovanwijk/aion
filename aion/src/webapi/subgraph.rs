use actix_web::{
    web, Error, HttpRequest, HttpResponse,// HttpServer, Responder,
    Result,
};
use crate::APIActors;
use crate::SETTINGS;
use crate::webapi::ReturnData;
use crate::indexstorage::PinDescriptor;
use serde::{Serialize, Deserialize};




#[get("/subgraph")]
pub async fn subgraphFn(data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    //let r = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
    let r = data.storage.clone_state();
    Ok(HttpResponse::Ok()
    .content_type("application/json")
    .body( serde_json::to_string_pretty(&ReturnData {
        data: r
    }).unwrap()))
}

// #[get("/subgraph/force_entry")]
// pub async fn forceEntryFn(data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
//     //let r = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
//     let r = data.storage.get_last_lifeline();
//     //let r = data.storage.clone_state();
//     Ok(HttpResponse::Ok()
//     .content_type("application/json")
//     .body( serde_json::to_string_pretty(&ReturnData {
//         data: timewarp_path
//     }).unwrap()))
// }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BetweenStartEnd {
    between_start:Option<String>,
    between_end:Option<String>
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InsertSubgraph {
    pub data: Vec<PinDescriptor>,
    pub node: String
}




#[post("/subgraph/insert")]
pub async fn insert_subgraph_fn(info: web::Json<InsertSubgraph>, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    match info.data.first() {
        Some(first_item) =>
            match data.storage.get_lifeline_tx(&first_item.lifeline_tx) {
                Some(_ll_tx) => {
                    match &first_item.lifeline_component {
                        Some(ll_comp) => {
                            match &ll_comp.between_start {
                                Some(test_tx) => if !data.storage.is_in_graph(&test_tx) {
                                    return Ok(HttpResponse::NotFound()
                                    .content_type("application/json")
                                    .body("{\"error\":\"Between_start transaction not in subgraph\"}"));
                                },
                                _ => {}
                            }

                            match &ll_comp.between_end {
                                Some(test_tx) => if !data.storage.is_in_graph(&test_tx) {
                                    return Ok(HttpResponse::NotFound()
                                    .content_type("application/json")
                                    .body("{\"error\":\"Between_end transaction not in subgraph\"}"));
                                },
                                _ => {}
                            }
                        }, 
                        None =>  return Ok(HttpResponse::NotFound()
                                .content_type("application/json")
                                .body("{\"error\":\"No lifeline component transaction not in subgraph\"}"))
                    }

                },
                None => return Ok(HttpResponse::NotFound()
                .content_type("application/json")
                .body("{\"error\":\"First transaction is not a known lifeline transaction\"}"))
            }
            ,
        None => return Ok(HttpResponse::NotFound()
        .content_type("application/json")
        .body("{\"error\":\"Empty list\"}"))
        
    };
    //validate all
    for a in &info.data {
        if a.lifeline_component.is_none() {
            return Ok(HttpResponse::NotFound()
                .content_type("application/json")
                .body("{\"error\":\"Missing lifeline_component\"}"))
        }
    }
    let mut pin_ids:Vec<String> = vec!();
    for pin_descriptor in &info.data {
        let node = if info.node == "" { SETTINGS.node_settings.iri_connection() } else { info.node.clone() };
        let pulljob = pin_descriptor.to_pull_job(node);//TODO 
        data.storage.store_pin_descriptor(pin_descriptor.clone());
        data.storage.add_pull_job(&pulljob);
        pin_ids.push(pin_descriptor.id());
    }
    
    //if it connects to a lifeline already just return the object.
  
    // let is_local_node = info.node == SETTINGS.node_settings.iri_connection();
   
   // let pin_descriptor = info.to_pin_descriptor();
    
  
    return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(format!("{{ \"pinids\":\"{}\"}}", serde_json::to_string_pretty(&pin_ids).unwrap())));
    
   
   
}



#[get("/subgraph/connect/{txid}")]
pub async fn subgraphConnectFn(info: web::Path<String>,data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    let result = match data.storage.get_between_subgraph(&info) {
        Ok(v) => v,
        Err(e) => return Ok(HttpResponse::BadRequest().body(format!("{{\"error\" : \"{}\"}}", e.to_string())))
    };

    Ok(HttpResponse::Ok()
    .content_type("application/json")
    .body( serde_json::to_string_pretty(&ReturnData {
        data: BetweenStartEnd{
            between_start: result.0,
            between_end: result.1
        }
    }).unwrap()))

}

#[get("/subgraph/getpath/{start}/{end}")]
pub async fn subgraphGetPathFn(info: web::Path<(String, String)>,data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    //let r = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
    let start = match data.storage.get_lifeline_tx(&info.0) {
        Some(v) => v,
        None => return Ok(HttpResponse::BadRequest().body("{\"error\" : \"Start is not a lifeline transaction\"}"))
    };
    let end = match data.storage.get_lifeline_tx(&info.1) {
        Some(v) => v,
        None => return Ok(HttpResponse::BadRequest().body("{\"error\" : \"End is not a lifeline transaction\"}"))
    };
    //let subgraph = data.storage.LIFELINE_SUBGRAPH.lock();
  

    info!("Calling subgraph connect");

    let timewarp_path = data.storage.get_subgraph_path(info.0.clone(), info.1.clone()).expect("To return a result");
    



    //let r = data.storage.clone_state();
    Ok(HttpResponse::Ok()
    .content_type("application/json")
    .body( serde_json::to_string_pretty(&ReturnData {
        data: timewarp_path
    }).unwrap()))
}