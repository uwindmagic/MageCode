use axum::{extract::Json,http::{header,Request,StatusCode},middleware::{self,Next},response::{IntoResponse,Response},routing::{get,post},Router};
use serde::Deserialize;
#[derive(Deserialize)]pub struct Id{pub id:String}
#[derive(Deserialize)]pub struct Link{pub url:String}
async fn guard(req:Request<axum::body::Body>,next:Next)->Response{
    let host=req.headers().get(header::HOST).and_then(|v|v.to_str().ok());
    let origin=req.headers().get(header::ORIGIN).and_then(|v|v.to_str().ok());
    if host!=Some("127.0.0.1:1421")||!matches!(origin,Some("http://127.0.0.1:1420")|Some("http://localhost:1420")){return StatusCode::FORBIDDEN.into_response()}
    next.run(req).await
}
fn result<T:serde::Serialize>(r:Result<T,String>)->Response{match r{Ok(v)=>Json(v).into_response(),Err(e)=>(StatusCode::BAD_REQUEST,Json(serde_json::json!({"error":e}))).into_response()}}
async fn scan()->Response{result(tokio::task::spawn_blocking(magecode_local_source::scan).await.unwrap_or(Err("来源读取任务失败".into())))}
async fn read(Json(input):Json<Id>)->Response{result(tokio::task::spawn_blocking(move||magecode_local_source::read_session(&input.id)).await.unwrap_or(Err("来源读取任务失败".into())))}
async fn open(Json(input):Json<Id>)->Response{result(magecode_local_source::open_session(&input.id).map(|_|serde_json::json!({"opened":true})))}
async fn open_link(Json(input):Json<Link>)->Response{result(magecode_local_source::open_web_link(&input.url).map(|_|serde_json::json!({"opened":true})))}
pub fn run()->Result<(),Box<dyn std::error::Error>>{
    tokio::runtime::Runtime::new()?.block_on(async{
        let router=Router::new().route("/api/local-source/scan",post(scan)).route("/api/local-source/session",post(read)).route("/api/local-source/open",post(open)).route("/api/local-source/open-link",post(open_link)).route_layer(middleware::from_fn(guard)).route("/health",get(||async{"MageCode source bridge"}));
        let listener=tokio::net::TcpListener::bind("127.0.0.1:1421").await?;
        axum::serve(listener,router).await
    })?;
    Ok(())
}
