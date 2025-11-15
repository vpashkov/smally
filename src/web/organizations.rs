use axum::{
    extract::{Form, Query},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use maud::{html, Markup};
use serde::Deserialize;

use crate::auth::session::{create_session_cookie, create_session_token_with_org, SessionCookie};
use crate::database;
use crate::models::{OrganizationRole, TierType};
use crate::uuid_dashless::DashlessUuid;
use axum::extract::Path;
use axum::http::header;
use chrono::Utc;

use super::components::layout;

/// Query parameters for organizations list
#[derive(Debug, Deserialize)]
pub struct OrganizationsQuery {
    pub new: Option<bool>,
}

/// Organization with user's role
#[derive(Debug, sqlx::FromRow)]
struct OrganizationWithRole {
    id: uuid::Uuid,
    name: String,
    slug: String,
    tier: TierType,
    is_active: bool,
    role: OrganizationRole,
}

/// Form data for creating organization
#[derive(Debug, Deserialize)]
pub struct CreateOrganizationForm {
    pub name: String,
    pub slug: String,
}

/// List all organizations for the current user
pub async fn list(
    session: SessionCookie,
    Query(query): Query<OrganizationsQuery>,
) -> Result<Markup, Response> {
    let pool = database::get_db();
    let user_id = session.user_id();

    // Fetch organizations where user is a member
    let organizations = sqlx::query_as::<_, OrganizationWithRole>(
        r#"
        SELECT o.id, o.name, o.slug, o.tier, o.is_active, om.role
        FROM organizations o
        INNER JOIN organization_members om ON o.id = om.organization_id
        WHERE om.user_id = $1
        ORDER BY o.created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch organizations: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to fetch organizations",
        )
            .into_response()
    })?;

    Ok(layout::base(
        "Organizations",
        html! {
            (layout::navbar(session.email(), None, &[]))
            (layout::container(html! {
                div class="space-y-6" {
                    // Header
                    div class="md:flex md:items-center md:justify-between" {
                        div class="flex-1 min-w-0" {
                            h1 class="text-3xl font-bold text-gray-900" {
                                "Organizations"
                            }
                            p class="mt-2 text-sm text-gray-500" {
                                "Manage your organizations and teams"
                            }
                        }
                        div class="mt-4 flex md:mt-0 md:ml-4" {
                            button
                                onclick="document.getElementById('create-org-modal').classList.remove('hidden')"
                                class="inline-flex items-center px-4 py-2 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-primary hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary" {
                                svg class="mr-2 h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" {
                                    path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" {}
                                }
                                "New Organization"
                            }
                        }
                    }

                    // Organizations grid
                    @if organizations.is_empty() {
                        (layout::card("No Organizations", html! {
                            div class="text-center py-12" {
                                svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" {
                                    path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4" {}
                                }
                                h3 class="mt-2 text-sm font-medium text-gray-900" {
                                    "No organizations"
                                }
                                p class="mt-1 text-sm text-gray-500" {
                                    "Get started by creating a new organization."
                                }
                                div class="mt-6" {
                                    button
                                        onclick="document.getElementById('create-org-modal').classList.remove('hidden')"
                                        class="inline-flex items-center px-4 py-2 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-primary hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary" {
                                        "Create Organization"
                                    }
                                }
                            }
                        }))
                    } @else {
                        div class="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3" {
                            @for org in &organizations {
                                (organization_card(org))
                            }
                        }
                    }
                }

                // Create organization modal
                (create_organization_modal(query.new.unwrap_or(false)))
            }))
        },
    ))
}

/// Render an organization card
fn organization_card(org: &OrganizationWithRole) -> Markup {
    let tier_badge = match org.tier {
        TierType::Free => ("bg-gray-100 text-gray-800", "Free"),
        TierType::Pro => ("bg-blue-100 text-blue-800", "Pro"),
        TierType::Scale => ("bg-purple-100 text-purple-800", "Scale"),
    };

    let role_badge = match org.role {
        OrganizationRole::Owner => ("bg-yellow-100 text-yellow-800", "Owner"),
        OrganizationRole::Admin => ("bg-green-100 text-green-800", "Admin"),
        OrganizationRole::Member => ("bg-gray-100 text-gray-800", "Member"),
    };

    html! {
        div class="bg-white overflow-hidden shadow rounded-lg hover:shadow-md transition-shadow" {
            div class="p-6" {
                div class="flex items-center justify-between" {
                    h3 class="text-lg font-medium text-gray-900 truncate" {
                        (org.name)
                    }
                    @if org.is_active {
                        span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800" {
                            "Active"
                        }
                    } @else {
                        span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-red-100 text-red-800" {
                            "Inactive"
                        }
                    }
                }
                p class="mt-1 text-sm text-gray-500" {
                    "Slug: " span class="font-mono" { (org.slug) }
                }
                div class="mt-4 flex items-center space-x-2" {
                    span class=(format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}", tier_badge.0)) {
                        (tier_badge.1)
                    }
                    span class=(format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}", role_badge.0)) {
                        (role_badge.1)
                    }
                }
                div class="mt-6" {
                    a
                        href=(format!("/organizations/{}", org.id.simple()))
                        class="text-primary hover:text-blue-500 text-sm font-medium" {
                        "View details →"
                    }
                }
            }
        }
    }
}

/// Create organization modal
fn create_organization_modal(auto_open: bool) -> Markup {
    let modal_class = if auto_open {
        "fixed z-10 inset-0 overflow-y-auto"
    } else {
        "hidden fixed z-10 inset-0 overflow-y-auto"
    };

    html! {
        div
            id="create-org-modal"
            class=(modal_class)
            aria-labelledby="modal-title"
            role="dialog"
            aria-modal="true" {
            div class="flex items-end justify-center min-h-screen pt-4 px-4 pb-20 text-center sm:block sm:p-0" {
                // Background overlay
                div
                    onclick="document.getElementById('create-org-modal').classList.add('hidden')"
                    class="fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity"
                    aria-hidden="true" {}

                // Center modal
                span class="hidden sm:inline-block sm:align-middle sm:h-screen" aria-hidden="true" { "\u{200B}" }

                div class="inline-block align-bottom bg-white rounded-lg px-4 pt-5 pb-4 text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-lg sm:w-full sm:p-6" {
                    div {
                        div class="mt-3 text-center sm:mt-0 sm:text-left" {
                            h3 class="text-lg leading-6 font-medium text-gray-900" id="modal-title" {
                                "Create New Organization"
                            }
                            div class="mt-4" {
                                form action="/organizations" method="POST" {
                                    div class="space-y-4" {
                                        div {
                                            label for="name" class="block text-sm font-medium text-gray-700" {
                                                "Organization Name"
                                            }
                                            input
                                                type="text"
                                                name="name"
                                                id="name"
                                                required
                                                class="mt-1 block w-full border border-gray-300 rounded-md shadow-sm py-2 px-3 focus:outline-none focus:ring-primary focus:border-primary sm:text-sm"
                                                placeholder="Acme Inc.";
                                        }
                                        div {
                                            label for="slug" class="block text-sm font-medium text-gray-700" {
                                                "Slug"
                                            }
                                            input
                                                type="text"
                                                name="slug"
                                                id="slug"
                                                required
                                                pattern="[a-z0-9-]+"
                                                class="mt-1 block w-full border border-gray-300 rounded-md shadow-sm py-2 px-3 focus:outline-none focus:ring-primary focus:border-primary sm:text-sm"
                                                placeholder="acme-inc";
                                            p class="mt-1 text-xs text-gray-500" {
                                                "Lowercase letters, numbers, and hyphens only"
                                            }
                                        }
                                    }
                                    div class="mt-5 sm:mt-6 sm:grid sm:grid-cols-2 sm:gap-3 sm:grid-flow-row-dense" {
                                        button
                                            type="submit"
                                            class="w-full inline-flex justify-center rounded-md border border-transparent shadow-sm px-4 py-2 bg-primary text-base font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary sm:col-start-2 sm:text-sm" {
                                            "Create"
                                        }
                                        button
                                            type="button"
                                            onclick="document.getElementById('create-org-modal').classList.add('hidden')"
                                            class="mt-3 w-full inline-flex justify-center rounded-md border border-gray-300 shadow-sm px-4 py-2 bg-white text-base font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary sm:mt-0 sm:col-start-1 sm:text-sm" {
                                            "Cancel"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Handle organization creation
pub async fn create(
    session: SessionCookie,
    Form(form): Form<CreateOrganizationForm>,
) -> Result<Response, Response> {
    let pool = database::get_db();
    let user_id = session.user_id();

    // Validate slug format
    if !form
        .slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err((
            StatusCode::BAD_REQUEST,
            layout::base("Invalid Slug", html! {
                div class="min-h-screen flex items-center justify-center bg-gray-50" {
                    div class="max-w-md w-full" {
                        (layout::alert("Slug must contain only lowercase letters, numbers, and hyphens", "error"))
                        a href="/organizations" class="text-primary hover:text-blue-500" {
                            "← Back to organizations"
                        }
                    }
                }
            }),
        )
            .into_response());
    }

    // Check if slug is already taken
    let existing =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM organizations WHERE slug = $1")
            .bind(&form.slug)
            .fetch_one(pool)
            .await
            .map_err(|e| {
                tracing::error!("Database error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
            })?;

    if existing > 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            layout::base("Slug Taken", html! {
                div class="min-h-screen flex items-center justify-center bg-gray-50" {
                    div class="max-w-md w-full" {
                        (layout::alert("This slug is already taken. Please choose another.", "error"))
                        a href="/organizations" class="text-primary hover:text-blue-500" {
                            "← Back to organizations"
                        }
                    }
                }
            }),
        )
            .into_response());
    }

    // Create organization
    let org_id = sqlx::query_scalar::<_, uuid::Uuid>(
        "INSERT INTO organizations (name, slug, owner_id, tier, is_active, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id",
    )
    .bind(&form.name)
    .bind(&form.slug)
    .bind(user_id)
    .bind(TierType::Free)
    .bind(true)
    .bind(Utc::now().naive_utc())
    .bind(Utc::now().naive_utc())
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create organization: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create organization",
        )
            .into_response()
    })?;

    // Add user as owner
    sqlx::query(
        "INSERT INTO organization_members (organization_id, user_id, role, created_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(org_id)
    .bind(user_id)
    .bind(OrganizationRole::Owner)
    .bind(Utc::now().naive_utc())
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to add organization member: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to add organization member",
        )
            .into_response()
    })?;

    // Redirect to organizations list
    Ok(Redirect::to("/organizations").into_response())
}

/// Switch organization context
pub async fn switch_org(
    session: SessionCookie,
    Path(org_id): Path<DashlessUuid>,
) -> Result<Response, Response> {
    let pool = database::get_db();
    let user_id = session.user_id();
    let org_id = org_id.into_inner();

    // Verify user is a member of this organization
    let is_member = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM organization_members WHERE organization_id = $1 AND user_id = $2)",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
    })?;

    if !is_member {
        return Err((
            StatusCode::FORBIDDEN,
            layout::base(
                "Access Denied",
                html! {
                    div class="min-h-screen flex items-center justify-center bg-gray-50" {
                        div class="max-w-md w-full" {
                            (layout::alert("You don't have access to this organization", "error"))
                            a href="/organizations" class="text-primary hover:text-blue-500" {
                                "← Back to organizations"
                            }
                        }
                    }
                },
            ),
        )
            .into_response());
    }

    // Create new session token with organization context
    let token = create_session_token_with_org(user_id, session.email(), Some(org_id)).map_err(
        |e| {
            tracing::error!("Failed to create session token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update session",
            )
                .into_response()
        },
    )?;

    // Create session cookie
    let cookie = create_session_cookie(&token);

    // Redirect to dashboard with new session
    let mut response = Redirect::to("/dashboard").into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, cookie.to_string().parse().unwrap());

    Ok(response)
}
