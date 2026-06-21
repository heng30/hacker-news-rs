use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

use crate::pages::home::HomePage;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| view! { <p>"Not found"</p> }>
                <Route path=path!("/") view=HomePage/>
            </Routes>
        </Router>
    }
}
