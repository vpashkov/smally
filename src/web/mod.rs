pub mod api_keys;
pub mod auth;
pub mod components;
pub mod dashboard;
pub mod organizations;

use axum::response::Redirect;
use maud::{html, Markup};

/// Home page - redirects to login if not authenticated, otherwise to dashboard
pub async fn home() -> Redirect {
    // For now, always redirect to login
    // TODO: Check session cookie and redirect to dashboard if authenticated
    Redirect::to("/login")
}

/// 404 Not Found page
pub async fn not_found() -> Markup {
    components::layout::base(
        "404 Not Found",
        html! {
            div class="min-h-screen flex items-center justify-center bg-gray-50" {
                div class="text-center" {
                    h1 class="text-6xl font-bold text-gray-900 mb-4" { "404" }
                    p class="text-xl text-gray-600 mb-8" { "Page not found" }
                    a href="/" class="text-blue-600 hover:text-blue-800 underline" {
                        "Go back home"
                    }
                }
            }
        },
    )
}
