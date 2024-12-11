use rand::Rng;
use rand::distributions::Alphanumeric;

fn generate_random_string(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

pub fn generate_username() -> String {
    generate_random_string(8)
}

pub fn generate_password() -> String {
    generate_random_string(12)
}

