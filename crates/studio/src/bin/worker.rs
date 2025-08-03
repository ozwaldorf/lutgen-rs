//! Web worker

#[cfg(target_arch = "wasm32")]
fn main() {
    use gloo_worker::Registrable;
    lutgen_studio::Worker::registrar().register();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // stub for clippy
}
