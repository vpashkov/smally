use axum::{
    extract::{Form, Path, Query},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use maud::{html, Markup};
use serde::Deserialize;

use crate::auth::session::SessionCookie;
use crate::auth::{sign_token_direct, TokenData};
use crate::config;
use crate::database;
use crate::models::{APIKey, OrganizationRole, TierType};
use crate::uuid_dashless::DashlessUuid;
use chrono::Utc;
use uuid::Uuid;

use super::components::layout;
use super::organizations::OrganizationsQuery;

/// Organization with user's role (for access check)
#[derive(Debug, sqlx::FromRow)]
struct OrganizationWithRole {
    id: Uuid,
    name: String,
    tier: TierType,
    is_active: bool,
    role: OrganizationRole,
}

/// Form data for creating API key
#[derive(Debug, Deserialize)]
pub struct CreateAPIKeyForm {
    pub name: String,
}

/// Helper struct for org list
#[derive(Debug, sqlx::FromRow)]
struct OrgListItem {
    id: Uuid,
    name: String,
}

/// Show organization detail with API keys
pub async fn show(
    session: SessionCookie,
    Path(org_id): Path<DashlessUuid>,
    Query(query): Query<OrganizationsQuery>,
) -> Result<Markup, Response> {
    let pool = database::get_db();
    let user_id = session.user_id();
    let org_id = org_id.into_inner();

    // Fetch all user's organizations for the dropdown
    let all_orgs = sqlx::query_as::<_, OrgListItem>(
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

    // Check user has access to this organization
    let org = sqlx::query_as::<_, OrganizationWithRole>(
        r#"
        SELECT o.id, o.name, o.tier, o.is_active, om.role
        FROM organizations o
        INNER JOIN organization_members om ON o.id = om.organization_id
        WHERE o.id = $1 AND om.user_id = $2
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
    })?
    .ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            layout::base("Organization Not Found", html! {
                div class="min-h-screen flex items-center justify-center bg-gray-50" {
                    div class="max-w-md w-full" {
                        (layout::alert("Organization not found or you don't have access", "error"))
                        a href="/organizations" class="text-primary hover:text-blue-500" {
                            "← Back to organizations"
                        }
                    }
                }
            }),
        )
            .into_response()
    })?;

    // Fetch API keys for this organization
    let api_keys = sqlx::query_as::<_, APIKey>(
        "SELECT * FROM api_keys WHERE organization_id = $1 ORDER BY created_at DESC",
    )
    .bind(org_id)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch API keys: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to fetch API keys",
        )
            .into_response()
    })?;

    // Build organization dropdown data
    let current_org_id_simple = org_id.simple().to_string();
    let current_org_name = &org.name;

    let other_orgs: Vec<(String, String)> = all_orgs
        .iter()
        .filter(|o| o.id != org_id)
        .map(|o| (o.id.simple().to_string(), o.name.clone()))
        .collect();

    let other_orgs_refs: Vec<(&str, &str)> = other_orgs
        .iter()
        .map(|(id, name)| (id.as_str(), name.as_str()))
        .collect();

    Ok(layout::base(
        &format!("{} - Organization", org.name),
        html! {
            (layout::navbar(
                session.email(),
                Some((current_org_id_simple.as_str(), current_org_name)),
                &other_orgs_refs
            ))
            (layout::container(html! {
                // Breadcrumb
                nav class="mb-6" {
                    ol class="flex items-center space-x-2 text-sm" {
                        li {
                            a href="/organizations" class="text-gray-500 hover:text-gray-700" { "Organizations" }
                        }
                        li class="text-gray-400" { "/" }
                        li class="text-gray-900 font-medium" { (org.name) }
                    }
                }

                div class="space-y-6" {
                    // Organization header
                    div class="bg-white shadow rounded-lg p-6" {
                        div class="flex items-center justify-between" {
                            div {
                                h1 class="text-3xl font-bold text-gray-900" { (org.name) }
                                div class="mt-3 flex items-center space-x-2" {
                                    @let tier_class = match org.tier {
                                        TierType::Free => "bg-gray-100 text-gray-800",
                                        TierType::Pro => "bg-blue-100 text-blue-800",
                                        TierType::Scale => "bg-purple-100 text-purple-800",
                                    };
                                    @let tier_label = match org.tier {
                                        TierType::Free => "Free",
                                        TierType::Pro => "Pro",
                                        TierType::Scale => "Scale",
                                    };
                                    span class=(format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}", tier_class)) {
                                        (tier_label)
                                    }
                                    @let role_class = match org.role {
                                        OrganizationRole::Owner => "bg-yellow-100 text-yellow-800",
                                        OrganizationRole::Admin => "bg-green-100 text-green-800",
                                        OrganizationRole::Member => "bg-gray-100 text-gray-800",
                                    };
                                    @let role_label = match org.role {
                                        OrganizationRole::Owner => "Owner",
                                        OrganizationRole::Admin => "Admin",
                                        OrganizationRole::Member => "Member",
                                    };
                                    span class=(format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}", role_class)) {
                                        (role_label)
                                    }
                                }
                            }
                        }
                    }

                    // API Keys section
                    div {
                        div class="flex items-center justify-between mb-4" {
                            h2 class="text-xl font-bold text-gray-900" { "API Keys" }
                            button
                                onclick="document.getElementById('create-key-modal').classList.remove('hidden')"
                                class="inline-flex items-center px-4 py-2 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-primary hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary" {
                                svg class="mr-2 h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" {
                                    path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" {}
                                }
                                "New API Key"
                            }
                        }

                        @if api_keys.is_empty() {
                            (layout::card("No API Keys", html! {
                                div class="text-center py-12" {
                                    svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" {
                                        path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" {}
                                    }
                                    h3 class="mt-2 text-sm font-medium text-gray-900" {
                                        "No API keys"
                                    }
                                    p class="mt-1 text-sm text-gray-500" {
                                        "Create an API key to start using the API."
                                    }
                                    div class="mt-6" {
                                        button
                                            onclick="document.getElementById('create-key-modal').classList.remove('hidden')"
                                            class="inline-flex items-center px-4 py-2 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-primary hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary" {
                                            "Create API Key"
                                        }
                                    }
                                }
                            }))
                        } @else {
                            (api_keys_table(&api_keys, org_id))
                        }
                    }
                }

                // Create API key modal
                (create_api_key_modal(org_id, query.new.unwrap_or(false)))
            }))
        },
    ))
}

/// Render API keys table
fn api_keys_table(api_keys: &[APIKey], org_id: uuid::Uuid) -> Markup {
    html! {
        div class="bg-white shadow overflow-hidden sm:rounded-lg" {
            table class="min-w-full divide-y divide-gray-200" {
                thead class="bg-gray-50" {
                    tr {
                        th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider" { "Name" }
                        th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider" { "Key Prefix" }
                        th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider" { "Status" }
                        th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider" { "Last Used" }
                        th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider" { "Created" }
                        th scope="col" class="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider" { "Actions" }
                    }
                }
                tbody class="bg-white divide-y divide-gray-200" {
                    @for key in api_keys {
                        tr {
                            td class="px-6 py-4 whitespace-nowrap" {
                                div class="text-sm font-medium text-gray-900" { (key.name) }
                            }
                            td class="px-6 py-4 whitespace-nowrap" {
                                code class="text-xs text-gray-600" { (format!("fe_{}...", &key.key_id.to_string()[..8])) }
                            }
                            td class="px-6 py-4 whitespace-nowrap" {
                                @if key.is_active {
                                    span class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-green-100 text-green-800" {
                                        "Active"
                                    }
                                } @else {
                                    span class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-red-100 text-red-800" {
                                        "Revoked"
                                    }
                                }
                            }
                            td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500" {
                                @if let Some(last_used) = key.last_used_at {
                                    (last_used.format("%Y-%m-%d %H:%M").to_string())
                                } @else {
                                    span class="text-gray-400" { "Never" }
                                }
                            }
                            td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500" {
                                (key.created_at.format("%Y-%m-%d").to_string())
                            }
                            td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium" {
                                @if key.is_active {
                                    form action=(format!("/organizations/{}/keys/{}/revoke", org_id.simple(), key.id.simple())) method="POST" class="inline" {
                                        button
                                            type="submit"
                                            class="text-red-600 hover:text-red-900"
                                            onclick="return confirm('Are you sure you want to revoke this API key? This cannot be undone.')" {
                                            "Revoke"
                                        }
                                    }
                                } @else {
                                    span class="text-gray-400" { "Revoked" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Create API key modal
fn create_api_key_modal(org_id: uuid::Uuid, auto_open: bool) -> Markup {
    let modal_class = if auto_open {
        "fixed z-10 inset-0 overflow-y-auto"
    } else {
        "hidden fixed z-10 inset-0 overflow-y-auto"
    };

    html! {
        div
            id="create-key-modal"
            class=(modal_class)
            aria-labelledby="modal-title"
            role="dialog"
            aria-modal="true" {
            div class="flex items-end justify-center min-h-screen pt-4 px-4 pb-20 text-center sm:block sm:p-0" {
                div
                    onclick="document.getElementById('create-key-modal').classList.add('hidden')"
                    class="fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity"
                    aria-hidden="true" {}

                span class="hidden sm:inline-block sm:align-middle sm:h-screen" aria-hidden="true" { "\u{200B}" }

                div class="inline-block align-bottom bg-white rounded-lg px-4 pt-5 pb-4 text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-lg sm:w-full sm:p-6" {
                    div {
                        div class="mt-3 text-center sm:mt-0 sm:text-left" {
                            h3 class="text-lg leading-6 font-medium text-gray-900" id="modal-title" {
                                "Create New API Key"
                            }
                            div class="mt-4" {
                                form action=(format!("/organizations/{}/keys", org_id.simple())) method="POST" {
                                    div class="space-y-4" {
                                        div {
                                            label for="name" class="block text-sm font-medium text-gray-700" {
                                                "Key Name"
                                            }
                                            input
                                                type="text"
                                                name="name"
                                                id="name"
                                                required
                                                class="mt-1 block w-full border border-gray-300 rounded-md shadow-sm py-2 px-3 focus:outline-none focus:ring-primary focus:border-primary sm:text-sm"
                                                placeholder="Production API Key";
                                            p class="mt-1 text-xs text-gray-500" {
                                                "A descriptive name to help you identify this key"
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
                                            onclick="document.getElementById('create-key-modal').classList.add('hidden')"
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

/// Handle API key creation
pub async fn create(
    session: SessionCookie,
    Path(org_id): Path<DashlessUuid>,
    Form(form): Form<CreateAPIKeyForm>,
) -> Result<Response, Response> {
    let pool = database::get_db();
    let user_id = session.user_id();
    let org_id = org_id.into_inner();

    // Fetch all user's organizations for the dropdown
    let all_orgs = sqlx::query_as::<_, OrgListItem>(
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

    // Check user has access to this organization and get org name
    let org_info = sqlx::query_as::<_, OrgListItem>(
        "SELECT o.id, o.name FROM organizations o
         INNER JOIN organization_members om ON o.id = om.organization_id
         WHERE o.id = $1 AND om.user_id = $2",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
    })?
    .ok_or_else(|| (StatusCode::FORBIDDEN, "Access denied").into_response())?;

    // Get organization tier
    let org_tier =
        sqlx::query_scalar::<_, TierType>("SELECT tier FROM organizations WHERE id = $1")
            .bind(org_id)
            .fetch_one(pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch organization tier: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to fetch organization tier",
                )
                    .into_response()
            })?;

    // Generate UUIDv7 for the API key
    let key_id = Uuid::now_v7();

    // Get tier limits
    let (max_tokens, monthly_quota) = get_tier_limits(org_tier);

    // Create token data
    let token_data = TokenData {
        org_id,
        key_id,
        tier: org_tier,
        max_tokens: max_tokens as i32,
        monthly_quota,
    };

    // Sign the token
    let settings = crate::config::get_settings();
    let private_key_bytes = hex::decode(&settings.token_private_key).map_err(|e| {
        tracing::error!("Failed to decode private key: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to decode private key",
        )
            .into_response()
    })?;
    let signing_key = ed25519_dalek::SigningKey::from_bytes(
        &private_key_bytes[..32].try_into().map_err(|e| {
            tracing::error!("Invalid key length: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Invalid key length").into_response()
        })?,
    );

    let token = sign_token_direct(&token_data, &signing_key).map_err(|e| {
        tracing::error!("Failed to sign token: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to sign token").into_response()
    })?;

    let full_token = format!("fe_{}", token);

    // Save to database
    sqlx::query(
        "INSERT INTO api_keys (organization_id, key_id, name, is_active, created_at)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(org_id)
    .bind(key_id)
    .bind(&form.name)
    .bind(true)
    .bind(Utc::now().naive_utc())
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create API key: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create API key",
        )
            .into_response()
    })?;

    // Build organization dropdown data
    let current_org_id_simple = org_id.simple().to_string();
    let current_org_name = &org_info.name;

    let other_orgs: Vec<(String, String)> = all_orgs
        .iter()
        .filter(|o| o.id != org_id)
        .map(|o| (o.id.simple().to_string(), o.name.clone()))
        .collect();

    let other_orgs_refs: Vec<(&str, &str)> = other_orgs
        .iter()
        .map(|(id, name)| (id.as_str(), name.as_str()))
        .collect();

    // Show the token to the user (only once!)
    Ok((
        StatusCode::OK,
        layout::base("API Key Created", html! {
            (layout::navbar(
                session.email(),
                Some((current_org_id_simple.as_str(), current_org_name)),
                &other_orgs_refs
            ))
            (layout::container(html! {
                div class="max-w-2xl mx-auto" {
                    (layout::alert("API key created successfully! Copy it now - you won't be able to see it again.", "success"))

                    div class="mt-6 bg-white shadow rounded-lg p-6" {
                        h3 class="text-lg font-medium text-gray-900 mb-4" { "Your API Key" }
                        div class="bg-gray-50 rounded-md p-4 mb-4" {
                            code class="text-sm break-all" { (full_token) }
                        }
                        button
                            onclick=(format!("navigator.clipboard.writeText('{}'); this.textContent = 'Copied!'; setTimeout(() => this.textContent = 'Copy to Clipboard', 2000)", full_token))
                            class="inline-flex items-center px-4 py-2 border border-gray-300 shadow-sm text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary" {
                            "Copy to Clipboard"
                        }
                    }

                    div class="mt-6" {
                        a
                            href=(format!("/organizations/{}", org_id.simple()))
                            class="text-primary hover:text-blue-500" {
                            "← Back to organization"
                        }
                    }
                }
            }))
        }),
    ).into_response())
}

/// Handle API key revocation
pub async fn revoke(
    session: SessionCookie,
    Path((org_id, key_id)): Path<(DashlessUuid, DashlessUuid)>,
) -> Result<Response, Response> {
    let pool = database::get_db();
    let user_id = session.user_id();
    let org_id = org_id.into_inner();
    let key_id = key_id.into_inner();

    // Check user has access to this organization
    let _org = sqlx::query_scalar::<_, Uuid>(
        "SELECT o.id FROM organizations o
         INNER JOIN organization_members om ON o.id = om.organization_id
         WHERE o.id = $1 AND om.user_id = $2",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
    })?
    .ok_or_else(|| (StatusCode::FORBIDDEN, "Access denied").into_response())?;

    // Revoke the key
    sqlx::query("UPDATE api_keys SET is_active = false WHERE id = $1 AND organization_id = $2")
        .bind(key_id)
        .bind(org_id)
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to revoke API key: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to revoke API key",
            )
                .into_response()
        })?;

    // Redirect back to organization page
    Ok(Redirect::to(&format!("/organizations/{}", org_id.simple())).into_response())
}

/// Get tier limits for token creation
fn get_tier_limits(tier: TierType) -> (usize, i32) {
    let settings = config::get_settings();
    match tier {
        TierType::Free => (settings.max_tokens, settings.free_tier_limit),
        TierType::Pro => (settings.max_tokens, settings.pro_tier_limit),
        TierType::Scale => (settings.max_tokens, settings.scale_tier_limit),
    }
}
