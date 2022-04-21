
use env_logger::Env;

pub fn init_log(default_value: &str) {
    env_logger::Builder::from_env(Env::default().default_filter_or(default_value))
        .try_init()
        .unwrap_or_default();
}
