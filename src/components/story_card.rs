use crate::{components::icons, models::Story};
use leptos::{prelude::*, task::spawn_local};

/// Parse Markdown to HTML using marked.js (client only)
#[cfg(not(feature = "ssr"))]
fn marked_parse(md: &str) -> String {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = marked, js_name = parse)]
        fn marked_parse_js(source: &str) -> String;
    }

    marked_parse_js(md)
}

#[cfg(feature = "ssr")]
fn marked_parse(md: &str) -> String {
    md.to_string()
}

#[cfg(not(feature = "ssr"))]
async fn copy_to_clipboard(text: &str) -> bool {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().unwrap();
    let navigator = js_sys::Reflect::get(&window, &JsValue::from_str("navigator")).ok();
    let Some(navigator) = navigator else {
        return false;
    };
    let clipboard = js_sys::Reflect::get(&navigator, &JsValue::from_str("clipboard")).ok();
    let Some(clipboard) = clipboard else {
        return false;
    };
    let write_text = js_sys::Reflect::get(&clipboard, &JsValue::from_str("writeText")).ok();
    let Some(write_text) = write_text else {
        return false;
    };

    let promise = js_sys::Function::from(write_text)
        .call1(&clipboard, &JsValue::from_str(text))
        .ok();
    let Some(promise) = promise else { return false };

    JsFuture::from(js_sys::Promise::from(promise)).await.is_ok()
}

#[cfg(feature = "ssr")]
async fn copy_to_clipboard(_text: &str) -> bool {
    false
}

#[component]
pub fn StoryCard(
    story: Story,
    index: usize,
    read_stories: ReadSignal<std::collections::HashSet<i64>>,
    favorite_stories: ReadSignal<std::collections::HashSet<i64>>,
    on_mark_read: Callback<i64>,
    on_regenerate: Callback<i64>,
    on_copy: Callback<String>,
    on_toggle_favorite: Callback<i64>,
) -> impl IntoView {
    let hn_id = story.hn_id;
    let (story_sig, _) = signal(story);
    let title = move || story_sig.get().title;
    let url = move || story_sig.get().url.clone().unwrap_or_default();
    let by = move || story_sig.get().by;
    let score = move || story_sig.get().score;
    let tag = move || story_sig.get().tag;
    let summary = move || story_sig.get().summary.clone();
    let (copied, set_copied) = signal(false);

    view! {
        <div class=move || if read_stories.get().contains(&hn_id) { "story clicked" } else { "story" }>
            <div class="story-title">
                <a href=url target="_blank" on:click=move |_| on_mark_read.run(hn_id)>
                    {format!("{}. ", index + 1)}{title}
                </a>
                <div class="story-actions">
                    <button
                        class="mark-read-btn"
                        on:click=move |_| on_mark_read.run(hn_id)
                        disabled=move || read_stories.get().contains(&hn_id)
                        title="标为已读"
                    >
                        <span class="icon" inner_html=move || if read_stories.get().contains(&hn_id) { icons::CHECK } else { icons::BOOK }></span>
                    </button>
                    <button
                        class="regenerate-btn"
                        on:click=move |_| on_regenerate.run(hn_id)
                        title="重新生成摘要"
                    >
                        <span class="icon" inner_html=icons::REFRESH_SMALL></span>
                    </button>
                    <button
                        class="copy-btn"
                        on:click=move |_| {
                            let s = summary().unwrap_or_default();
                            let t = title();
                            let text = if s.is_empty() { t } else { format!("{}\n\n{}", t, s) };
                            let set_copied = set_copied;
                            let on_copy = on_copy;
                            spawn_local(async move {
                                if copy_to_clipboard(&text).await {
                                    set_copied.set(true);
                                    set_timeout(move || set_copied.set(false), std::time::Duration::from_secs(2));
                                    on_copy.run("已复制到剪贴板".to_string());
                                } else {
                                    on_copy.run("复制失败".to_string());
                                }
                            });
                        }
                        title="复制摘要"
                    >
                        <span class="icon" inner_html=move || if copied.get() { icons::COPY_CHECK } else { icons::COPY }></span>
                    </button>
                    <button
                        class=move || if favorite_stories.get().contains(&hn_id) { "favorite-btn active" } else { "favorite-btn" }
                        on:click=move |_| on_toggle_favorite.run(hn_id)
                        title=move || if favorite_stories.get().contains(&hn_id) { "取消收藏" } else { "收藏" }
                    >
                        <span class="icon" inner_html=move || if favorite_stories.get().contains(&hn_id) { icons::HEART_FILLED } else { icons::HEART }></span>
                    </button>
                </div>
            </div>
            <div class="story-meta">
                {move || format!("{} 分 by {}", score(), by())}
                {move || {
                    let t = tag();
                    if t != "top" {
                        view! { <span class="story-tag">{format!(" | {}", t)}</span> }.into_any()
                    } else {
                        view! { <span></span> }.into_any()
                    }
                }}
                {format!(" | ")}
                <a href=format!("https://news.ycombinator.com/item?id={}", hn_id) target="_blank">"讨论"</a>
            </div>
            {move || {
                let reads = read_stories.get();
                let s = summary();
                let has_summary = s.is_some();
                let display_summary = s.unwrap_or_default();
                let is_read = reads.contains(&hn_id);

                if has_summary && !display_summary.is_empty() {
                    // Render Markdown to HTML via marked.js (client only)
                    let html = if !is_server() {
                        marked_parse(&display_summary)
                    } else {
                        display_summary
                    };
                    view! {
                        <div class="story-summary markdown-body" inner_html=html></div>
                    }.into_any()
                } else if !has_summary && !is_read {
                    view! {
                        <div class="story-summary placeholder">
                            "生成摘要中..."
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}
        </div>
    }
}
