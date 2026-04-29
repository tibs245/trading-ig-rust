use wiremock::Match;
use wiremock::Request;

/// Asserts that the request includes a non-empty `X-IG-API-KEY` header.
pub struct HasApiKey;

impl Match for HasApiKey {
    fn matches(&self, request: &Request) -> bool {
        request
            .headers
            .get("X-IG-API-KEY")
            .is_some_and(|v| !v.is_empty())
    }
}

/// Asserts the `Version` header matches the expected value.
pub struct HasVersion(pub u8);

impl Match for HasVersion {
    fn matches(&self, request: &Request) -> bool {
        request
            .headers
            .get("Version")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u8>().ok())
            .is_some_and(|v| v == self.0)
    }
}

/// Asserts that an `Authorization: Bearer …` header is present.
pub struct HasBearer;

impl Match for HasBearer {
    fn matches(&self, request: &Request) -> bool {
        request
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|s| s.starts_with("Bearer "))
    }
}

/// Asserts that CST + X-SECURITY-TOKEN headers are present (v1/v2 sessions).
pub struct HasCstHeaders;

impl Match for HasCstHeaders {
    fn matches(&self, request: &Request) -> bool {
        request.headers.contains_key("CST") && request.headers.contains_key("X-SECURITY-TOKEN")
    }
}
