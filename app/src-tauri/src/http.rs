use reqwest::blocking::{Client, ClientBuilder};
use std::sync::Once;

pub fn client() -> ClientBuilder {
    static RUSTLS: Once = Once::new();
    RUSTLS.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
    Client::builder()
}
