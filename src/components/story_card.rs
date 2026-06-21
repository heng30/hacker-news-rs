use leptos::prelude::*;

use crate::models::Story;

/// Render a single story card
#[component]
pub fn StoryCard(
    story: Story,
    index: usize,
    is_read: bool,
    lang: String,
    on_mark_read: Callback<i64>,
    on_regenerate: Callback<i64>,
) -> impl IntoView {
    let clicked_class = if is_read { "story clicked" } else { "story" };
    let hn_id = story.hn_id;
    let title = story.title.clone();
    let url = story.url.clone().unwrap_or_default();
    let by = story.by.clone();
    let score = story.score;
    let tag = story.tag.clone();
    let summary = story.summary.clone();
    let summary_zh = story.summary_zh.clone();

    let has_summary = match lang.as_str() {
        "zh" => summary_zh.is_some(),
        _ => summary.is_some(),
    };

    let summary_text = match lang.as_str() {
        "zh" => summary_zh.clone(),
        _ => summary.clone(),
    };

    let display_summary = summary_text.unwrap_or_default();

    view! {
        <div class=clicked_class>
            <div class="story-title">
                <a href=url target="_blank" on:click=move |_| on_mark_read.run(hn_id)>
                    {format!("{}. {}", index + 1, title)}
                </a>
                <div class="story-actions">
                    <button
                        class="mark-read-btn"
                        on:click=move |_| on_mark_read.run(hn_id)
                        disabled=is_read
                        title="Mark as read"
                    >
                        {if is_read {
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
                        title="Regenerate summary"
                    >
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M21 12a9 9 0 11-2.636-6.364"/>
                            <path d="M21 3v6h-6"/>
                        </svg>
                    </button>
                </div>
            </div>
            <div class="story-meta">
                {format!("{} points by {}", score, by)}
                {if tag != "top" {
                    view! { <span class="story-tag">{format!(" | {}", tag)}</span> }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
                {format!(" | ")}
                <a href=format!("https://news.ycombinator.com/item?id={}", hn_id) target="_blank">"discuss"</a>
            </div>
            {if has_summary && !display_summary.is_empty() {
                view! {
                    <div class="story-summary" inner_html=display_summary></div>
                }.into_any()
            } else if !has_summary {
                view! {
                    <div class="story-summary placeholder">
                        {if lang == "zh" { "生成摘要中..." } else { "Generating summary..." }}
                    </div>
                }.into_any()
            } else {
                view! { <div></div> }.into_any()
            }}
        </div>
    }
}
