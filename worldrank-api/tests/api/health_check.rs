use crate::helpers::TestApp;

#[actix_rt::test]
async fn health_check_works() {
    // Arrange
    let app = TestApp::spawn().await;

    // Act
    let response = app.post_health_check().await;

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
