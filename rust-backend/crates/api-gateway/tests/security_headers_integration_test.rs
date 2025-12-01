//! Integration tests for Security Headers Middleware
//!
//! These tests verify that security headers are properly applied to all API endpoints
//! in a production-like environment.

use actix_web::{test, web, App, HttpResponse};
use api_gateway::middleware::security_headers::{SecurityHeaders, SecurityHeadersConfig};

/// Test handler for health check
async fn health_handler() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy"
    }))
}

/// Test handler for triggers endpoint
async fn triggers_handler() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "data": [],
        "pagination": {
            "page": 1,
            "page_size": 20,
            "total_pages": 0,
            "total_items": 0
        }
    }))
}

/// Test handler for discovery endpoint
async fn discovery_handler() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "name": "API 8004 Backend",
        "version": "1.0.0"
    }))
}

#[actix_web::test]
async fn test_security_headers_on_health_endpoint() {
    let app = test::init_service(
        App::new()
            .wrap(SecurityHeaders::for_api())
            .route("/api/v1/health", web::get().to(health_handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/v1/health").to_request();
    let resp = test::call_service(&app, req).await;

    // Verify all security headers are present
    assert!(resp.headers().contains_key("x-content-type-options"));
    assert!(resp.headers().contains_key("x-frame-options"));
    assert!(resp.headers().contains_key("x-xss-protection"));
    assert!(resp.headers().contains_key("referrer-policy"));
    assert!(resp.headers().contains_key("permissions-policy"));
    assert!(resp.headers().contains_key("cross-origin-embedder-policy"));
    assert!(resp.headers().contains_key("cross-origin-opener-policy"));
    assert!(resp.headers().contains_key("cross-origin-resource-policy"));

    // Verify response is still valid JSON
    assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
}

#[actix_web::test]
async fn test_security_headers_on_triggers_endpoint() {
    let app = test::init_service(
        App::new()
            .wrap(SecurityHeaders::for_api())
            .route("/api/v1/triggers", web::get().to(triggers_handler)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/triggers")
        .to_request();
    let resp = test::call_service(&app, req).await;

    // Verify all security headers are present
    assert!(resp.headers().contains_key("x-content-type-options"));
    assert!(resp.headers().contains_key("cross-origin-embedder-policy"));

    // Verify response is still valid JSON
    assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
}

#[actix_web::test]
async fn test_security_headers_on_discovery_endpoint() {
    let app = test::init_service(
        App::new()
            .wrap(SecurityHeaders::for_api())
            .route("/.well-known/agent.json", web::get().to(discovery_handler)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/.well-known/agent.json")
        .to_request();
    let resp = test::call_service(&app, req).await;

    // Verify all security headers are present
    assert!(resp.headers().contains_key("x-content-type-options"));
    assert!(resp.headers().contains_key("cross-origin-embedder-policy"));

    // Verify response is still valid JSON
    assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
}

#[actix_web::test]
async fn test_hsts_production_config() {
    let config = SecurityHeadersConfig {
        enable_hsts: true,
        hsts_max_age: 31_536_000,
        hsts_include_subdomains: true,
        hsts_preload: false,
        frame_options: "DENY".to_string(),
        content_security_policy: None,
        referrer_policy: "strict-origin-when-cross-origin".to_string(),
    };

    let app = test::init_service(
        App::new()
            .wrap(SecurityHeaders::new(config))
            .route("/api/v1/health", web::get().to(health_handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/v1/health").to_request();
    let resp = test::call_service(&app, req).await;

    // Verify HSTS is enabled
    assert!(resp.headers().contains_key("strict-transport-security"));
    let hsts = resp
        .headers()
        .get("strict-transport-security")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(hsts.contains("max-age=31536000"));
    assert!(hsts.contains("includeSubDomains"));
    assert!(!hsts.contains("preload"));
}

#[actix_web::test]
async fn test_multiple_endpoints_have_consistent_headers() {
    let app = test::init_service(
        App::new()
            .wrap(SecurityHeaders::for_api())
            .route("/api/v1/health", web::get().to(health_handler))
            .route("/api/v1/triggers", web::get().to(triggers_handler))
            .route("/.well-known/agent.json", web::get().to(discovery_handler)),
    )
    .await;

    let endpoints = vec![
        "/api/v1/health",
        "/api/v1/triggers",
        "/.well-known/agent.json",
    ];

    for endpoint in endpoints {
        let req = test::TestRequest::get().uri(endpoint).to_request();
        let resp = test::call_service(&app, req).await;

        // All endpoints should have the same security headers
        assert!(
            resp.headers().contains_key("x-content-type-options"),
            "Missing x-content-type-options on {}",
            endpoint
        );
        assert!(
            resp.headers().contains_key("x-frame-options"),
            "Missing x-frame-options on {}",
            endpoint
        );
        assert!(
            resp.headers().contains_key("cross-origin-embedder-policy"),
            "Missing cross-origin-embedder-policy on {}",
            endpoint
        );

        // Verify values are consistent
        assert_eq!(
            resp.headers().get("x-content-type-options").unwrap(),
            "nosniff",
            "Inconsistent x-content-type-options on {}",
            endpoint
        );
        assert_eq!(
            resp.headers().get("x-frame-options").unwrap(),
            "DENY",
            "Inconsistent x-frame-options on {}",
            endpoint
        );
    }
}

#[actix_web::test]
async fn test_error_responses_have_security_headers() {
    async fn error_handler() -> HttpResponse {
        HttpResponse::BadRequest().json(serde_json::json!({
            "error": {
                "code": "bad_request",
                "message": "Invalid request"
            }
        }))
    }

    let app = test::init_service(
        App::new()
            .wrap(SecurityHeaders::for_api())
            .route("/error", web::get().to(error_handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/error").to_request();
    let resp = test::call_service(&app, req).await;

    // Even error responses should have security headers
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    assert!(resp.headers().contains_key("x-content-type-options"));
    assert!(resp.headers().contains_key("cross-origin-embedder-policy"));
}

#[actix_web::test]
async fn test_post_requests_have_security_headers() {
    async fn create_handler() -> HttpResponse {
        HttpResponse::Created().json(serde_json::json!({
            "id": "trigger-123",
            "name": "New Trigger"
        }))
    }

    let app = test::init_service(
        App::new()
            .wrap(SecurityHeaders::for_api())
            .route("/api/v1/triggers", web::post().to(create_handler)),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/triggers")
        .set_json(serde_json::json!({
            "name": "Test Trigger"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    // POST requests should also have security headers
    assert_eq!(resp.status(), actix_web::http::StatusCode::CREATED);
    assert!(resp.headers().contains_key("x-content-type-options"));
    assert!(resp.headers().contains_key("cross-origin-embedder-policy"));
}

#[actix_web::test]
async fn test_no_csp_for_api_config() {
    let app = test::init_service(
        App::new()
            .wrap(SecurityHeaders::for_api())
            .route("/api/test", web::get().to(health_handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/test").to_request();
    let resp = test::call_service(&app, req).await;

    // API config should NOT have CSP (APIs don't serve HTML/JS)
    assert!(!resp.headers().contains_key("content-security-policy"));

    // But should have all other headers
    assert!(resp.headers().contains_key("x-content-type-options"));
    assert!(resp.headers().contains_key("cross-origin-embedder-policy"));
}

#[actix_web::test]
async fn test_default_config_has_csp() {
    let app = test::init_service(
        App::new()
            .wrap(SecurityHeaders::default())
            .route("/test", web::get().to(health_handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/test").to_request();
    let resp = test::call_service(&app, req).await;

    // Default config SHOULD have CSP
    assert!(resp.headers().contains_key("content-security-policy"));
    let csp = resp
        .headers()
        .get("content-security-policy")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(csp.contains("default-src 'self'"));
}

#[actix_web::test]
async fn test_permissions_policy_disables_dangerous_features() {
    let app = test::init_service(
        App::new()
            .wrap(SecurityHeaders::for_api())
            .route("/test", web::get().to(health_handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/test").to_request();
    let resp = test::call_service(&app, req).await;

    let permissions = resp
        .headers()
        .get("permissions-policy")
        .unwrap()
        .to_str()
        .unwrap();

    // Verify all dangerous features are disabled
    assert!(permissions.contains("geolocation=()"));
    assert!(permissions.contains("camera=()"));
    assert!(permissions.contains("microphone=()"));
    assert!(permissions.contains("payment=()"));
    assert!(permissions.contains("usb=()"));
    assert!(permissions.contains("accelerometer=()"));
    assert!(permissions.contains("gyroscope=()"));
    assert!(permissions.contains("magnetometer=()"));
}

#[actix_web::test]
async fn test_cross_origin_policies_all_same_origin() {
    let app = test::init_service(
        App::new()
            .wrap(SecurityHeaders::for_api())
            .route("/test", web::get().to(health_handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/test").to_request();
    let resp = test::call_service(&app, req).await;

    // All cross-origin policies should be restrictive
    assert_eq!(
        resp.headers()
            .get("cross-origin-embedder-policy")
            .unwrap()
            .to_str()
            .unwrap(),
        "require-corp"
    );
    assert_eq!(
        resp.headers()
            .get("cross-origin-opener-policy")
            .unwrap()
            .to_str()
            .unwrap(),
        "same-origin"
    );
    assert_eq!(
        resp.headers()
            .get("cross-origin-resource-policy")
            .unwrap()
            .to_str()
            .unwrap(),
        "same-origin"
    );
}

#[actix_web::test]
async fn test_referrer_policy_protects_privacy() {
    let app = test::init_service(
        App::new()
            .wrap(SecurityHeaders::for_api())
            .route("/test", web::get().to(health_handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/test").to_request();
    let resp = test::call_service(&app, req).await;

    // Referrer policy should protect user privacy
    assert_eq!(
        resp.headers()
            .get("referrer-policy")
            .unwrap()
            .to_str()
            .unwrap(),
        "strict-origin-when-cross-origin"
    );
}

#[actix_web::test]
async fn test_xss_protection_enabled() {
    let app = test::init_service(
        App::new()
            .wrap(SecurityHeaders::for_api())
            .route("/test", web::get().to(health_handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/test").to_request();
    let resp = test::call_service(&app, req).await;

    // XSS protection should be enabled with block mode
    assert_eq!(
        resp.headers()
            .get("x-xss-protection")
            .unwrap()
            .to_str()
            .unwrap(),
        "1; mode=block"
    );
}
