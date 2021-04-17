use crate::helpers::TestApp;
use worldrank_api::domain::PlayerSummary;

#[actix_rt::test]
async fn top_returns_a_200_for_valid_form_data() {
    // Arrange
    let app = TestApp::spawn().await;
    let body = "source=codeforces&start=0&many=10";

    // Act
    let response = app.post_top(body.into()).await;

    // Assert
    assert_eq!(200, response.status().as_u16());

    let top: Vec<PlayerSummary> = response.json().await.expect("Failed to parse as JSON");
    assert_eq!(top.len(), 10);
}

#[actix_rt::test]
async fn top_returns_a_400_when_data_is_missing() {
    // Arrange
    let app = TestApp::spawn().await;
    let test_cases = vec![("source=codeforces&start=0", "missing many")];

    for (body, error_message) in test_cases {
        // Act
        let response = app.post_top(body.into()).await;

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
async fn top_returns_a_400_when_fields_are_present_but_empty() {
    // Arrange
    let app = TestApp::spawn().await;
    let test_cases = vec![
        ("source=codeforces&start=&many=10", "empty start"),
        ("source=codeforces&start=a&many=10", "non-numeric start"),
        ("source=codeforces&start=0&many=", "empty many"),
        ("source=codeforces&start=0&many=a", "non-numeric many"),
        ("source=codeforces&start=987654321&many=10", "start too big"),
    ];

    for (body, description) in test_cases {
        // Act
        let response = app.post_top(body.into()).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            description
        );
    }
}
