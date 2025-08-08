use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, HtmlVideoElement, MediaStream, MediaStreamConstraints, MediaStreamTrack, window, CanvasRenderingContext2d, ImageData};
use yew::prelude::*;
use yew::events::MouseEvent;
use crate::blockchain::{Interaction, Block};
use gloo::timers::callback::Interval;
use std::rc::Rc;
use crate::storage::{load_chain, save_chain};

#[derive(Properties, PartialEq, Clone, Default)]
pub struct ScannerProps;

#[function_component(Scanner)]
pub fn scanner(_props: &ScannerProps) -> Html {
    let status = use_state(|| "Idle".to_string());
    let blocks = use_state(|| vec![]);
    let facing_mode = use_state(|| "environment".to_string()); // "user" or "environment"
    let _scan_interval = use_mut_ref(|| Option::<gloo::timers::callback::Interval>::None);
    let scan_interval = _scan_interval.clone();
    // Session-level debounce: recently seen QR strings (clear on restart)
    let seen_recent = use_mut_ref(|| std::collections::VecDeque::<(String, f64)>::new());

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
        let si = scan_interval.clone();
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
                        // stop polling
                        *si.borrow_mut() = None;
                    }
                }
            }
        }
    });

    // helper: start scanning now (captures scan_interval)
    let start_scan_now: Rc<dyn Fn(UseStateHandle<String>, NodeRef, NodeRef, UseStateHandle<Vec<Block>>, String)> = {
        let scan_interval_outer = scan_interval.clone();
        Rc::new(move |status: UseStateHandle<String>,
                      video_ref: NodeRef,
                      canvas_ref: NodeRef,
                      blocks: UseStateHandle<Vec<Block>>,
                      facing: String| {
        // stop existing before starting new
        if let Some(video) = video_ref.cast::<HtmlVideoElement>() {
            if video.src_object().is_some() {
                if let Some(stream) = video.src_object() {
                    if let Ok(ms) = stream.dyn_into::<MediaStream>() {
                        let tracks = ms.get_tracks();
                        for i in 0..tracks.length() {
                            let js = tracks.get(i);
                            if let Ok(track) = js.dyn_into::<MediaStreamTrack>() { track.stop(); }
                        }
                        let _ = video.set_src_object(None);
                    }
                }
            }
        }
    let scan_interval_inner = scan_interval_outer.clone();
    let seen_recent_inner = seen_recent.clone();
    wasm_bindgen_futures::spawn_local(async move {
            status.set("Requesting camera...".into());
            let nav = window().unwrap().navigator();
            let md = nav.media_devices().unwrap();
            let constr = MediaStreamConstraints::new();
            // Build a JS object for video constraints with facingMode
            let video_obj = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&video_obj, &JsValue::from_str("facingMode"), &JsValue::from_str(&facing));
            constr.set_video(&video_obj.into());
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
                                let seen_recent = seen_recent_inner.clone();
                                let interval = Interval::new(400, move || {
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
                                                // Debounce same QR within 5s and prevent duplicates in chain
                                                let now_ms = js_sys::Date::new_0().get_time();
                                                // prune old entries
                                                {
                                                    let mut q = seen_recent.borrow_mut();
                                                    while let Some((_, ts)) = q.front() {
                                                        if now_ms - *ts > 5000.0 { q.pop_front(); } else { break; }
                                                    }
                                                    if q.iter().any(|(s, _)| s == &text) {
                                                        // already seen very recently
                                                        return;
                                                    }
                                                }

                                                let mut chain = load_chain();
                                                if chain.has_qr_content(&text) {
                                                    status_clone.set("Already mined for this QR".into());
                                                    // record seen to throttle UI jitter
                                                    seen_recent.borrow_mut().push_back((text, now_ms));
                                                    return;
                                                }

                                                status_clone.set("Mining…".into());
                                                let device_hash = device_fingerprint();
                                                let data = Interaction { qr_content: text.clone(), device_hash, geolocation: None };
                                                chain.add_block(data, now_ms);
                                                save_chain(&chain);
                                                blocks_clone.set(chain.blocks.iter().rev().take(10).cloned().collect());
                                                // update 3D viz
                                                let json = serde_json::to_string(&chain).unwrap_or_else(|_| "{}".into());
                                                update_three(&json);
                                                // record seen so repeated frames within 5s don’t retrigger
                                                seen_recent.borrow_mut().push_back((text, now_ms));
                                            }
                                        }
                                    }
                                });
                                *scan_interval_inner.borrow_mut() = Some(interval);
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
    })};

    // Start Scan button handler
    let start_scan_button: Callback<MouseEvent> = {
        let status = status.clone();
        let video_ref = video_ref.clone();
        let canvas_ref = canvas_ref.clone();
        let blocks = blocks.clone();
        let facing_mode = facing_mode.clone();
        let sc = start_scan_now.clone();
        Callback::from(move |_| {
            let facing = (*facing_mode).clone();
            (*sc)(status.clone(), video_ref.clone(), canvas_ref.clone(), blocks.clone(), facing);
        })
    };

    let flip_camera = {
        let facing_mode = facing_mode.clone();
        let status = status.clone();
        let video_ref = video_ref.clone();
        let canvas_ref = canvas_ref.clone();
        let blocks = blocks.clone();
        let sc = start_scan_now.clone();
        Callback::from(move |_| {
            let new_mode = if *facing_mode == "environment" { "user" } else { "environment" };
            facing_mode.set(new_mode.to_string());
            // auto-restart scanning with new mode
            (*sc)(status.clone(), video_ref.clone(), canvas_ref.clone(), blocks.clone(), new_mode.to_string());
        })
    };

    html! {
        <>
        <div style="display:none">
            <video id="video" ref={video_ref} playsinline=true></video>
            <canvas id="canvas" ref={canvas_ref}></canvas>
        </div>
    <div class="panel" style="position:relative; z-index:2; margin:12px; padding:12px; background:#ffffffcc; border:1px solid #e5e7eb; border-radius:8px;">
            <div class="row" style="display:flex; gap:8px; align-items:center; flex-wrap:wrap;">
                <button class="btn primary" onclick={start_scan_button.clone()}> {"Start Scan"} </button>
                <button class="btn" onclick={stop_stream.clone()}> {"Stop"} </button>
                <button class="btn" onclick={flip_camera}> {"Flip Camera"} </button>
                <span class="muted" style="margin-left:8px">{ format!("{} ({})", (*status).clone(), (*facing_mode).clone()) }</span>
            </div>
            <ul id="blocks" style="margin-top:8px;">
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
