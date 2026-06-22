use leptos::prelude::*;

#[component]
pub fn Toast(
    message: ReadSignal<String>,
    toast_type: ReadSignal<String>,
    visible: ReadSignal<bool>,
) -> impl IntoView {
    let class = move || {
        let base = "toast";
        let show = if visible.get() { " show" } else { "" };
        let t = toast_type.get();
        format!("{}{} {}", base, show, t)
    };

    view! {
        <div id="toast" class=class>
            {move || message.get()}
        </div>
    }
}
