use crate::components::icons;
use leptos::prelude::*;

#[component]
pub fn Navbar(
    is_dark: ReadSignal<bool>,
    #[prop(default = false)] minimal: bool,
    #[prop(default = signal(false).0)] is_fetching: ReadSignal<bool>,
    #[prop(default = signal(false).0)] show_only_unread: ReadSignal<bool>,
    #[prop(default = Callback::new(|_| {}))] on_refresh: Callback<()>,
    #[prop(default = Callback::new(|_| {}))] on_toggle_read: Callback<()>,
    #[prop(default = Callback::new(|_| {}))] on_calendar: Callback<()>,
    #[prop(default = Callback::new(|_| {}))] on_favorites: Callback<()>,
    on_toggle_theme: Callback<()>,
    #[prop(default = Callback::new(|_| {}))] on_settings: Callback<()>,
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
                <a href="/" class="nav-title">
                    <span class="icon" inner_html=icons::NAV_LOGO></span>
                    "Hacker News"
                </a>
            </div>
            <div class="navbar-right">
                {move || (!minimal).then(|| view! {
                    <>
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

                        <button class="icon-btn" title="收藏" on:click=move |_| on_favorites.run(())>
                            <span class="icon" inner_html=icons::BOOKMARK></span>
                        </button>
                    </>
                })}

                <button class="icon-btn" title="切换主题" on:click=move |_| on_toggle_theme.run(())>
                    <span class="icon" inner_html=move || if is_dark.get() { icons::SUN } else { icons::MOON }></span>
                </button>

                {move || (!minimal).then(|| view! {
                    <button class="icon-btn" title="设置" on:click=move |_| on_settings.run(())>
                        <span class="icon" inner_html=icons::SETTINGS></span>
                    </button>
                })}

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
