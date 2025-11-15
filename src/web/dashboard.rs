use maud::{html, Markup};

use super::components::layout;

/// Show dashboard (placeholder for now)
pub async fn show() -> Markup {
    // TODO: Extract user from session cookie
    // For now, show a placeholder dashboard
    let user_email = "user@example.com";

    layout::base("Dashboard", html! {
        (layout::navbar(user_email))
        (layout::container(html! {
            h1 class="text-3xl font-bold text-gray-900 mb-8" {
                "Dashboard"
            }

            // Stats cards
            div class="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-3" {
                // Organizations card
                (layout::card("Organizations", html! {
                    div class="flex items-center" {
                        div class="flex-shrink-0 bg-primary rounded-md p-3" {
                            svg class="h-6 w-6 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24" {
                                path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4" {}
                            }
                        }
                        div class="ml-5 w-0 flex-1" {
                            dl {
                                dt class="text-sm font-medium text-gray-500 truncate" {
                                    "Total Organizations"
                                }
                                dd class="text-3xl font-semibold text-gray-900" {
                                    "1"
                                }
                            }
                        }
                    }
                }))

                // API Keys card
                (layout::card("API Keys", html! {
                    div class="flex items-center" {
                        div class="flex-shrink-0 bg-green-500 rounded-md p-3" {
                            svg class="h-6 w-6 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24" {
                                path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" {}
                            }
                        }
                        div class="ml-5 w-0 flex-1" {
                            dl {
                                dt class="text-sm font-medium text-gray-500 truncate" {
                                    "Active API Keys"
                                }
                                dd class="text-3xl font-semibold text-gray-900" {
                                    "0"
                                }
                            }
                        }
                    }
                }))

                // Usage card
                (layout::card("Usage This Month", html! {
                    div class="flex items-center" {
                        div class="flex-shrink-0 bg-purple-500 rounded-md p-3" {
                            svg class="h-6 w-6 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24" {
                                path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6" {}
                            }
                        }
                        div class="ml-5 w-0 flex-1" {
                            dl {
                                dt class="text-sm font-medium text-gray-500 truncate" {
                                    "Embeddings Generated"
                                }
                                dd class="text-3xl font-semibold text-gray-900" {
                                    "0"
                                }
                            }
                        }
                    }
                }))
            }

            // Quick actions
            div class="mt-8" {
                h2 class="text-xl font-bold text-gray-900 mb-4" {
                    "Quick Actions"
                }

                div class="grid grid-cols-1 gap-4 sm:grid-cols-2" {
                    a
                        href="/organizations"
                        class="relative block w-full border-2 border-gray-300 border-dashed rounded-lg p-12 text-center hover:border-gray-400 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary" {
                        svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" {
                            path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" {}
                        }
                        span class="mt-2 block text-sm font-medium text-gray-900" {
                            "Create New Organization"
                        }
                    }

                    button
                        class="relative block w-full border-2 border-gray-300 border-dashed rounded-lg p-12 text-center hover:border-gray-400 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary" {
                        svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" {
                            path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" {}
                        }
                        span class="mt-2 block text-sm font-medium text-gray-900" {
                            "Generate API Key"
                        }
                    }
                }
            }

            // Getting started guide
            div class="mt-8" {
                (layout::card("Getting Started", html! {
                    div class="prose max-w-none" {
                        p class="text-gray-600" {
                            "Welcome to FastEmbed! Get started by creating an organization and generating your first API key."
                        }

                        ol class="mt-4 space-y-2 text-sm text-gray-600" {
                            li { "Create an organization to manage your team" }
                            li { "Generate an API key for your organization" }
                            li { "Start making API calls to generate embeddings" }
                        }

                        div class="mt-6" {
                            a
                                href="#"
                                class="text-primary hover:text-blue-500 font-medium" {
                                "View API Documentation →"
                            }
                        }
                    }
                }))
            }
        }))
    })
}
