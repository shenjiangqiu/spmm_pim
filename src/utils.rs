#[cfg(test)]
pub(crate) mod test{
    use env_logger::Env;

    pub fn init_log(){
        env_logger::Builder::from_env(Env::default().default_filter_or("debug")).try_init().unwrap_or_default();
    }
}