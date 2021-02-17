use actix_web::{web, App, HttpServer};
use serde_derive::Deserialize;
mod handlers;
mod resque;

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

async fn open_redis(
    config: &AppConfig,
) -> Result<redis::aio::ConnectionManager, Box<dyn std::error::Error>> {
    let client = match &config.connection_string {
        Some(val) => redis::Client::open(val.as_ref()),
        None => redis::Client::open(redis::ConnectionInfo::from(config)),
    };
    client?
        .get_tokio_connection_manager()
        .await
        .map_err(|e| e.into())
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

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    let app_config = load_config().unwrap();
    let redis = open_redis(&app_config).await?;
    let plugin_manager = make_plugin_manager(&app_config).expect("error loading plugins");
    let sub_uri = std::env::var("SUB_URI").unwrap_or_else(|_| "".to_string());
    let data = web::Data::new(handlers::AppState {
        redis,
        plugins: plugin_manager,
    });
    let result = HttpServer::new(move || {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .app_data(data.clone())
            .service(
                web::scope(&sub_uri)
                    // .route("/", web::get().to(handlers::home))
                    // .route("", web::get().to(handlers::home))
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
                    ), // .route("{filename:.*}", web::get().to(handlers::static_assets)),
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
