use crate::helpers::TestApp;

#[actix_rt::test]
async fn player_returns_a_200_for_valid_form_data() {
    // Arrange
    let app = TestApp::spawn().await;
    let test_cases = vec![
        ("source=codeforces", "count all"),
        ("source=codeforces&max=1499", "count low"),
        ("source=codeforces&min=1500", "count high"),
        ("source=codeforces&min=-1000&max=900", "include negatives"),
    ];

    let mut counts = vec![];
    for (body, error_message) in test_cases {
        // Act
        let response = app.post_count(body.into()).await;

        // Assert
        assert_eq!(
            200,
            response.status().as_u16(),
            "The API did not succeed when the payload was {}.",
            error_message
        );
        let c: usize = response.json().await.expect("Failed to parse as JSON");
        assert!(c > 0);
        counts.push(c);
    }
    assert_eq!(counts[0], counts[1] + counts[2]);
}

#[actix_rt::test]
async fn player_returns_a_400_when_fields_are_invalid() {
    // Arrange
    let app = TestApp::spawn().await;
    let test_cases = vec![
        ("source=codeforces&min=a", "non-numeric min"),
        ("source=codeforces&max=a", "non-numeric max"),
        ("source=codeforces&min=1500&max=1400", "inverted range"),
    ];

    for (body, description) in test_cases {
        // Act
        let response = app.post_count(body.into()).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            description
        );
    }
}
