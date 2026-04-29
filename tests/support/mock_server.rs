//! Builder-style wrapper around `wiremock` that knows how to mount
//! IG-shaped responses from fixture files.
//!
//! Usage:
//!
//! ```ignore
//! let mock = IgMockServer::start().await
//!     .mount_login_v3()
//!     .mount_get("accounts", 1, "accounts/list_v1.json")
//!     .build();
//! let client = mock.client(); // pre-built IgClient pointing at the mock
//! ```
//!
//! Each test owns its own `IgMockServer`, so `cargo test` parallelism is safe.

use trading_ig::{Credentials, Environment, IgClient};
use url::Url;
use wiremock::matchers::{header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::fixtures;
use super::matchers::{HasApiKey, HasVersion};

pub struct IgMockServer {
    pub server: MockServer,
}

impl IgMockServer {
    pub async fn start() -> Self {
        Self {
            server: MockServer::start().await,
        }
    }

    pub fn base_url(&self) -> Url {
        Url::parse(&self.server.uri()).expect("wiremock returns a valid URL")
    }

    /// Access the underlying `wiremock::MockServer` for advanced mock setup.
    pub fn server(&self) -> &MockServer {
        &self.server
    }

    /// Build an [`IgClient`] pointing at this mock, with sensible defaults.
    pub fn client(&self) -> IgClient {
        IgClient::builder()
            .environment(Environment::Custom(self.base_url()))
            .api_key("test-api-key")
            .credentials(Credentials::password("test-user", "test-pass"))
            .build()
            .expect("client builds")
    }

    /// Mount a fixture-backed response for an authenticated GET.
    pub async fn mount_get(&self, path_str: &str, version: u8, fixture: &str) -> &Self {
        let body = fixtures::load(fixture);
        Mock::given(method("GET"))
            .and(path(path_str))
            .and(HasApiKey)
            .and(HasVersion(version))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Content-Type", "application/json; charset=UTF-8")
                    .set_body_string(body),
            )
            .mount(&self.server)
            .await;
        self
    }

    /// Mount the v3 login response.
    pub async fn mount_login_v3(&self) -> &Self {
        let body = fixtures::load("session/login_v3.json");
        Mock::given(method("POST"))
            .and(path("session"))
            .and(HasApiKey)
            .and(HasVersion(3))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Content-Type", "application/json; charset=UTF-8")
                    .set_body_string(body),
            )
            .mount(&self.server)
            .await;
        self
    }

    /// Mount the v2 login response (CST/XST in headers).
    pub async fn mount_login_v2(&self) -> &Self {
        let body = fixtures::load("session/login_v2.json");
        Mock::given(method("POST"))
            .and(path("session"))
            .and(HasApiKey)
            .and(HasVersion(2))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Content-Type", "application/json; charset=UTF-8")
                    .insert_header("CST", "demo-cst-token")
                    .insert_header("X-SECURITY-TOKEN", "demo-xst-token")
                    .set_body_string(body),
            )
            .mount(&self.server)
            .await;
        self
    }

    /// Mount a fixture-backed response for an authenticated POST.
    pub async fn mount_post(&self, path_str: &str, version: u8, fixture: &str) -> &Self {
        let body = fixtures::load(fixture);
        Mock::given(method("POST"))
            .and(path(path_str))
            .and(HasApiKey)
            .and(HasVersion(version))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Content-Type", "application/json; charset=UTF-8")
                    .set_body_string(body),
            )
            .mount(&self.server)
            .await;
        self
    }

    /// Mount a fixture-backed response for an authenticated PUT.
    pub async fn mount_put(&self, path_str: &str, version: u8, fixture: &str) -> &Self {
        let body = fixtures::load(fixture);
        Mock::given(method("PUT"))
            .and(path(path_str))
            .and(HasApiKey)
            .and(HasVersion(version))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Content-Type", "application/json; charset=UTF-8")
                    .set_body_string(body),
            )
            .mount(&self.server)
            .await;
        self
    }

    /// Mount an empty-body 200 for an authenticated DELETE.
    pub async fn mount_delete(&self, path_str: &str, version: u8) -> &Self {
        Mock::given(method("DELETE"))
            .and(path(path_str))
            .and(HasApiKey)
            .and(HasVersion(version))
            .respond_with(ResponseTemplate::new(200))
            .mount(&self.server)
            .await;
        self
    }

    /// Mount a fixture-backed JSON response for an authenticated DELETE.
    pub async fn mount_delete_json(&self, path_str: &str, version: u8, fixture: &str) -> &Self {
        let body = fixtures::load(fixture);
        Mock::given(method("DELETE"))
            .and(path(path_str))
            .and(HasApiKey)
            .and(HasVersion(version))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Content-Type", "application/json; charset=UTF-8")
                    .set_body_string(body),
            )
            .mount(&self.server)
            .await;
        self
    }

    /// Mount an arbitrary error response.
    pub async fn mount_error(&self, http_method: &str, path_str: &str, status: u16, error_code: &str) -> &Self {
        Mock::given(method(http_method))
            .and(path(path_str))
            .and(header_exists("X-IG-API-KEY"))
            .respond_with(
                ResponseTemplate::new(status)
                    .insert_header("Content-Type", "application/json; charset=UTF-8")
                    .set_body_string(format!(r#"{{"errorCode":"{error_code}"}}"#)),
            )
            .mount(&self.server)
            .await;
        self
    }
}
