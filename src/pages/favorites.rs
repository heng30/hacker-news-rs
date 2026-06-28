use crate::{
    components::{icons, layout::Navbar, toast::Toast},
    models::Story,
    server_fns::{
        preferences::{get_favorite_stories, get_user_preferences, toggle_story_favorite},
        stories::regenerate_summary,
    },
};
use leptos::prelude::*;
use leptos::task::spawn_local;

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
pub fn FavoritesPage() -> impl IntoView {
    let (is_dark, set_is_dark) = signal(false);
    let (stories_signal, set_stories) = signal(Vec::<Story>::new());
    let (toast_msg, set_toast_msg) = signal(String::new());
    let (toast_type, set_toast_type) = signal(String::new());
    let (toast_visible, set_toast_visible) = signal(false);

    // Load user preferences (theme, favorites)
    let prefs_resource = Resource::new(
        || (),
        move |_| async move { get_user_preferences().await.ok() },
    );

    Effect::new(move |_| {
        if let Some(prefs) = prefs_resource.get().flatten() {
            let is_dark_new = prefs.theme == "dark";
            set_is_dark.set(is_dark_new);

            if !is_server()
                && is_dark_new
                && let Some(html) = document().document_element()
            {
                _ = html.set_attribute("data-theme", "dark");
            }
        }
    });

    // Load favorite stories with details
    let resource = Resource::new(
        || (),
        move |_| async move { get_favorite_stories().await.ok().unwrap_or_default() },
    );

    Effect::new(move |_| {
        if let Some(data) = resource.get() {
            set_stories.set(data);
        }
    });

    let show_toast = move |msg: String, t: String| {
        set_toast_msg.set(msg);
        set_toast_type.set(t);
        set_toast_visible.set(true);
        set_timeout(
            move || set_toast_visible.set(false),
            std::time::Duration::from_secs(3),
        );
    };

    let do_unfavorite = move |hn_id: i64| {
        spawn_local(async move {
            match toggle_story_favorite(hn_id).await {
                Ok(_) => {
                    set_stories.update(|s| s.retain(|story| story.hn_id != hn_id));
                    show_toast("已取消收藏".to_string(), "success".to_string());
                }
                Err(e) => show_toast(format!("错误: {}", e), "error".to_string()),
            }
        });
    };

    let do_regenerate = move |hn_id: i64| {
        spawn_local(async move {
            match regenerate_summary(hn_id).await {
                Ok(updated) => {
                    set_stories.update(|stories| {
                        if let Some(s) = stories.iter_mut().find(|s| s.hn_id == hn_id) {
                            *s = updated;
                        }
                    });
                }
                Err(e) => show_toast(format!("错误: {}", e), "error".to_string()),
            }
        });
    };

    let do_copy = move |msg: String| {
        show_toast(msg, "success".to_string());
    };

    let toggle_theme = move || {
        let new_dark = !is_dark.get();
        set_is_dark.set(new_dark);
        let theme = if new_dark { "dark" } else { "light" };

        if !is_server()
            && let Some(html) = document().document_element()
        {
            _ = html.set_attribute("data-theme", theme);
        }

        spawn_local(async move {
            _ = crate::server_fns::preferences::set_theme(theme.to_string()).await;
        });
    };

    view! {
        <Navbar
            is_dark=is_dark
            minimal=true
            on_toggle_theme=Callback::new(move |_| toggle_theme())
        />

        <main class="main-container">
            <div class="content-area">
                <div class="stories-container">
                    <Suspense fallback=|| view! {
                        <div class="empty-state">
                            <div class="loading-spinner"></div>
                            <p>"加载中..."</p>
                        </div>
                    }>
                        {move || {
                            let stories = stories_signal.get();
                            if stories.is_empty() {
                                view! {
                                    <div class="empty-state">
                                        <span class="icon" inner_html=icons::HEART></span>
                                        <p>"暂无收藏"</p>
                                        <p style="font-size: 13px; margin-top: 8px">"在首页点击心形按钮收藏故事"</p>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div>
                                        <For
                                            each=move || stories_signal.get()
                                            key=|story| story.hn_id
                                            children=move |story| {
                                                let hn_id = story.hn_id;
                                                let (story_sig, _) = signal(story);
                                                let title = move || story_sig.get().title;
                                                let url = move || story_sig.get().url.clone().unwrap_or_default();
                                                let by = move || story_sig.get().by;
                                                let score = move || story_sig.get().score;
                                                let tag = move || story_sig.get().tag;
                                                let summary = move || story_sig.get().summary.clone();
                                                let (copied, set_copied) = signal(false);

                                                let idx = stories_signal.get()
                                                    .iter()
                                                    .position(|s| s.hn_id == hn_id)
                                                    .unwrap_or(0);

                                                view! {
                                                    <div class="story">
                                                        <div class="story-title">
                                                            <a href=url target="_blank">
                                                                {format!("{}. ", idx + 1)}{title}
                                                            </a>
                                                            <div class="story-actions">
                                                                <button
                                                                    class="regenerate-btn"
                                                                    on:click=move |_| do_regenerate(hn_id)
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
                                                                        let do_copy = do_copy;
                                                                        spawn_local(async move {
                                                                            if copy_to_clipboard(&text).await {
                                                                                set_copied.set(true);
                                                                                set_timeout(move || set_copied.set(false), std::time::Duration::from_secs(2));
                                                                                do_copy("已复制到剪贴板".to_string());
                                                                            } else {
                                                                                do_copy("复制失败".to_string());
                                                                            }
                                                                        });
                                                                    }
                                                                    title="复制摘要"
                                                                >
                                                                    <span class="icon" inner_html=move || if copied.get() { icons::COPY_CHECK } else { icons::COPY }></span>
                                                                </button>
                                                                <button
                                                                    class="unfavorite-btn"
                                                                    on:click=move |_| do_unfavorite(hn_id)
                                                                    title="取消收藏"
                                                                >
                                                                    <span class="icon" inner_html=icons::HEART_FILLED></span>
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
                                                            let s = summary();
                                                            let has_summary = s.is_some();
                                                            let display_summary = s.unwrap_or_default();

                                                            if has_summary && !display_summary.is_empty() {
                                                                let html = if !is_server() {
                                                                    marked_parse(&display_summary)
                                                                } else {
                                                                    display_summary
                                                                };
                                                                view! {
                                                                    <div class="story-summary markdown-body" inner_html=html></div>
                                                                }.into_any()
                                                            } else if !has_summary {
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
                                        />
                                    </div>
                                }.into_any()
                            }
                        }}
                    </Suspense>
                </div>
            </div>
        </main>

        <Toast
            message=toast_msg.into()
            toast_type=toast_type.into()
            visible=toast_visible.into()
        />
    }
}
