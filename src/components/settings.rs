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
                        "设置"
                    </h2>
                    <button class="modal-close" on:click=move |_| on_close.run(())>"×"</button>
                </div>

                <Suspense fallback=|| view! { <div>"加载中..."</div> }>
                    {move || config_resource.get().map(|config| {
                        match config {
                            Some(c) => {
                                let api_base_url = c.api_base_url.clone();
                                let model = c.model.clone();
                                let masked_api_key = c.masked_api_key.clone();
                                let socks5 = c.socks5_proxy.clone().unwrap_or_else(|| "未启用".to_string());
                                let keywords = c.search_keywords.clone().unwrap_or_else(|| "未启用".to_string());
                                let auto_update = if c.auto_update_interval > 0 {
                                    format!("{} 分钟", c.auto_update_interval)
                                } else {
                                    "未启用".to_string()
                                };
                                view! {
                                    <>
                                        <div class="form-group">
                                            <label>"API 地址"</label>
                                            <div class="env-badge"><span>{api_base_url}</span></div>
                                        </div>
                                        <div class="form-group">
                                            <label>"模型"</label>
                                            <div class="env-badge"><span>{model}</span></div>
                                        </div>
                                        <div class="form-group">
                                            <label>"API 密钥"</label>
                                            <div class="env-badge"><span>{masked_api_key}</span></div>
                                        </div>
                                        <div class="form-group">
                                            <label>"SOCKS5 代理"</label>
                                            <div class="env-badge"><span>{socks5}</span></div>
                                        </div>
                                        <div class="form-group">
                                            <label>"搜索关键词"</label>
                                            <div class="env-badge"><span>{keywords}</span></div>
                                        </div>
                                        <div class="form-group">
                                            <label>"自动更新"</label>
                                            <div class="env-badge"><span>{auto_update}</span></div>
                                        </div>
                                    </>
                                }.into_any()
                            }
                            None => view! { <div>"加载配置失败"</div> }.into_any(),
                        }
                    })}
                </Suspense>

                <div class="modal-divider"></div>

                <div class="form-group data-management">
                    <label>"数据管理"</label>
                    <div class="modal-actions">
                        <button class="btn-secondary" on:click=move |_| handle_delete_read()>
                            "删除所有已读故事"
                        </button>
                        <button class="btn-danger" on:click=move |_| handle_delete_all()>
                            "删除所有故事"
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}
