use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;
use maud::{html, Markup};

use super::components::layout;
use crate::auth::session::SessionCookie;
use crate::database;

/// Organization info for dropdown
#[derive(Debug, sqlx::FromRow)]
struct OrgInfo {
    id: uuid::Uuid,
    name: String,
}

/// Show dashboard with user session
pub async fn show(session: SessionCookie) -> Result<Markup, Response> {
    let pool = database::get_db();
    let user_id = session.user_id();
    let user_email = session.email();

    // Fetch all organizations user belongs to
    let all_orgs = sqlx::query_as::<_, OrgInfo>(
        "SELECT o.id, o.name
         FROM organizations o
         INNER JOIN organization_members om ON o.id = om.organization_id
         WHERE om.user_id = $1 AND o.is_active = true
         ORDER BY o.created_at ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
    })?;

    if all_orgs.is_empty() {
        // No organizations - show message to create one
        return Ok(layout::base(
            "Dashboard",
            html! {
                (layout::navbar(user_email, None, &[]))
                (layout::container(html! {
                    div class="text-center py-12" {
                        h1 class="text-2xl font-bold text-gray-900 mb-4" {
                            "Welcome to Smally!"
                        }
                        p class="text-gray-600 mb-8" {
                            "You don't have any organizations yet. Create one to get started."
                        }
                        a
                            href="/organizations?new=true"
                            class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-primary hover:bg-blue-700" {
                            "Create Organization"
                        }
                    }
                }))
            },
        ));
    }

    // Determine current organization from session or default to first one
    let current_org_id = session.current_org_id().unwrap_or(all_orgs[0].id);

    // Find current org in the list
    let current_org_index = all_orgs
        .iter()
        .position(|org| org.id == current_org_id)
        .unwrap_or(0);

    let current_org = &all_orgs[current_org_index];
    let current_org_name = &current_org.name;
    let current_org_id_simple = current_org.id.simple().to_string();

    // Build other orgs list for dropdown
    let other_orgs: Vec<(String, String)> = all_orgs
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != current_org_index)
        .map(|(_, org)| (org.id.simple().to_string(), org.name.clone()))
        .collect();

    // Convert to slices for navbar
    let other_orgs_refs: Vec<(&str, &str)> = other_orgs
        .iter()
        .map(|(id, name)| (id.as_str(), name.as_str()))
        .collect();

    Ok(layout::base(
        "Dashboard",
        html! {
            (layout::navbar(
                user_email,
                Some((current_org_id_simple.as_str(), current_org_name.as_str())),
                &other_orgs_refs
            ))
            (layout::container(html! {
                h1 class="text-3xl font-bold text-gray-900 mb-8" {
                    "Dashboard - " (current_org_name)
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
                            href="/organizations?new=true"
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
                                "Welcome to Smally! Get started by creating an organization and generating your first API key."
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
        },
    ))
}
