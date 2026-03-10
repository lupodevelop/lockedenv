// macros are imported via crate root; call as `env_lock::load!`

fn main() {
    let config = env_lock::load! {
        PORT: u16 = 8080,
        DATABASE_URL: String,
        DEBUG: bool = false,
    };
    println!("config: {:?}", config);
}
