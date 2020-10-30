use actix_web::{web, App, HttpServer};
use redis;
use serde_derive::Deserialize;
mod handlers;
mod resque;
use config;

#[derive(Deserialize)]
struct AppConfig {
    connection_string: Option<String>,
    password: Option<String>,
    port: u16,
    database: i64,
    hostname: String,
    plugin_dir: Option<String>,
    username: Option<String>,
}

fn load_config() -> Result<AppConfig, config::ConfigError> {
    let mut settings = config::Config::default();
    settings
        .set_default("hostname", String::from(""))?
        .set_default("port", 6379)?
        .set_default("database", 0)?
        .merge(config::Environment::with_prefix("REDIS"))?;
    if let Ok(val) = std::env::var("RESQUE_PLUGIN_DIR") {
        settings.set("plugin_dir", val)?;
    }
    settings.try_into::<AppConfig>()
}

impl From<&AppConfig> for redis::ConnectionInfo {
    fn from(config: &AppConfig) -> Self {
        redis::ConnectionInfo {
            addr: Box::new(redis::ConnectionAddr::Tcp(
                config.hostname.clone(),
                config.port,
            )),
            db: config.database,
            username: config.username.clone(),
            passwd: config.password.clone(),
        }
    }
}

fn create_redis_client(config: &AppConfig) -> redis::RedisResult<redis::Client> {
    match &config.connection_string {
        Some(val) => redis::Client::open(val.as_ref()),
        None => redis::Client::open(redis::ConnectionInfo::from(config)),
    }
}

fn make_plugin_manager(
    config: &AppConfig,
) -> Result<plugin_manager::PluginManager, std::io::Error> {
    let mut plugin_manager = plugin_manager::PluginManager::new();
    if let Some(val) = config.plugin_dir.as_ref() {
        plugin_manager.load_directory(val)?;
    }
    Ok(plugin_manager)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    let app_config = load_config().unwrap();
    let manager = create_redis_client(&app_config).unwrap();
    let pool = r2d2::Pool::builder().build(manager).unwrap();
    let plugin_manager = make_plugin_manager(&app_config).expect("error loading plugins");
    let sub_uri = std::env::var("SUB_URI").unwrap_or("".to_string());
    let data = web::Data::new(handlers::AppState {
        pool: pool,
        plugins: plugin_manager,
    });
    let result = HttpServer::new(move || {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .app_data(data.clone())
            .service(
                web::scope(&sub_uri)
                    .route("/", web::get().to(handlers::home))
                    .route("", web::get().to(handlers::home))
                    .service(
                        web::scope("/api")
                            .service(handlers::resque_stats)
                            .service(handlers::failed_jobs)
                            .service(handlers::active_workers)
                            .service(handlers::queue_details)
                            .service(handlers::delete_failed_jobs)
                            .service(handlers::delete_queue_contents)
                            .service(handlers::delete_failed_job)
                            .service(handlers::retry_failed_job)
                            .service(handlers::retry_all)
                            .service(handlers::delete_worker),
                    )
                    .route("{filename:.*}", web::get().to(handlers::static_assets)),
            )
    })
    .bind("0.0.0.0:8080")
    .expect("unable to bind server")
    .run()
    .await;
    if let Err(e) = result {
        println!("Unable to start: {}", e);
    }
    Ok(())
}
