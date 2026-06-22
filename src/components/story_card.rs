use crate::models::Story;
use leptos::prelude::*;

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
    // SSR fallback: return raw markdown, client will re-render
    md.to_string()
}

#[component]
pub fn StoryCard(
    story: Story,
    index: usize,
    read_stories: ReadSignal<std::collections::HashSet<i64>>,
    on_mark_read: Callback<i64>,
    on_regenerate: Callback<i64>,
) -> impl IntoView {
    let hn_id = story.hn_id;
    let (story_sig, _) = signal(story);
    let title = move || story_sig.get().title;
    let url = move || story_sig.get().url.clone().unwrap_or_default();
    let by = move || story_sig.get().by;
    let score = move || story_sig.get().score;
    let tag = move || story_sig.get().tag;
    let summary = move || story_sig.get().summary.clone();

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
                        {move || if read_stories.get().contains(&hn_id) {
                            view! {
                                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <polyline points="20 6 9 17 4 12"/>
                                </svg>
                            }.into_any()
                        } else {
                            view! {
                                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/>
                                    <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/>
                                </svg>
                            }.into_any()
                        }}
                    </button>
                    <button
                        class="regenerate-btn"
                        on:click=move |_| on_regenerate.run(hn_id)
                        title="重新生成摘要"
                    >
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M21 12a9 9 0 11-2.636-6.364"/>
                            <path d="M21 3v6h-6"/>
                        </svg>
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
