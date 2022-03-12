use crate::helpers::TestApp;

#[tokio::test]
async fn top_returns_a_200_for_valid_form_data() {
    // Arrange
    let app = TestApp::spawn().await;
    let body = "source=codeforces&query=EbTec&many=10";

    // Act
    let response = app.post_autocomplete(body.into()).await;

    // Assert
    assert_eq!(200, response.status().as_u16());

    let top: Vec<String> = response.json().await.expect("Failed to parse as JSON");
    assert_eq!(top, vec!["EbTech"]);
}

#[tokio::test]
async fn top_returns_a_400_when_data_is_missing() {
    // Arrange
    let app = TestApp::spawn().await;
    let test_cases = vec![
        ("source=codeforces&query=EbTec", "missing many"),
        ("source=codeforces&many=10", "missing query"),
    ];

    for (body, error_message) in test_cases {
        // Act
        let response = app.post_autocomplete(body.into()).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}
