use crate::pages::{favorites::FavoritesPage, home::HomePage};
use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| view! { <p>"Not found"</p> }>
                <Route path=path!("/") view=HomePage/>
                <Route path=path!("/favorites") view=FavoritesPage/>
            </Routes>
        </Router>
    }
}
