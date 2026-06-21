use crate::app::App;
use leptos::prelude::*;
use leptos_meta::MetaTags;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1.0"/>
                <title>"Hacker News"</title>
                <link rel="icon" type="image/svg+xml" href="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'%3E%3Crect width='32' height='32' fill='%23ff6600'/%3E%3Ctext x='16' y='23' font-size='18' font-family='Arial,sans-serif' font-weight='bold' fill='white' text-anchor='middle'%3EHN%3C/text%3E%3C/svg%3E"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
                <link rel="stylesheet" href="/style.css"/>
                <script src="https://cdn.jsdelivr.net/npm/marked/marked.min.js"></script>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}
