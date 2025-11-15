use maud::{html, Markup, DOCTYPE};

/// Base HTML layout with Tailwind CSS and HTMX
pub fn base(title: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " - FastEmbed" }

                // Tailwind CSS (using CDN for now, can switch to build later)
                script src="https://cdn.tailwindcss.com" {}

                // HTMX for dynamic interactions
                script src="https://unpkg.com/htmx.org@1.9.10" {}

                // Custom configuration for Tailwind
                script {
                    r#"
                    tailwind.config = {
                        theme: {
                            extend: {
                                colors: {
                                    primary: '#3b82f6',
                                    secondary: '#8b5cf6',
                                }
                            }
                        }
                    }
                    "#
                }
            }
            body class="bg-gray-50 min-h-screen" {
                (content)
            }
        }
    }
}

/// Navigation bar for authenticated pages
pub fn navbar(user_email: &str) -> Markup {
    html! {
        nav class="bg-white shadow-sm border-b border-gray-200" {
            div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8" {
                div class="flex justify-between h-16" {
                    div class="flex" {
                        // Logo
                        div class="flex-shrink-0 flex items-center" {
                            a href="/dashboard" class="text-2xl font-bold text-primary" {
                                "FastEmbed"
                            }
                        }

                        // Navigation links
                        div class="hidden sm:ml-6 sm:flex sm:space-x-8" {
                            a href="/dashboard"
                              class="border-primary text-gray-900 inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium" {
                                "Dashboard"
                            }
                            a href="/organizations"
                              class="border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700 inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium" {
                                "Organizations"
                            }
                        }
                    }

                    // User menu
                    div class="flex items-center" {
                        div class="ml-3 relative" {
                            button
                                type="button"
                                class="flex text-sm rounded-full focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary"
                                id="user-menu-button"
                                aria-expanded="false"
                                aria-haspopup="true" {
                                span class="sr-only" { "Open user menu" }
                                div class="h-8 w-8 rounded-full bg-primary flex items-center justify-center text-white font-medium" {
                                    (user_email.chars().next().unwrap_or('U').to_uppercase())
                                }
                            }
                        }
                        span class="ml-3 text-sm text-gray-700" { (user_email) }
                        form action="/logout" method="post" class="ml-4" {
                            button
                                type="submit"
                                class="text-sm text-gray-500 hover:text-gray-700" {
                                "Logout"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Container for main content
pub fn container(content: Markup) -> Markup {
    html! {
        div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8" {
            (content)
        }
    }
}

/// Card component
pub fn card(title: &str, content: Markup) -> Markup {
    html! {
        div class="bg-white overflow-hidden shadow rounded-lg" {
            div class="px-4 py-5 sm:p-6" {
                h3 class="text-lg leading-6 font-medium text-gray-900 mb-4" {
                    (title)
                }
                (content)
            }
        }
    }
}

/// Button component
pub fn button(text: &str, button_type: &str, extra_classes: &str) -> Markup {
    let base_classes = "inline-flex justify-center py-2 px-4 border border-transparent shadow-sm text-sm font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-2";

    let color_classes = match button_type {
        "primary" => "text-white bg-primary hover:bg-blue-700 focus:ring-primary",
        "secondary" => "text-white bg-gray-600 hover:bg-gray-700 focus:ring-gray-500",
        "danger" => "text-white bg-red-600 hover:bg-red-700 focus:ring-red-500",
        _ => "text-white bg-primary hover:bg-blue-700 focus:ring-primary",
    };

    html! {
        button
            type="submit"
            class=(format!("{} {} {}", base_classes, color_classes, extra_classes)) {
            (text)
        }
    }
}

/// Alert message component
pub fn alert(message: &str, alert_type: &str) -> Markup {
    let (bg_class, text_class, _border_class) = match alert_type {
        "success" => ("bg-green-50", "text-green-800", "border-green-200"),
        "error" => ("bg-red-50", "text-red-800", "border-red-200"),
        "warning" => ("bg-yellow-50", "text-yellow-800", "border-yellow-200"),
        "info" => ("bg-blue-50", "text-blue-800", "border-blue-200"),
        _ => ("bg-blue-50", "text-blue-800", "border-blue-200"),
    };

    html! {
        div class=(format!("rounded-md p-4 mb-4 border {}", bg_class)) {
            div class="flex" {
                div class="flex-shrink-0" {
                    // Icon placeholder
                }
                div class=(format!("ml-3 {}", text_class)) {
                    p class="text-sm font-medium" {
                        (message)
                    }
                }
            }
        }
    }
}
