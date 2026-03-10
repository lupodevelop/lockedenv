// macros are imported via crate root; call as `lockedenv::load!`

fn main() {
    let config = lockedenv::load! {
        PORT: u16 = 8080,
        DATABASE_URL: String,
        DEBUG: bool = false,
    };
    println!("config: {:?}", config);
}
