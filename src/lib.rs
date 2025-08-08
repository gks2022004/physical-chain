use wasm_bindgen::prelude::*;
use yew::prelude::*;

mod blockchain;
mod qr;
mod viz;
mod storage;

#[wasm_bindgen]
pub fn run_app() {
    yew::Renderer::<App>::new().render();
}

#[wasm_bindgen(start)]
pub fn start() {
    // Auto-start when loaded by Trunk's default loader
    run_app();
}

#[wasm_bindgen]
pub fn js_bootstrap() {
    // Hook up buttons and video via JS-side IDs.
    // Most logic handled in the Yew app.
}

#[function_component(App)]
fn app() -> Html {
    html! {
        <MainApp />
    }
}

#[derive(Properties, PartialEq, Clone, Default)]
struct MainProps;

#[function_component(MainApp)]
fn main_app(_props: &MainProps) -> Html {
    html! {
        <>
            <crate::qr::Scanner />
            <crate::viz::Viewport />
        </>
    }
}
