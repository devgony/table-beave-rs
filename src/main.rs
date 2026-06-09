mod parser;

#[cfg(target_arch = "wasm32")]
mod app;

#[cfg(target_arch = "wasm32")]
fn main() {
    app::mount();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("beave-rs is a Leptos web app. Run it with `trunk serve`.");
}
