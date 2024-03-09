use redis::Commands;



pub fn update_call_status(call_id: String, status: String) -> redis::RedisResult<()> {
    let coninfo = redis::ConnectionInfo {
        addr: redis::ConnectionAddr::Tcp(std::env::var("REDIS_URI").expect("REDIS_URI not set in environment"), std::env::var("REDIS_PORT").expect("REDIS_PORT not set in environment").parse().unwrap()),
        redis: redis::RedisConnectionInfo {
            db: 0,
            username: Some("default".to_string()),
            password: Some( std::env::var("REDIS_PASSWORD").expect("REDIS_PASSWORD not set in environment"))
        },
    };
    let client = redis::Client::open(coninfo)?;
    let mut con = client.get_connection()?;
    con.hset("call_info", call_id, status)?;
    Ok(())
}



pub fn get_call_status(call_id: String) -> redis::RedisResult<String> {
    let coninfo = redis::ConnectionInfo {
        addr: redis::ConnectionAddr::Tcp(std::env::var("REDIS_URI").expect("REDIS_URI not set in environment"), std::env::var("REDIS_PORT").expect("REDIS_PORT not set in environment").parse().unwrap()),
        redis: redis::RedisConnectionInfo {
            db: 0,
            username: Some("default".to_string()),
            password: Some( std::env::var("REDIS_PASSWORD").expect("REDIS_PASSWORD not set in environment"))
        },
    };
    let client = redis::Client::open(coninfo)?;
    let mut con = client.get_connection()?;
    let status: String = con.hget("call_info", call_id).unwrap_or("call.playback.started".to_string());
    Ok(status)
}
