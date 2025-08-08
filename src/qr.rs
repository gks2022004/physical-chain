use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, HtmlVideoElement, MediaStream, MediaStreamConstraints, MediaStreamTrack, window, CanvasRenderingContext2d, ImageData};
use yew::prelude::*;
use crate::blockchain::Interaction;
use crate::storage::{load_chain, save_chain};

#[derive(Properties, PartialEq, Clone, Default)]
pub struct ScannerProps;

#[function_component(Scanner)]
pub fn scanner(_props: &ScannerProps) -> Html {
    let status = use_state(|| "Idle".to_string());
    let blocks = use_state(|| vec![]);

    let video_ref = use_node_ref();
    let canvas_ref = use_node_ref();

    // Load chain on mount
    {
        let blocks = blocks.clone();
        use_effect_with((), move |_| {
            let chain = load_chain();
            blocks.set(chain.blocks.iter().rev().take(10).cloned().collect());
            || {}
        });
    }

    let stop_stream = Callback::from({
        let status = status.clone();
        let video_ref = video_ref.clone();
        move |_| {
            if let Some(video) = video_ref.cast::<HtmlVideoElement>() {
                if let Some(stream) = video.src_object() {
                    if let Ok(ms) = stream.dyn_into::<MediaStream>() {
                        let tracks = ms.get_tracks();
                        let len = tracks.length();
                        for i in 0..len {
                            let js = tracks.get(i);
                            if js.is_undefined() || js.is_null() { continue; }
                            if let Ok(track) = js.dyn_into::<MediaStreamTrack>() {
                                track.stop();
                            }
                        }
                        video.set_src_object(None);
                        status.set("Stopped".into());
                    }
                }
            }
        }
    });

    let start_scan = Callback::from({
        let status_handle = status.clone();
        let video_ref_handle = video_ref.clone();
        let canvas_ref_handle = canvas_ref.clone();
        let blocks_handle = blocks.clone();
        move |_| {
            let status = status_handle.clone();
            let video_ref = video_ref_handle.clone();
            let canvas_ref = canvas_ref_handle.clone();
            let blocks = blocks_handle.clone();
            wasm_bindgen_futures::spawn_local(async move {
                status.set("Requesting camera...".into());
                let nav = window().unwrap().navigator();
                let md = nav.media_devices().unwrap();
                let constr = MediaStreamConstraints::new();
                constr.set_video(&JsValue::TRUE);
                match md.get_user_media_with_constraints(&constr) {
                    Ok(promise) => {
                        match wasm_bindgen_futures::JsFuture::from(promise).await {
                            Ok(stream_js) => {
                                if let Some(video) = video_ref.cast::<HtmlVideoElement>() {
                                    let stream = MediaStream::from(stream_js);
                                    let _ = video.set_src_object(Some(&stream));
                                    let _ = video.play();
                                    status.set("Scanning...".into());

                                    // Simple polling decode loop using canvas and qrcode reader.
                                    let video_clone = video_ref.clone();
                                    let canvas_clone = canvas_ref.clone();
                                    let status_clone = status.clone();
                                    let blocks_clone = blocks.clone();
                                    gloo::timers::callback::Interval::new(500, move || {
                                        if let (Some(video), Some(canvas)) = (video_clone.cast::<HtmlVideoElement>(), canvas_clone.cast::<HtmlCanvasElement>()) {
                                            let w = video.video_width();
                                            let h = video.video_height();
                                            if w == 0 || h == 0 { return; }
                                            canvas.set_width(w);
                                            canvas.set_height(h);
                                            let ctx = canvas.get_context("2d").unwrap().unwrap().dyn_into::<CanvasRenderingContext2d>().unwrap();
                                            let _ = ctx.draw_image_with_html_video_element(&video, 0.0, 0.0);
                                            if let Ok(img_data) = ctx.get_image_data(0.0, 0.0, w as f64, h as f64) {
                                                if let Some(text) = decode_qr_from_imagedata(&img_data) {
                                                    status_clone.set(format!("QR: {}", text));
                                                    // Add a block
                                                    let mut chain = load_chain();
                                                    let device_hash = device_fingerprint();
                                                    let ts = js_sys::Date::new_0().get_time();
                                                    let data = Interaction { qr_content: text, device_hash, geolocation: None };
                                                    chain.add_block(data, ts);
                                                    save_chain(&chain);
                                                    blocks_clone.set(chain.blocks.iter().rev().take(10).cloned().collect());
                                                    // update 3D viz
                                                    let json = serde_json::to_string(&chain).unwrap_or_else(|_| "{}".into());
                                                    update_three(&json);
                                                }
                                            }
                                        }
                                    }).forget();
                                }
                            }
                            Err(e) => {
                                web_sys::console::error_1(&e);
                                status.set("Camera error".into());
                            }
                        }
                    }
                    Err(e) => {
                        web_sys::console::error_1(&e);
                        status.set("Camera error".into());
                    }
                }
            });
        }
    });

    html! {
        <>
        <div style="display:none">
            <video id="video" ref={video_ref} playsinline=true></video>
            <canvas id="canvas" ref={canvas_ref}></canvas>
        </div>
        <div>
            <button class="btn primary" onclick={start_scan.clone()}>{"Start Scan"}</button>
            <button class="btn" onclick={stop_stream}>{"Stop"}</button>
            <span style="margin-left:8px" class="muted">{ (*status).clone() }</span>
            <ul id="blocks">
                { for (*blocks).iter().map(|b| html!{ <li>{format!("#{} {}..", b.index, &b.hash[..8.min(b.hash.len())])}</li> }) }
            </ul>
        </div>
        </>
    }
}

fn device_fingerprint() -> String {
    // Basic fingerprint from user agent + time salt
    let ua = web_sys::window().unwrap().navigator().user_agent().unwrap_or_default();
    let now = js_sys::Date::new_0().get_time();
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    hasher.update(format!("{}:{}", ua, (now as u64 / 1000 / 60)).as_bytes());
    hex::encode(hasher.finalize())
}

fn decode_qr_from_imagedata(img: &ImageData) -> Option<String> {
    // Use JS global jsQR if available (loaded in index.html). Fallback to None.
    let window = web_sys::window()?;
    let jsqr = js_sys::Reflect::get(&window, &JsValue::from_str("jsQR")).ok()?;
    if jsqr.is_undefined() || jsqr.is_null() { return None; }
    // Get typed array from ImageData.data via Reflect
    let img_js: &JsValue = img.as_ref();
    let data = js_sys::Reflect::get(img_js, &JsValue::from_str("data")).ok()?;
    let width = img.width();
    let height = img.height();
    let res = js_sys::Reflect::apply(
        &jsqr.into(),
        &JsValue::NULL,
        &js_sys::Array::of3(&data, &JsValue::from_f64(width as f64), &JsValue::from_f64(height as f64)),
    ).ok()?;
    if res.is_undefined() || res.is_null() { return None; }
    let text = js_sys::Reflect::get(&res, &JsValue::from_str("data")).ok()?;
    text.as_string()
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = updateThree)]
    fn update_three(chain_json: &str);
}
