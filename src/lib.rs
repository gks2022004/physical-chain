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
            <div id="app" style="display:grid; grid-template-columns:360px 1fr; grid-template-rows:auto 1fr; grid-template-areas:'header header' 'sidebar main'; height:100vh;">
                <header style="grid-area:header; padding:10px 16px; background:#0f172a; color:#e2e8f0; display:flex; align-items:center; gap:12px;">
                    <h1 style="font-size:18px; margin:0;">{"Physical Chain"}</h1>
                    <span style="font-size:12px; padding:2px 6px; border-radius:4px; background:#1e293b; color:#a3e635;">{"MVP"}</span>
                </header>
                <aside style="grid-area:sidebar; border-right:1px solid #e5e7eb; padding:12px; overflow:auto;">
                    <crate::qr::Scanner />
                </aside>
                <main style="grid-area:main; position:relative;">
                    <div id="renderer" style="position:absolute; inset:0;"></div>
                    <crate::viz::Viewport />
                </main>
            </div>
        </>
    }
}
