use crate::{
    components::icons,
    server_fns::{
        config::get_config,
        stories::{delete_all_stories, delete_read_stories},
    },
};
use leptos::{prelude::*, task::spawn_local};

#[component]
pub fn SettingsModal(
    is_open: ReadSignal<bool>,
    on_close: Callback<()>,
    on_data_changed: Callback<()>,
) -> impl IntoView {
    let config_resource = Resource::new(
        move || is_open.get(),
        move |_| async move { get_config().await.ok() },
    );

    let overlay_class = move || {
        if is_open.get() {
            "modal-overlay active"
        } else {
            "modal-overlay"
        }
    };

    let handle_delete_all = move || {
        let on_data_changed = on_data_changed;
        spawn_local(async move {
            if let Ok(count) = delete_all_stories().await {
                on_data_changed.run(());
                leptos::logging::log!("Deleted {} stories", count);
            }
        });
    };

    let handle_delete_read = move || {
        let on_data_changed = on_data_changed;
        spawn_local(async move {
            if let Ok(count) = delete_read_stories(vec![]).await {
                on_data_changed.run(());
                leptos::logging::log!("Deleted {} read stories", count);
            }
        });
    };

    view! {
        <div class=overlay_class on:click=move |e| {
            if e.target() == e.current_target() {
                on_close.run(());
            }
        }>
            <div class="modal">
                <div class="modal-header">
                    <h2>
                        <span class="icon" inner_html=icons::SETTINGS_WITH_STYLE></span>
                        "Configuration"
                    </h2>
                    <button class="modal-close" on:click=move |_| on_close.run(())>"×"</button>
                </div>

                <Suspense fallback=|| view! { <div>"Loading..."</div> }>
                    {move || config_resource.get().map(|config| {
                        match config {
                            Some(c) => {
                                let api_base_url = c.api_base_url.clone();
                                let model = c.model.clone();
                                let masked_api_key = c.masked_api_key.clone();
                                let socks5 = c.socks5_proxy.clone().unwrap_or_else(|| "Disabled".to_string());
                                let keywords = c.search_keywords.clone().unwrap_or_else(|| "Disabled".to_string());
                                let auto_update = if c.auto_update_interval > 0 {
                                    format!("{} min", c.auto_update_interval)
                                } else {
                                    "Disabled".to_string()
                                };
                                view! {
                                    <>
                                        <div class="form-group">
                                            <label>"API Base URL"</label>
                                            <div class="env-badge"><span>{api_base_url}</span></div>
                                        </div>
                                        <div class="form-group">
                                            <label>"Model"</label>
                                            <div class="env-badge"><span>{model}</span></div>
                                        </div>
                                        <div class="form-group">
                                            <label>"API Key"</label>
                                            <div class="env-badge"><span>{masked_api_key}</span></div>
                                        </div>
                                        <div class="form-group">
                                            <label>"SOCKS5 Proxy"</label>
                                            <div class="env-badge"><span>{socks5}</span></div>
                                        </div>
                                        <div class="form-group">
                                            <label>"Search Keywords"</label>
                                            <div class="env-badge"><span>{keywords}</span></div>
                                        </div>
                                        <div class="form-group">
                                            <label>"Auto Update"</label>
                                            <div class="env-badge"><span>{auto_update}</span></div>
                                        </div>
                                    </>
                                }.into_any()
                            }
                            None => view! { <div>"Failed to load config"</div> }.into_any(),
                        }
                    })}
                </Suspense>

                <div class="modal-divider"></div>

                <div class="form-group data-management">
                    <label>"Data Management"</label>
                    <div class="modal-actions">
                        <button class="btn-secondary" on:click=move |_| handle_delete_read()>
                            "Remove all read stories"
                        </button>
                        <button class="btn-danger" on:click=move |_| handle_delete_all()>
                            "Remove all stories"
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}
