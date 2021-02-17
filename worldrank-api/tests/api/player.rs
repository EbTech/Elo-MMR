use crate::helpers::TestApp;
use worldrank_api::domain::HistoryPoint;

#[actix_rt::test]
async fn player_returns_a_200_for_valid_form_data() {
    // Arrange
    let app = TestApp::spawn().await;
    let body = "handle=tourist";

    // Act
    let response = app.post_player(body.into()).await;

    // Assert
    assert_eq!(200, response.status().as_u16());

    let history: Vec<HistoryPoint> = response.json().await.expect("Failed to parse as JSON");
    assert!(!history.is_empty());
}

#[actix_rt::test]
async fn player_returns_a_400_when_data_is_missing() {
    // Arrange
    let app = TestApp::spawn().await;
    let test_cases = vec![("", "missing handle")];

    for (body, error_message) in test_cases {
        // Act
        let response = app.post_player(body.into()).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[actix_rt::test]
async fn player_returns_a_400_when_fields_are_present_but_empty() {
    // Arrange
    let app = TestApp::spawn().await;
    let test_cases = vec![("handle=", "empty handle"), ("handle=<>", "invalid handle")];

    for (body, description) in test_cases {
        // Act
        let response = app.post_player(body.into()).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            description
        );
    }
}
