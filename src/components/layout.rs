use crate::components::icons;
use leptos::prelude::*;

#[component]
pub fn Navbar(
    is_fetching: ReadSignal<bool>,
    show_only_unread: ReadSignal<bool>,
    on_refresh: Callback<()>,
    on_toggle_read: Callback<()>,
    on_calendar: Callback<()>,
    on_toggle_theme: Callback<()>,
    on_settings: Callback<()>,
    is_dark: ReadSignal<bool>,
) -> impl IntoView {
    let refresh_class = move || {
        if is_fetching.get() {
            "icon-btn loading"
        } else {
            "icon-btn"
        }
    };

    view! {
        <nav class="navbar">
            <div class="navbar-left">
                <h1>
                    <span class="icon" inner_html=icons::NAV_LOGO></span>
                    "Hacker News"
                </h1>
            </div>
            <div class="navbar-right">
                <button
                    class=refresh_class
                    title="刷新故事"
                    on:click=move |_| on_refresh.run(())
                    disabled=move || is_fetching.get()
                >
                    <span class="icon" inner_html=icons::REFRESH></span>
                </button>

                <button
                    class="icon-btn"
                    title=move || if show_only_unread.get() { "显示全部" } else { "仅显示未读" }
                    on:click=move |_| on_toggle_read.run(())
                >
                    <span class="icon" inner_html=move || if show_only_unread.get() { icons::EYE_OPEN } else { icons::EYE_CLOSED }></span>
                </button>

                <button class="icon-btn" title="日历" on:click=move |_| on_calendar.run(())>
                    <span class="icon" inner_html=icons::CALENDAR></span>
                </button>

                <button class="icon-btn" title="切换主题" on:click=move |_| on_toggle_theme.run(())>
                    <span class="icon" inner_html=move || if is_dark.get() { icons::SUN } else { icons::MOON }></span>
                </button>

                <button class="icon-btn" title="设置" on:click=move |_| on_settings.run(())>
                    <span class="icon" inner_html=icons::SETTINGS></span>
                </button>

                <a
                    href="https://github.com/heng30/hacker-news-rs"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="icon-btn"
                    title="GitHub"
                    style="text-decoration: none; display: flex; align-items: center; justify-content: center;"
                >
                    <span class="icon" inner_html=icons::GITHUB></span>
                </a>
            </div>
        </nav>
    }
}
