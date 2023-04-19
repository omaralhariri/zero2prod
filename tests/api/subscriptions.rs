use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let app = spawn_app().await;
    let body = "name=omar%20alhariri&email=o.alhariri%40outlook.com";
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;
    let response = app.post_subscriptions(body.into()).await;
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "o.alhariri@outlook.com");
    assert_eq!(saved.name, "omar alhariri");
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    let app = spawn_app().await;
    let body = "name=omar%20alhariri&email=o.alhariri%40outlook.com";
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;
    app.post_subscriptions(body.into()).await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "o.alhariri@outlook.com");
    assert_eq!(saved.name, "omar alhariri");
    assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
async fn subscribe_returns_400_for_invalid_form_data() {
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscriptions(invalid_body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The api did not fail when the payload was {}",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_400_when_fields_are_present_but_empty() {
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=test@rest.com", "empty name"),
        ("name=test&email=", "empty email"),
        ("name=&email=invalid-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        let response = app.post_subscriptions(body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API return 400 Bad Request when the payload was {}",
            description
        );
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_form_data() {
    let app = spawn_app().await;
    let body = "name=omar%20alhariri&email=o.alhariri%40outlook.com";
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let app = spawn_app().await;
    let body = "name=omar%20alhariri&email=o.alhariri%40outlook.com";
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confiramtion_links(email_request);

    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    let app = spawn_app().await;
    let body = "name=omar&email=omar%40example.com";

    sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscription_token;",)
        .execute(&app.db_pool)
        .await
        .unwrap();

    let response = app.post_subscriptions(body.into()).await;

    assert_eq!(response.status().as_u16(), 500);
}
