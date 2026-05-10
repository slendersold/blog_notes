//! HTTP-транспорт `BlogClient`: публичный список постов против wiremock без бэкенда.

use blog_client::{BlogClient, Transport};

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn list_posts_via_client_http() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/posts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "posts": [{
                "id": 1,
                "title": "t",
                "content": "c",
                "author_id": 2,
                "author_username": "alice",
                "created_at": "2020-01-01T00:00:00Z",
                "updated_at": "2020-01-01T00:00:00Z"
            }],
            "total": 1,
            "limit": 20,
            "offset": 0
        })))
        .mount(&server)
        .await;

    let mut client = BlogClient::new(Transport::Http(server.uri().to_string()))
        .await
        .expect("client");
    let out = client.list_posts(20, 0).await.expect("list_posts");
    assert_eq!(out.total, 1);
    assert_eq!(out.posts.len(), 1);
    assert_eq!(out.posts[0].author_username, "alice");
}
