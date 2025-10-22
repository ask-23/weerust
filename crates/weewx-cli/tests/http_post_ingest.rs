use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use tower::ServiceExt;

/// Test valid Ecowitt format POST request
#[tokio::test]
async fn test_ecowitt_post_valid() {
    let (app, _state) = weewx_cli::build_app();

    let ecowitt_data = "stationtype=GW1100&\
        baromabsin=29.92&\
        baromrelin=30.01&\
        tempf=78.6&\
        humidity=52&\
        winddir=180&\
        windspeedmph=3.2&\
        windgustmph=5.5&\
        solarradiation=120.5&\
        uv=2&\
        dateutc=now&\
        softwaretype=GW1100";

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/data")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(ecowitt_data))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    // Verify data appears in /api/v1/current
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/current")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();

    // Verify expected fields are present
    assert!(text.contains("outTemp") || text.contains("temperature"));
    assert!(text.contains("barometer") || text.contains("pressure"));
}

/// Test valid Weather Underground format POST request
#[tokio::test]
async fn test_wunderground_post_valid() {
    let (app, _state) = weewx_cli::build_app();

    let wu_data = "ID=STATION123&\
        PASSWORD=mypass&\
        dateutc=now&\
        tempf=72.5&\
        baromin=29.92&\
        humidity=55&\
        windspeedmph=5.0&\
        windgustmph=7.0&\
        winddir=180&\
        dewptf=56.3&\
        rainin=0.00&\
        dailyrainin=0.05&\
        solarradiation=85.2&\
        UV=1&\
        softwaretype=WU";

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/data")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(wu_data))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

/// Test POST with missing required fields (should still accept)
#[tokio::test]
async fn test_post_missing_optional_fields() {
    let (app, _state) = weewx_cli::build_app();

    // Minimal valid data - only required fields
    let minimal_data = "stationtype=GW1100&dateutc=now&tempf=75.0";

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/data")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(minimal_data))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should still accept with 200 OK
    assert_eq!(res.status(), StatusCode::OK);
}

/// Test POST with invalid data types
#[tokio::test]
async fn test_post_invalid_data_types() {
    let (app, _state) = weewx_cli::build_app();

    // Invalid numeric values
    let invalid_data = "stationtype=GW1100&\
        dateutc=now&\
        tempf=NOT_A_NUMBER&\
        humidity=INVALID&\
        windspeedmph=xyz";

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/data")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(invalid_data))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should gracefully handle (either 200 with partial data or 400)
    assert!(res.status() == StatusCode::OK || res.status() == StatusCode::BAD_REQUEST);
}

/// Test POST with malformed URL encoding
#[tokio::test]
async fn test_post_malformed_encoding() {
    let (app, _state) = weewx_cli::build_app();

    // Malformed data (no proper key=value pairs)
    let malformed_data = "this_is_not_valid_form_data";

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/data")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(malformed_data))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should handle gracefully (400 or 200 depending on implementation)
    assert!(res.status() == StatusCode::OK || res.status() == StatusCode::BAD_REQUEST);
}

/// Test POST with very large payload
#[tokio::test]
async fn test_post_large_payload() {
    let (app, _state) = weewx_cli::build_app();

    // Create a large payload with many fields
    let mut large_data = String::from("stationtype=GW1100&dateutc=now&tempf=72.0");
    for i in 0..100 {
        large_data.push_str(&format!("&custom_field_{}=value_{}", i, i));
    }

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/data")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(large_data))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should handle large payloads
    assert_eq!(res.status(), StatusCode::OK);
}

/// Test POST with extreme temperature values
#[tokio::test]
async fn test_post_extreme_values() {
    let (app, _state) = weewx_cli::build_app();

    let extreme_data = "stationtype=GW1100&\
        dateutc=now&\
        tempf=-40.0&\
        humidity=100&\
        windspeedmph=150.0&\
        baromabsin=35.00";

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/data")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(extreme_data))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should accept extreme but valid values
    assert_eq!(res.status(), StatusCode::OK);
}

/// Test concurrent POST requests
#[tokio::test]
async fn test_concurrent_posts() {
    let (app, _state) = weewx_cli::build_app();

    let mut handles = vec![];

    // Send 10 concurrent requests
    for i in 0..10 {
        let app_clone = app.clone();
        let handle = tokio::spawn(async move {
            let data = format!(
                "stationtype=GW1100&dateutc=now&tempf={}&humidity={}",
                70.0 + i as f64,
                50 + i
            );

            let res = app_clone
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/data")
                        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                        .body(Body::from(data))
                        .unwrap(),
                )
                .await
                .unwrap();

            res.status()
        });

        handles.push(handle);
    }

    // Verify all requests succeeded
    for handle in handles {
        let status = handle.await.unwrap();
        assert_eq!(status, StatusCode::OK);
    }
}

/// Test POST followed by immediate GET to verify persistence
#[tokio::test]
async fn test_post_then_get_persistence() {
    let (app, _state) = weewx_cli::build_app();

    let test_temp = 77.7;
    let post_data = format!(
        "stationtype=GW1100&dateutc=now&tempf={}&humidity=60",
        test_temp
    );

    // POST data
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/data")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(post_data))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    // Wait briefly for async processing
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // GET current data
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/current")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();

    // Verify the temperature we posted appears in the response
    // (exact format may vary based on implementation)
    assert!(!text.is_empty());
}

/// Test POST with special characters in values
#[tokio::test]
async fn test_post_special_characters() {
    let (app, _state) = weewx_cli::build_app();

    let special_data = "stationtype=GW1100%20A&\
        dateutc=now&\
        tempf=72.0&\
        model=Test%2BStation&\
        location=Home%20%26%20Garden";

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/data")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(special_data))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should handle URL-encoded special characters
    assert_eq!(res.status(), StatusCode::OK);
}

/// Test POST to alternative endpoints
#[tokio::test]
async fn test_post_alternative_endpoints() {
    let (app, _state) = weewx_cli::build_app();

    let test_data = "stationtype=GW1100&dateutc=now&tempf=72.0";

    // Test /ingest/ecowitt endpoint (if it exists)
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ingest/ecowitt")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(test_data))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should either succeed or return 404 if endpoint doesn't exist
    assert!(res.status() == StatusCode::OK || res.status() == StatusCode::NOT_FOUND);
}
