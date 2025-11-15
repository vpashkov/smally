pub mod api_keys;
pub mod auth;
pub mod components;
pub mod organizations;

use maud::{html, Markup};

/// Home page - landing page with login button
pub async fn home() -> Markup {
    components::layout::base(
        "Smally - Fast Text Embeddings API",
        html! {
            div class="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100" {
                // Navigation
                nav class="bg-white shadow-sm" {
                    div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8" {
                        div class="flex justify-between h-16" {
                            div class="flex items-center gap-2" {
                                (components::layout::logo())
                                span class="text-2xl font-bold text-primary" { "Smally" }
                            }
                            div class="flex items-center gap-4" {
                                a
                                    href="/login"
                                    class="text-gray-700 hover:text-primary font-medium" {
                                    "Sign in"
                                }
                                a
                                    href="/register"
                                    class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-primary hover:bg-blue-700" {
                                    "Get Started"
                                }
                            }
                        }
                    }
                }

                // Hero section
                div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 pt-20 pb-16 text-center" {
                    h1 class="text-5xl font-extrabold text-gray-900 sm:text-6xl md:text-7xl mb-8" {
                        "Fast Text Embeddings API"
                    }
                    p class="text-xl text-gray-600 max-w-3xl mx-auto mb-12" {
                        "Production-ready embedding service powered by state-of-the-art BERT models. "
                        "Generate high-quality vector representations for semantic search, RAG, and more."
                    }
                    div class="flex justify-center gap-4" {
                        a
                            href="/register"
                            class="inline-flex items-center px-8 py-3 border border-transparent text-base font-medium rounded-md shadow-sm text-white bg-primary hover:bg-blue-700" {
                            "Start Free"
                        }
                        a
                            href="/docs"
                            class="inline-flex items-center px-8 py-3 border border-gray-300 text-base font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50" {
                            "Documentation"
                        }
                    }
                }

                // Features
                div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 pb-20" {
                    div class="grid grid-cols-1 gap-8 sm:grid-cols-2 lg:grid-cols-3" {
                        (feature_card(
                            "⚡ Fast",
                            "Optimized ONNX runtime with intelligent caching for blazing-fast embeddings"
                        ))
                        (feature_card(
                            "🔒 Secure",
                            "API key authentication with rate limiting and usage tracking"
                        ))
                        (feature_card(
                            "📊 Scalable",
                            "Built with Rust for high performance and reliable production deployments"
                        ))
                    }
                }
            }
        },
    )
}

fn feature_card(title: &str, description: &str) -> Markup {
    html! {
        div class="bg-white rounded-lg shadow-md p-6" {
            h3 class="text-xl font-bold text-gray-900 mb-2" {
                (title)
            }
            p class="text-gray-600" {
                (description)
            }
        }
    }
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
