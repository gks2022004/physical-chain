use yew::prelude::*;
use wasm_bindgen::prelude::*;

#[derive(Properties, PartialEq, Clone, Default)]
pub struct ViewportProps;

#[function_component(Viewport)]
pub fn viewport(_props: &ViewportProps) -> Html {
    use_effect_with((), |_| {
        // Basic Three.js init via JS global loaded from CDN in index.html
    let chain = crate::storage::load_chain();
    let json = serde_json::to_string(&chain).unwrap_or_else(|_| "{}".into());
    init_three(&json);
        || {}
    });
    html!{ <div id="renderer"></div> }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = initThree)]
    fn init_three(chain_json: &str);

    #[wasm_bindgen(js_name = updateThree)]
    fn update_three(chain_json: &str);
}
