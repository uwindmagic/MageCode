fn main(){
    let result=magecode_local_source::scan().expect("default source scan");
    println!("visible_sessions={} total={} excluded={} projects={}",result.sessions.len(),result.total,result.excluded,result.sessions.iter().filter_map(|s|s["source"]["nativeProjectId"].as_str()).collect::<std::collections::HashSet<_>>().len());
    let id=std::env::args().nth(1).unwrap_or("01a174b3-1a61-7830-89dd-68847d814340".into());
    let detail=magecode_local_source::read_session(&id).expect("read local visible messages");
    println!("message_count={} assistant_count={} warnings={} truncated={}",detail["messageCount"],detail["assistantCount"],detail["warnings"].as_array().map_or(0,Vec::len),detail["truncated"]);
}
