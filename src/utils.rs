use rand::{distributions::Alphanumeric, Rng};

pub fn gen_alias() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect()
}
