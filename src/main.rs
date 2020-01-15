use actix_web::{web, App, HttpServer};
mod handlers;
mod resque;

fn get_redis_passwd() -> Option<String> {
    match std::env::var("REDIS_PASSWORD") {
        Ok(val) => Some(val),
        Err(_) => None,
    }
}

fn get_detailed_connection() -> redis::RedisResult<redis::Client> {
    let addr = std::env::var("REDIS_HOSTNAME").unwrap_or("".to_string());
    let port: u16 = std::env::var("REDIS_PORT")
        .unwrap_or("".to_string())
        .parse()
        .unwrap_or(6379);
    let passwd = get_redis_passwd();
    let db: i64 = std::env::var("REDIS_DATABASE")
        .unwrap_or("".to_string())
        .parse()
        .unwrap_or(0);
    let con_addr = redis::ConnectionAddr::Tcp(addr, port);
    let conn_info = redis::ConnectionInfo {
        db: db,
        addr: Box::new(con_addr),
        passwd: passwd,
    };
    redis::Client::open(conn_info)
}

fn create_redis_client() -> redis::RedisResult<redis::Client> {
    match std::env::var("REDIS_CONNECTION_STRING") {
        Ok(val) => redis::Client::open(val.as_ref()),
        Err(_) => get_detailed_connection(),
    }
}

fn make_plugin_manager() -> Result<plugin_manager::PluginManager, std::io::Error> {
    let mut plugin_manager = plugin_manager::PluginManager::new();
    if let Ok(val) = std::env::var("RESQUE_PLUGIN_DIR") {
        plugin_manager.load_directory(val.as_str())?;
    }
    Ok(plugin_manager)
}

fn main() {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    let redis_client = create_redis_client().unwrap();
    let plugin_manager = make_plugin_manager().expect("error loading plugins");
    let sub_uri = std::env::var("SUB_URI").unwrap_or("".to_string());
    let data = web::Data::new(handlers::AppState {
        client: redis_client,
        plugins: plugin_manager,
    });
    let result = HttpServer::new(move || {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .register_data(data.clone())
            .service(
                web::scope(&sub_uri)
                    .service(handlers::home)
                    .service(handlers::resque_stats)
                    .service(handlers::failed_jobs)
                    .service(handlers::active_workers)
                    .service(handlers::queue_details)
                    .service(handlers::delete_failed_jobs)
                    .service(handlers::delete_queue_contents)
                    .service(handlers::delete_failed_job)
                    .service(handlers::retry_failed_job)
                    .service(handlers::retry_all)
                    .service(handlers::delete_worker)
                    .route("{filename:.*}", web::get().to(handlers::static_assets)),
            )
    })
    .bind("0.0.0.0:8080")
    .expect("unable to bind server")
    .run();
    match result {
        Ok(_) => println!("Server listening"),
        Err(e) => println!("Unable to start: {}", e),
    }
}
