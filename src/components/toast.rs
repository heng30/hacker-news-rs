use leptos::prelude::*;

#[component]
pub fn Toast(
    message: Signal<String>,
    toast_type: Signal<String>,
    visible: Signal<bool>,
) -> impl IntoView {
    view! {
        <div
            id="toast"
            class="toast"
            class:show=move || visible.get()
            class:success=move || toast_type.get() == "success"
            class:error=move || toast_type.get() == "error"
            class:loading=move || toast_type.get() == "loading"
        >
            {move || message.get()}
        </div>
    }
}
