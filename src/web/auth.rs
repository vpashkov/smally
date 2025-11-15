use axum::{
    extract::{Form, Query},
    http::{header, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use maud::{html, Markup};
use serde::Deserialize;

use crate::auth::session::{clear_session_cookie, create_session_cookie, create_session_token};
use crate::database;
use crate::models::{TierType, User};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;

use super::components::layout;

/// Validate redirect URL to prevent open redirect attacks
/// Only allows relative URLs starting with /
fn validate_redirect_url(url: &str) -> String {
    if url.starts_with('/') && !url.starts_with("//") {
        url.to_string()
    } else {
        "/organizations".to_string()
    }
}

/// Redirect query parameter
#[derive(Debug, Deserialize)]
pub struct RedirectQuery {
    pub next: Option<String>,
}

/// Login form data
#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
    pub next: Option<String>,
}

/// Register form data
#[derive(Debug, Deserialize)]
pub struct RegisterForm {
    pub email: String,
    pub password: String,
    pub name: String,
}

/// Show login page
pub async fn login_page(Query(redirect): Query<RedirectQuery>) -> Markup {
    layout::base(
        "Login",
        html! {
            div class="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8" {
                div class="max-w-md w-full space-y-8" {
                    div {
                        h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900" {
                            "Sign in to your account"
                        }
                        p class="mt-2 text-center text-sm text-gray-600" {
                            "Or "
                            a href="/register" class="font-medium text-primary hover:text-blue-500" {
                                "create a new account"
                            }
                        }
                    }

                    // Login form
                    form class="mt-8 space-y-6" action="/login" method="POST" {
                        input type="hidden" name="remember" value="true";
                        @if let Some(next) = redirect.next {
                            input type="hidden" name="next" value=(next);
                        }

                        div class="rounded-md shadow-sm -space-y-px" {
                            div {
                                label for="email" class="sr-only" { "Email address" }
                                input
                                    id="email"
                                    name="email"
                                    type="email"
                                    autocomplete="email"
                                    required
                                    class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-t-md focus:outline-none focus:ring-primary focus:border-primary focus:z-10 sm:text-sm"
                                    placeholder="Email address";
                            }
                            div {
                                label for="password" class="sr-only" { "Password" }
                                input
                                    id="password"
                                    name="password"
                                    type="password"
                                    autocomplete="current-password"
                                    required
                                    class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-b-md focus:outline-none focus:ring-primary focus:border-primary focus:z-10 sm:text-sm"
                                    placeholder="Password";
                            }
                        }

                        div class="flex items-center justify-between" {
                            div class="flex items-center" {
                                input
                                    id="remember-me"
                                    name="remember-me"
                                    type="checkbox"
                                    class="h-4 w-4 text-primary focus:ring-primary border-gray-300 rounded";
                                label for="remember-me" class="ml-2 block text-sm text-gray-900" {
                                    "Remember me"
                                }
                            }

                            div class="text-sm" {
                                a href="#" class="font-medium text-primary hover:text-blue-500" {
                                    "Forgot your password?"
                                }
                            }
                        }

                        div {
                            button
                                type="submit"
                                class="group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-primary hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary" {
                                "Sign in"
                            }
                        }
                    }
                }
            }
        },
    )
}

/// Show register page
pub async fn register_page() -> Markup {
    layout::base(
        "Register",
        html! {
            div class="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8" {
                div class="max-w-md w-full space-y-8" {
                    div {
                        h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900" {
                            "Create your account"
                        }
                        p class="mt-2 text-center text-sm text-gray-600" {
                            "Already have an account? "
                            a href="/login" class="font-medium text-primary hover:text-blue-500" {
                                "Sign in"
                            }
                        }
                    }

                    // Register form
                    form class="mt-8 space-y-6" action="/register" method="POST" {
                        div class="rounded-md shadow-sm space-y-4" {
                            div {
                                label for="name" class="block text-sm font-medium text-gray-700" { "Full name" }
                                input
                                    id="name"
                                    name="name"
                                    type="text"
                                    required
                                    class="mt-1 appearance-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-md focus:outline-none focus:ring-primary focus:border-primary sm:text-sm"
                                    placeholder="John Doe";
                            }
                            div {
                                label for="email" class="block text-sm font-medium text-gray-700" { "Email address" }
                                input
                                    id="email"
                                    name="email"
                                    type="email"
                                    autocomplete="email"
                                    required
                                    class="mt-1 appearance-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-md focus:outline-none focus:ring-primary focus:border-primary sm:text-sm"
                                    placeholder="you@example.com";
                            }
                            div {
                                label for="password" class="block text-sm font-medium text-gray-700" { "Password" }
                                input
                                    id="password"
                                    name="password"
                                    type="password"
                                    autocomplete="new-password"
                                    required
                                    class="mt-1 appearance-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-md focus:outline-none focus:ring-primary focus:border-primary sm:text-sm"
                                    placeholder="At least 8 characters";
                            }
                        }

                        div {
                            button
                                type="submit"
                                class="group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-primary hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary" {
                                "Create account"
                            }
                        }
                    }
                }
            }
        },
    )
}

/// Handle login form submission
pub async fn login_submit(Form(form): Form<LoginForm>) -> Result<Response, Response> {
    let pool = database::get_db();

    // Find user by email
    let user = sqlx::query_as!(
        User,
        "SELECT id, email, name, password_hash, is_active, last_selected_org_id, created_at, updated_at
         FROM users WHERE email = $1",
        &form.email
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
    })?
    .ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            layout::base(
                "Login Failed",
                html! {
                    div class="min-h-screen flex items-center justify-center bg-gray-50" {
                        div class="max-w-md w-full" {
                            (layout::alert("Invalid email or password", "error"))
                            a href="/login" class="text-primary hover:text-blue-500" {
                                "← Back to login"
                            }
                        }
                    }
                },
            ),
        )
            .into_response()
    })?;

    // Check if user is active
    if !user.is_active {
        return Err((
            StatusCode::UNAUTHORIZED,
            layout::base(
                "Account Disabled",
                html! {
                    div class="min-h-screen flex items-center justify-center bg-gray-50" {
                        div class="max-w-md w-full" {
                            (layout::alert("Your account has been disabled", "error"))
                            a href="/login" class="text-primary hover:text-blue-500" {
                                "← Back to login"
                            }
                        }
                    }
                },
            ),
        )
            .into_response());
    }

    // Verify password
    let password_hash = user.password_hash.as_ref().ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            "Invalid email or password".to_string(),
        )
            .into_response()
    })?;

    let valid = verify(&form.password, password_hash).map_err(|e| {
        tracing::error!("Password verification error: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Password verification failed",
        )
            .into_response()
    })?;

    if !valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            layout::base(
                "Login Failed",
                html! {
                    div class="min-h-screen flex items-center justify-center bg-gray-50" {
                        div class="max-w-md w-full" {
                            (layout::alert("Invalid email or password", "error"))
                            a href="/login" class="text-primary hover:text-blue-500" {
                                "← Back to login"
                            }
                        }
                    }
                },
            ),
        )
            .into_response());
    }

    // Generate session token
    let token = create_session_token(user.id, &user.email).map_err(|e| {
        tracing::error!("Failed to create session token: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create session",
        )
            .into_response()
    })?;

    // Create session cookie
    let cookie = create_session_cookie(&token);

    println!("User {} logged in", user.email);
    println!(
        "User last_selected_org_id {}",
        user.last_selected_org_id.unwrap_or_default()
    );
    println!("Redirect next {:?}", form.next);

    // Validate and determine redirect URL
    let redirect_url = if let Some(next) = form.next.as_deref() {
        // If explicit redirect URL provided, use it
        validate_redirect_url(next)
    } else if let Some(org_id) = user.last_selected_org_id {
        // Redirect to last selected organization
        format!("/organizations/{}", org_id.simple())
    } else {
        // Default to organizations list
        "/organizations".to_string()
    };

    // Return redirect with Set-Cookie header
    let mut response = Redirect::to(&redirect_url).into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, cookie.to_string().parse().unwrap());

    Ok(response)
}

/// Handle register form submission
pub async fn register_submit(Form(form): Form<RegisterForm>) -> Result<Response, Response> {
    let pool = database::get_db();

    // Check if user already exists
    let existing = sqlx::query_as!(
        User,
        "SELECT id, email, name, password_hash, is_active, last_selected_org_id, created_at, updated_at
         FROM users WHERE email = $1",
        &form.email
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
    })?;

    if existing.is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            layout::base(
                "Registration Failed",
                html! {
                    div class="min-h-screen flex items-center justify-center bg-gray-50" {
                        div class="max-w-md w-full" {
                            (layout::alert("Email already registered", "error"))
                            a href="/register" class="text-primary hover:text-blue-500" {
                                "← Back to registration"
                            }
                        }
                    }
                },
            ),
        )
            .into_response());
    }

    // Hash password
    let password_hash = hash(&form.password, DEFAULT_COST).map_err(|e| {
        tracing::error!("Password hashing failed: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Password hashing failed").into_response()
    })?;

    // Generate organization ID on server (using v7 for time-ordered UUIDs)
    let org_id = uuid::Uuid::now_v7();
    let now = Utc::now().naive_utc();

    // Create user with last_selected_org_id set to personal organization
    let user = sqlx::query_as!(
        User,
        "INSERT INTO users (email, name, password_hash, is_active, last_selected_org_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, email, name, password_hash, is_active, last_selected_org_id, created_at, updated_at",
        &form.email,
        &form.name,
        &password_hash,
        true,
        org_id,
        now,
        now
    )
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create user: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create user").into_response()
    })?;

    // Create personal organization with generated ID
    let org_name = format!("{}'s Organization", form.email);

    sqlx::query(
        "INSERT INTO organizations (id, name, owner_id, tier, is_active, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(org_id)
    .bind(&org_name)
    .bind(user.id)
    .bind(TierType::Free)
    .bind(true)
    .bind(now)
    .bind(now)
    .execute(pool)
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
    .bind(user.id)
    .bind("owner")
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

    // Generate session token
    let token = create_session_token(user.id, &user.email).map_err(|e| {
        tracing::error!("Failed to create session token: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create session",
        )
            .into_response()
    })?;

    // Create session cookie
    let cookie = create_session_cookie(&token);

    // Redirect to the personal organization page (not the list)
    let redirect_url = format!("/organizations/{}", org_id.simple());
    let mut response = Redirect::to(&redirect_url).into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, cookie.to_string().parse().unwrap());

    Ok(response)
}

/// Handle logout - clear session cookie and redirect to login
pub async fn logout_submit() -> Response {
    let cookie = clear_session_cookie();

    let mut response = Redirect::to("/login").into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, cookie.to_string().parse().unwrap());

    response
}
