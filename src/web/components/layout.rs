use maud::{html, Markup, DOCTYPE};

/// Base HTML layout with Tailwind CSS and HTMX
pub fn base(title: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " - Smally" }

                // Tailwind CSS (using CDN for now, can switch to build later)
                script src="https://cdn.tailwindcss.com" {}

                // HTMX for dynamic interactions
                script src="https://unpkg.com/htmx.org@1.9.10" defer {}

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

/// Navigation bar for authenticated pages with organization switcher
pub fn navbar(
    user_email: &str,
    current_org: Option<(&str, &str)>,
    other_orgs: &[(&str, &str)],
) -> Markup {
    html! {
        nav class="bg-white shadow-sm border-b border-gray-200" {
            div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8" {
                div class="flex justify-between h-16" {
                    div class="flex items-center" {
                        // Logo
                        div class="flex-shrink-0 flex items-center" {
                            a href="/organizations" class="flex items-center gap-2" {
                                (logo())
                                span class="text-2xl font-bold text-primary" { "Smally" }
                            }
                        }

                        // Organization switcher dropdown
                        @if let Some((org_id, org_name)) = current_org {
                            div class="ml-6 relative" {
                                div class="relative inline-block text-left" {
                                    button
                                        type="button"
                                        onclick="document.getElementById('org-dropdown').classList.toggle('hidden')"
                                        class="inline-flex justify-center items-center w-full rounded-md border border-gray-300 shadow-sm px-4 py-2 bg-white text-sm font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary"
                                        id="org-menu-button"
                                        aria-expanded="false"
                                        aria-haspopup="true" {
                                        span { (org_name) }
                                        svg class="-mr-1 ml-2 h-5 w-5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" {
                                            path fill-rule="evenodd" d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z" clip-rule="evenodd" {}
                                        }
                                    }

                                    div
                                        class="hidden origin-top-right absolute left-0 mt-2 w-56 rounded-md shadow-lg bg-white ring-1 ring-black ring-opacity-5 z-10"
                                        id="org-dropdown"
                                        role="menu"
                                        aria-orientation="vertical"
                                        aria-labelledby="org-menu-button"
                                        tabindex="-1" {
                                        div class="py-1" role="none" {
                                            // Current organization (selected)
                                            a
                                                href=(format!("/switch-org/{}", org_id))
                                                class="flex items-center px-4 py-2 text-sm text-gray-900 bg-gray-100 font-medium"
                                                role="menuitem" {
                                                svg class="mr-3 h-5 w-5 text-primary" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" {
                                                    path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd" {}
                                                }
                                                (org_name)
                                            }

                                            @if !other_orgs.is_empty() {
                                                div class="border-t border-gray-100" {}

                                                @for (other_id, other_name) in other_orgs {
                                                    a
                                                        href=(format!("/switch-org/{}", other_id))
                                                        class="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100"
                                                        role="menuitem" {
                                                        span class="mr-8" {} // Spacer for alignment
                                                        (other_name)
                                                    }
                                                }
                                            }

                                            div class="border-t border-gray-100" {}
                                            a href="/organizations" class="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100" role="menuitem" {
                                                "Manage Organizations"
                                            }
                                        }
                                    }
                                }
                            }
                        } @else {
                            div class="ml-6" {
                                a href="/organizations" class="text-sm font-medium text-gray-700 hover:text-gray-900" {
                                    "Select Organization"
                                }
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

/// Smally logo - "S" on fire
pub fn logo() -> Markup {
    html! {
        svg width="40" height="40" viewBox="0 0 100 120" xmlns="http://www.w3.org/2000/svg" class="inline-block" {
            defs {
                // Gradient for fire effect
                linearGradient id="fireGradient" x1="0%" y1="0%" x2="0%" y2="100%" {
                    stop offset="0%" style="stop-color:#ff6b00;stop-opacity:1" {}
                    stop offset="50%" style="stop-color:#ff8800;stop-opacity:1" {}
                    stop offset="100%" style="stop-color:#ffa500;stop-opacity:1" {}
                }
                // Gradient for the S letter
                linearGradient id="letterGradient" x1="0%" y1="0%" x2="0%" y2="100%" {
                    stop offset="0%" style="stop-color:#3b82f6;stop-opacity:1" {}
                    stop offset="100%" style="stop-color:#1e40af;stop-opacity:1" {}
                }
            }

            // Flame shapes (background)
            path d="M 30,90 Q 25,75 30,60 Q 35,70 30,90 Z" fill="url(#fireGradient)" opacity="0.6" {
                animate attributeName="d" values="M 30,90 Q 25,75 30,60 Q 35,70 30,90 Z; M 30,90 Q 28,73 30,55 Q 32,68 30,90 Z; M 30,90 Q 25,75 30,60 Q 35,70 30,90 Z" dur="1.5s" repeatCount="indefinite" {}
            }
            path d="M 50,95 Q 45,80 50,65 Q 55,75 50,95 Z" fill="url(#fireGradient)" opacity="0.7" {
                animate attributeName="d" values="M 50,95 Q 45,80 50,65 Q 55,75 50,95 Z; M 50,95 Q 47,78 50,60 Q 53,73 50,95 Z; M 50,95 Q 45,80 50,65 Q 55,75 50,95 Z" dur="1.8s" repeatCount="indefinite" {}
            }
            path d="M 70,90 Q 65,75 70,60 Q 75,70 70,90 Z" fill="url(#fireGradient)" opacity="0.6" {
                animate attributeName="d" values="M 70,90 Q 65,75 70,60 Q 75,70 70,90 Z; M 70,90 Q 68,73 70,55 Q 72,68 70,90 Z; M 70,90 Q 65,75 70,60 Q 75,70 70,90 Z" dur="2s" repeatCount="indefinite" {}
            }

            // The "S" letter
            path d="M 65,25 Q 75,20 75,30 Q 75,40 50,45 Q 25,50 25,60 Q 25,70 35,75 M 35,75 Q 25,70 25,60 Q 25,50 50,45 Q 75,40 75,30 Q 75,15 55,20"
                  fill="none"
                  stroke="url(#letterGradient)"
                  stroke-width="8"
                  stroke-linecap="round"
                  stroke-linejoin="round" {}
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
