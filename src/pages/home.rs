use std::collections::HashSet;

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::calendar::CalendarModal;
use crate::components::layout::Navbar;
use crate::components::settings::SettingsModal;
use crate::components::story_card::StoryCard;
use crate::components::toast::Toast;
use crate::models::Story;
use crate::server_fns::episodes::{
    get_episode_by_date, get_episodes, get_latest_episode,
};
use crate::server_fns::fetch::start_fetch;
use crate::server_fns::stories::regenerate_summary;

#[cfg(not(feature = "ssr"))]
use crate::server_fns::stories::get_story;

/// Main home page component
#[component]
pub fn HomePage() -> impl IntoView {
    // Theme state — persist in localStorage (only on client)
    let initial_dark = if !leptos::prelude::is_server() {
        leptos::prelude::window()
            .local_storage()
            .ok()
            .flatten()
            .and_then(|ls| ls.get_item("hns-theme").ok().flatten())
            .map(|v| v == "dark")
            .unwrap_or(false)
    } else {
        false
    };
    let (is_dark, set_is_dark) = signal(initial_dark);

    // Apply theme on mount (client only)
    if initial_dark {
        if let Some(html) = document().document_element() {
            let _ = html.set_attribute("data-theme", "dark");
        }
    }
    // Show only unread stories
    let (show_only_unread, set_show_only_unread) = signal(false);
    // Read stories (client-side)
    let (read_stories, set_read_stories) = signal(HashSet::<i64>::new());

    // On client mount, restore persisted state from localStorage
    // Using Effect ensures this runs after hydration is complete
    Effect::new(move |_| {
        if leptos::prelude::is_server() {
            return;
        }

        let ls = leptos::prelude::window()
            .local_storage()
            .ok()
            .flatten();

        if let Some(ls) = ls {
            // Restore show_only_unread
            if let Some(val) = ls.get_item("hns-show-unread").ok().flatten() {
                if val == "true" {
                    set_show_only_unread.set(true);
                }
            }

            // Restore read_stories
            if let Some(json) = ls.get_item("hns-read-stories").ok().flatten() {
                if let Ok(reads) = serde_json::from_str::<HashSet<i64>>(&json) {
                    set_read_stories.set(reads);
                }
            }
        }
    });
    // Currently selected date
    let (selected_date, set_selected_date) = signal(None::<String>);
    // Calendar modal
    let (calendar_open, set_calendar_open) = signal(false);
    // Settings modal
    let (settings_open, set_settings_open) = signal(false);
    // Fetching state
    let (is_fetching, set_is_fetching) = signal(false);
    // Toast
    let (toast_msg, set_toast_msg) = signal(String::new());
    let (toast_type, set_toast_type) = signal(String::new());
    let (toast_visible, set_toast_visible) = signal(false);

    // Signal-based story list — updated incrementally via SSE, no full refetch
    let (stories_signal, set_stories) = signal(Vec::<Story>::new());

    // Current episode data (used for initial load and date changes)
    let episode_resource = Resource::new(
        move || selected_date.get(),
        move |date| async move {
            match date {
                Some(d) => get_episode_by_date(d).await.ok().flatten(),
                None => get_latest_episode().await.ok().flatten(),
            }
        },
    );

    // When episode resource resolves, update the local stories signal
    Effect::new(move |_| {
        if let Some(data) = episode_resource.get().flatten() {
            set_stories.set(data.stories);
        }
    });

    // Episodes list for calendar
    let episodes_resource = Resource::new(
        || (),
        move |_| async move { get_episodes().await.ok().unwrap_or_default() },
    );

    // Toast helper
    let show_toast = move |msg: String, t: String| {
        set_toast_msg.set(msg);
        set_toast_type.set(t);
        set_toast_visible.set(true);
        set_timeout(
            move || set_toast_visible.set(false),
            std::time::Duration::from_secs(3),
        );
    };

    // Theme toggle — persist to localStorage (client only)
    let toggle_theme = move || {
        let new_dark = !is_dark.get();
        set_is_dark.set(new_dark);
        let theme = if new_dark { "dark" } else { "light" };
        if !leptos::prelude::is_server() {
            if let Some(html) = document().document_element() {
                let _ = html.set_attribute("data-theme", theme);
            }
            if let Some(ls) = leptos::prelude::window().local_storage().ok().flatten() {
                let _ = ls.set_item("hns-theme", theme);
            }
        }
    };

    // Refresh / fetch stories
    let do_fetch = move || {
        if is_fetching.get() {
            return;
        }
        set_is_fetching.set(true);
        show_toast(
            "正在从 Hacker News 获取故事...".to_string(),
            "loading".to_string(),
        );

        let episode_resource = episode_resource;
        let episodes_resource = episodes_resource;
        let set_is_fetching = set_is_fetching;
        let set_stories = set_stories;
        let show_toast = show_toast;

        spawn_local(async move {
            match start_fetch().await {
                Ok(_fetch_id) => {
                    // Connect to SSE for real-time updates
                    listen_fetch_events(
                        episode_resource,
                        episodes_resource,
                        set_is_fetching,
                        set_stories,
                        show_toast,
                    );
                }
                Err(e) => {
                    set_is_fetching.set(false);
                    show_toast(format!("错误: {}", e), "error".to_string());
                }
            }
        });
    };

    // Mark story as read — persist to localStorage
    let mark_read = move |hn_id: i64| {
        let mut reads = read_stories.get();
        reads.insert(hn_id);
        set_read_stories.set(reads.clone());
        if !leptos::prelude::is_server() {
            if let Some(ls) = leptos::prelude::window().local_storage().ok().flatten() {
                if let Ok(json) = serde_json::to_string(&reads) {
                    let _ = ls.set_item("hns-read-stories", &json);
                }
            }
        }
    };

    // Regenerate summary — update only the affected story
    let do_regenerate = move |hn_id: i64| {
        let set_stories = set_stories;
        spawn_local(async move {
            match regenerate_summary(hn_id).await {
                Ok(updated) => {
                    // Update only this story in the signal
                    set_stories.update(|stories| {
                        if let Some(s) = stories.iter_mut().find(|s| s.hn_id == hn_id) {
                            *s = updated;
                        }
                    });
                }
                Err(e) => {
                    show_toast(format!("错误: {}", e), "error".to_string());
                }
            }
        });
    };

    // Auto-fetch on page load (client only, runs once)
    {
        let fetched = std::rc::Rc::new(std::cell::Cell::new(false));
        Effect::new(move |_| {
            if !leptos::prelude::is_server() && !fetched.get() {
                fetched.set(true);
                do_fetch();
            }
        });
    }

    // Filter and sort stories from the local signal
    let display_stories = move || {
        let stories = stories_signal.get();
        let reads = read_stories.get();
        let show_unread = show_only_unread.get();

        let unclicked: Vec<Story> = stories
            .iter()
            .filter(|s| !reads.contains(&s.hn_id))
            .cloned()
            .collect();
        let clicked: Vec<Story> = stories
            .iter()
            .filter(|s| reads.contains(&s.hn_id))
            .cloned()
            .collect();

        if show_unread {
            unclicked
        } else {
            let mut result = unclicked;
            result.extend(clicked);
            result
        }
    };

    let episodes_list = move || {
        episodes_resource.get().unwrap_or_default()
    };

    view! {
        <Navbar
            is_fetching=is_fetching
            show_only_unread=show_only_unread
            on_refresh=Callback::new(move |_| do_fetch())
            on_toggle_read=Callback::new(move |_| {
                let new_val = !show_only_unread.get();
                set_show_only_unread.set(new_val);
                if !leptos::prelude::is_server() {
                    if let Some(ls) = leptos::prelude::window().local_storage().ok().flatten() {
                        let _ = ls.set_item("hns-show-unread", &new_val.to_string());
                    }
                }
            })
            on_calendar=Callback::new(move |_| set_calendar_open.set(true))
            on_toggle_theme=Callback::new(move |_| toggle_theme())
            on_settings=Callback::new(move |_| set_settings_open.set(true))
            is_dark=is_dark
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
                            let stories = display_stories();

                            if stories.is_empty() {
                                view! {
                                    <div class="empty-state">
                                        <svg class="empty-icon" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                                            <path d="M4 4h16v16H4z" />
                                            <path d="M9 9h6M9 12h6M9 15h4" />
                                        </svg>
                                        <p>"暂无内容"</p>
                                        <p style="font-size: 13px; margin-top: 8px">"点击刷新按钮或从日历选择日期"</p>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div>
                                        <For
                                            each=move || display_stories()
                                            key=|story| story.hn_id
                                            children=move |story| {
                                                // Compute index from the current display list
                                                let idx = display_stories()
                                                    .iter()
                                                    .position(|s| s.hn_id == story.hn_id)
                                                    .unwrap_or(0);
                                                view! {
                                                    <StoryCard
                                                        story=story
                                                        index=idx
                                                        read_stories=read_stories
                                                        on_mark_read=Callback::new(move |id| mark_read(id))
                                                        on_regenerate=Callback::new(move |id| do_regenerate(id))
                                                    />
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

        <CalendarModal
            is_open=calendar_open
            episodes=episodes_list()
            selected_date=selected_date.get()
            on_select_date=Callback::new(move |date: String| {
                set_selected_date.set(Some(date));
                set_calendar_open.set(false);
            })
            on_close=Callback::new(move |_| set_calendar_open.set(false))
        />

        <SettingsModal
            is_open=settings_open
            on_close=Callback::new(move |_| set_settings_open.set(false))
            on_data_changed=Callback::new(move |_| {
                episode_resource.refetch();
                episodes_resource.refetch();
            })
        />

        <Toast
            message=toast_msg
            toast_type=toast_type
            visible=toast_visible
        />
    }
}

/// Listen to SSE fetch events for real-time UI updates (client only)
/// Uses incremental updates — only fetches the changed story, not the whole list
#[cfg(not(feature = "ssr"))]
fn listen_fetch_events(
    episode_resource: Resource<Option<crate::models::EpisodeWithStories>>,
    episodes_resource: Resource<Vec<crate::models::Episode>>,
    set_is_fetching: WriteSignal<bool>,
    set_stories: WriteSignal<Vec<crate::models::Story>>,
    show_toast: impl Fn(String, String) + 'static,
) {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    let es = web_sys::EventSource::new("/api/fetch-events").unwrap();

    let on_message = {
        let es = es.clone();
        Closure::<dyn Fn(web_sys::MessageEvent)>::new(move |e: web_sys::MessageEvent| {
            let data = e.data().as_string().unwrap_or_default();
            if let Ok(event) = serde_json::from_str::<crate::models::FetchEvent>(&data) {
                match event {
                    crate::models::FetchEvent::StoryAdded { hn_id } => {
                        // Fetch only the new story and append it
                        let set_stories = set_stories;
                        spawn_local(async move {
                            if let Ok(Some(story)) = get_story(hn_id).await {
                                set_stories.update(|stories| {
                                    // Avoid duplicate
                                    if !stories.iter().any(|s| s.hn_id == hn_id) {
                                        stories.push(story);
                                    }
                                });
                            }
                        });
                    }
                    crate::models::FetchEvent::SummaryDone { hn_id } => {
                        // Fetch only the updated story and replace it in-place
                        let set_stories = set_stories;
                        spawn_local(async move {
                            if let Ok(Some(story)) = get_story(hn_id).await {
                                set_stories.update(|stories| {
                                    if let Some(s) = stories.iter_mut().find(|s| s.hn_id == hn_id) {
                                        *s = story;
                                    }
                                });
                            }
                        });
                    }
                    crate::models::FetchEvent::SummaryError { hn_id } => {
                        // Just log to console, no UI update needed for errors
                        web_sys::console::log_1(&format!("Summary generation failed for story {}", hn_id).into());
                    }
                    crate::models::FetchEvent::Finished { summaries, .. } => {
                        set_is_fetching.set(false);
                        // Final refetch to ensure consistency
                        episode_resource.refetch();
                        episodes_resource.refetch();
                        show_toast(
                            format!("已获取 {} 篇故事", summaries),
                            "success".to_string(),
                        );
                        // Close the EventSource after finished
                        es.close();
                    }
                }
            }
        })
    };

    let on_error = {
        let es = es.clone();
        Closure::<dyn Fn(web_sys::Event)>::new(move |_: web_sys::Event| {
            if es.ready_state() == web_sys::EventSource::CLOSED {
                set_is_fetching.set(false);
            }
        })
    };

    es.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();
    es.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    on_error.forget();
}

/// SSR stub — SSE listening only works on the client
#[cfg(feature = "ssr")]
fn listen_fetch_events(
    _episode_resource: Resource<Option<crate::models::EpisodeWithStories>>,
    _episodes_resource: Resource<Vec<crate::models::Episode>>,
    _set_is_fetching: WriteSignal<bool>,
    _set_stories: WriteSignal<Vec<crate::models::Story>>,
    _show_toast: impl Fn(String, String) + 'static,
) {
    // No-op on server
}
