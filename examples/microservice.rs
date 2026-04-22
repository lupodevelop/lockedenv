// macros are called with lockedenv:: prefix

struct AppConfig {
    port: u16,
    db_url: String,
    debug: bool,
}

impl AppConfig {
    fn from_env() -> Self {
        let raw = lockedenv::load! {
            PORT: u16 = 8080,
            DATABASE_URL: String,
            DEBUG: bool = false,
        };
        AppConfig {
            port: raw.PORT,
            db_url: raw.DATABASE_URL,
            debug: raw.DEBUG,
        }
    }
}

fn main() {
    let cfg = AppConfig::from_env();
    println!(
        "running on {} (debug={}, db={})",
        cfg.port, cfg.debug, cfg.db_url
    );
}
