use std::collections::HashSet;

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::calendar::CalendarModal;
use crate::components::layout::Navbar;
use crate::components::settings::SettingsModal;
use crate::components::story_card::StoryCard;
use crate::components::toast::Toast;
use crate::models::Story;
use crate::server_fns::config::set_lang;
use crate::server_fns::episodes::{
    get_episode_by_date, get_episodes, get_latest_episode,
};
use crate::server_fns::fetch::{get_fetch_status, start_fetch};
use crate::server_fns::stories::regenerate_summary;

/// Main home page component
#[component]
pub fn HomePage() -> impl IntoView {
    // Theme state
    let (is_dark, set_is_dark) = signal(false);
    // Language state
    let (lang, set_lang_signal) = signal("en".to_string());
    // Show only unread stories
    let (show_only_unread, set_show_only_unread) = signal(false);
    // Currently selected date
    let (selected_date, set_selected_date) = signal(None::<String>);
    // Read stories (client-side)
    let (read_stories, set_read_stories) = signal(HashSet::<i64>::new());
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

    // Current episode data
    let episode_resource = Resource::new(
        move || selected_date.get(),
        move |date| async move {
            match date {
                Some(d) => get_episode_by_date(d).await.ok().flatten(),
                None => get_latest_episode().await.ok().flatten(),
            }
        },
    );

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

    // Theme toggle
    let toggle_theme = move || {
        let new_dark = !is_dark.get();
        set_is_dark.set(new_dark);
        let doc = document();
        if let Some(html) = doc.document_element() {
            let _ = html.set_attribute("data-theme", if new_dark { "dark" } else { "light" });
        }
    };

    // Language toggle
    let toggle_lang = move || {
        let new_lang = if lang.get() == "en" { "zh" } else { "en" };
        let new_lang = new_lang.to_string();
        set_lang_signal.set(new_lang.clone());
        let lang_clone = new_lang.clone();
        spawn_local(async move {
            let _ = set_lang(lang_clone).await;
        });
    };

    // Refresh / fetch stories
    let do_fetch = move || {
        if is_fetching.get() {
            return;
        }
        set_is_fetching.set(true);
        show_toast(
            "Fetching stories from Hacker News...".to_string(),
            "loading".to_string(),
        );

        let lang_val = lang.get();
        spawn_local(async move {
            match start_fetch(lang_val).await {
                Ok(fetch_id) => {
                    // Start polling for status using set_timeout (cross-platform)
                    poll_fetch_status(
                        fetch_id,
                        set_is_fetching,
                        show_toast,
                        episode_resource,
                        episodes_resource,
                    );
                }
                Err(e) => {
                    set_is_fetching.set(false);
                    show_toast(format!("Error: {}", e), "error".to_string());
                }
            }
        });
    };

    // Mark story as read
    let mark_read = move |hn_id: i64| {
        let mut reads = read_stories.get();
        reads.insert(hn_id);
        set_read_stories.set(reads);
    };

    // Regenerate summary
    let do_regenerate = move |hn_id: i64| {
        let lang_val = lang.get();
        spawn_local(async move {
            match regenerate_summary(hn_id, lang_val).await {
                Ok(_) => {
                    episode_resource.refetch();
                }
                Err(e) => {
                    show_toast(format!("Error: {}", e), "error".to_string());
                }
            }
        });
    };

    // Get current stories from resource
    let current_stories = move || {
        episode_resource
            .get()
            .flatten()
            .map(|d| d.stories)
            .unwrap_or_default()
    };

    // Filter and sort stories
    let display_stories = move || {
        let stories = current_stories();
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
            on_toggle_read=Callback::new(move |_| set_show_only_unread.update(|v| *v = !*v))
            on_calendar=Callback::new(move |_| set_calendar_open.set(true))
            on_toggle_lang=Callback::new(move |_| toggle_lang())
            on_toggle_theme=Callback::new(move |_| toggle_theme())
            on_settings=Callback::new(move |_| set_settings_open.set(true))
            lang=lang
            is_dark=is_dark
        />

        <main class="main-container">
            <div class="content-area">
                <div class="stories-container">
                    <Suspense fallback=|| view! {
                        <div class="empty-state">
                            <div class="loading-spinner"></div>
                            <p>"Loading..."</p>
                        </div>
                    }>
                        {move || {
                            let stories = display_stories();
                            let reads = read_stories.get();
                            let lang_val = lang.get();

                            if stories.is_empty() {
                                view! {
                                    <div class="empty-state">
                                        <svg class="empty-icon" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                                            <path d="M4 4h16v16H4z" />
                                            <path d="M9 9h6M9 12h6M9 15h4" />
                                        </svg>
                                        <p>"No episode loaded"</p>
                                        <p style="font-size: 13px; margin-top: 8px">"Click refresh button or select a date from the calendar"</p>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <For
                                        each=move || stories.clone()
                                        key=|s: &Story| s.hn_id
                                        children=move |story: Story| {
                                            let is_read = reads.contains(&story.hn_id);
                                            let lang_clone = lang_val.clone();
                                            let idx = 0; // index not available in Leptos 0.8 For
                                            view! {
                                                <StoryCard
                                                    story=story
                                                    index=idx
                                                    is_read=is_read
                                                    lang=lang_clone
                                                    on_mark_read=Callback::new(move |id| mark_read(id))
                                                    on_regenerate=Callback::new(move |id| do_regenerate(id))
                                                />
                                            }
                                        }
                                    />
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

/// Poll fetch status using set_timeout (works in both SSR and WASM)
fn poll_fetch_status(
    fetch_id: String,
    set_is_fetching: WriteSignal<bool>,
    show_toast: impl Fn(String, String) + 'static,
    episode_resource: Resource<Option<crate::models::EpisodeWithStories>>,
    episodes_resource: Resource<Vec<crate::models::Episode>>,
) {
    spawn_local(async move {
        match get_fetch_status(fetch_id.clone()).await {
            Ok(progress) => {
                if progress.finished {
                    set_is_fetching.set(false);
                    show_toast(
                        format!("Fetched {} stories", progress.summaries_done),
                        "success".to_string(),
                    );
                    episode_resource.refetch();
                    episodes_resource.refetch();
                } else {
                    // Continue polling after 2 seconds
                    set_timeout(
                        move || {
                            poll_fetch_status(
                                fetch_id,
                                set_is_fetching,
                                show_toast,
                                episode_resource,
                                episodes_resource,
                            );
                        },
                        std::time::Duration::from_secs(2),
                    );
                }
            }
            Err(_) => {
                set_is_fetching.set(false);
            }
        }
    });
}
