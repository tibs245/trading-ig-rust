//! Endpoint methods for the history domain.
//!
//! Accessed via [`crate::IgClient::history`] → [`HistoryApi`].

use http::Method;

use crate::error::Result;
use crate::time::{ApiVersion, format as fmt_dt};

use super::HistoryApi;
use super::models::{
    Activity, ActivityPage, ActivityRequest, ActivityV1, ActivityV1Envelope, Transaction,
    TransactionType, TransactionsRequest, TransactionsResponse,
};

impl HistoryApi<'_> {
    // ────────────────────────────────────────────────────────────────────────
    // Activity v3
    // ────────────────────────────────────────────────────────────────────────

    /// Fetch all activities matching `req` from `GET /history/activity` (v3).
    ///
    /// Automatically follows `metadata.paging.next` until all pages have been
    /// collected and returns the combined list.
    ///
    /// # Pagination
    ///
    /// IG returns a `metadata.paging.next` URL when more results are
    /// available. Each subsequent request uses the query parameters embedded
    /// in that URL. The host portion is ignored — the configured environment
    /// base URL is always used — so pagination works correctly in tests
    /// (pointing at the mock server).
    ///
    /// # FIQL filter
    ///
    /// Use `req.filter` to pass a raw FIQL expression. Operators:
    /// `==` equals, `!=` not-equals, `,` OR, `;` AND.
    /// Example: `"type==POSITION;status==ACCEPTED"`.
    #[tracing::instrument(skip_all, fields(page_size = req.page_size, detailed = req.detailed))]
    pub async fn activity_v3(&self, req: ActivityRequest) -> Result<Vec<Activity>> {
        // Build initial query params from the request struct.
        let mut params: Vec<(String, String)> = Vec::new();
        if let Some(from) = req.from {
            params.push(("from".into(), fmt_dt(from, ApiVersion::V3)));
        }
        if let Some(to) = req.to {
            params.push(("to".into(), fmt_dt(to, ApiVersion::V3)));
        }
        if req.detailed {
            params.push(("detailed".into(), "true".into()));
        }
        if let Some(deal_id) = &req.deal_id {
            params.push(("dealId".into(), deal_id.0.clone()));
        }
        if let Some(filter) = &req.filter {
            params.push(("filter".into(), filter.clone()));
        }
        params.push(("pageSize".into(), req.page_size.to_string()));

        let mut all_activities: Vec<Activity> = Vec::new();

        // `next_params` drives the loop: Some(params) means "fetch a page";
        // None means we've consumed all pages.
        let mut next_params: Option<Vec<(String, String)>> = Some(params);

        while let Some(qp) = next_params.take() {
            let path = build_path_with_params("history/activity", &qp);

            let page: ActivityPage = self
                .client
                .transport
                .request(
                    Method::GET,
                    &path,
                    Some(3),
                    None::<&()>,
                    &self.client.session,
                )
                .await?;

            all_activities.extend(page.activities);

            // Follow the next-page URL if present.
            next_params = page
                .metadata
                .paging
                .next
                .map(|url| extract_query_params(&url))
                .transpose()?;
        }

        Ok(all_activities)
    }

    // ────────────────────────────────────────────────────────────────────────
    // Activity v1 — period
    // ────────────────────────────────────────────────────────────────────────

    /// Fetch activities for the last `milliseconds` milliseconds from now.
    ///
    /// Uses `GET /history/activity/{milliseconds}` (v1).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn run(client: &trading_ig::IgClient) -> trading_ig::Result<()> {
    /// // Last 24 hours:
    /// let acts = client.history().activity_by_period_v1(86_400_000).await?;
    /// # Ok(()) }
    /// ```
    #[tracing::instrument(skip_all, fields(milliseconds))]
    pub async fn activity_by_period_v1(&self, milliseconds: u64) -> Result<Vec<ActivityV1>> {
        let path = format!("history/activity/{milliseconds}");
        let env: ActivityV1Envelope = self
            .client
            .transport
            .request(
                Method::GET,
                &path,
                Some(1),
                None::<&()>,
                &self.client.session,
            )
            .await?;
        Ok(env.activities)
    }

    // ────────────────────────────────────────────────────────────────────────
    // Activity v1 — date range
    // ────────────────────────────────────────────────────────────────────────

    /// Fetch activities between two dates.
    ///
    /// Uses `GET /history/activity/{fromDate}/{toDate}` (v1).
    /// Dates are formatted in the v1 format (`YYYY:MM:DD-HH:MM:SS`).
    #[tracing::instrument(skip_all, fields(%from, %to))]
    pub async fn activity_by_date_range_v1(
        &self,
        from: chrono::NaiveDateTime,
        to: chrono::NaiveDateTime,
    ) -> Result<Vec<ActivityV1>> {
        let from_str = fmt_dt(from, ApiVersion::V1);
        let to_str = fmt_dt(to, ApiVersion::V1);
        let path = format!("history/activity/{from_str}/{to_str}");
        let env: ActivityV1Envelope = self
            .client
            .transport
            .request(
                Method::GET,
                &path,
                Some(1),
                None::<&()>,
                &self.client.session,
            )
            .await?;
        Ok(env.activities)
    }

    // ────────────────────────────────────────────────────────────────────────
    // Transactions v2
    // ────────────────────────────────────────────────────────────────────────

    /// Fetch transactions for the given request parameters.
    ///
    /// Uses `GET /history/transactions` (v2). Returns a single page of results
    /// plus paging metadata. Use `req.page_number` to iterate pages.
    #[tracing::instrument(skip_all)]
    pub async fn transactions_v2(&self, req: TransactionsRequest) -> Result<TransactionsResponse> {
        let mut params: Vec<(String, String)> = Vec::new();
        if let Some(tt) = req.trans_type {
            params.push(("type".into(), transaction_type_str(tt)));
        }
        if let Some(from) = req.from {
            params.push(("from".into(), fmt_dt(from, ApiVersion::V3)));
        }
        if let Some(to) = req.to {
            params.push(("to".into(), fmt_dt(to, ApiVersion::V3)));
        }
        if let Some(max) = req.max_span_seconds {
            params.push(("maxSpanSeconds".into(), max.to_string()));
        }
        if let Some(ps) = req.page_size {
            params.push(("pageSize".into(), ps.to_string()));
        }
        if let Some(pn) = req.page_number {
            params.push(("pageNumber".into(), pn.to_string()));
        }

        let path = build_path_with_params("history/transactions", &params);

        self.client
            .transport
            .request(
                Method::GET,
                &path,
                Some(2),
                None::<&()>,
                &self.client.session,
            )
            .await
    }

    // ────────────────────────────────────────────────────────────────────────
    // Transactions v1 — period
    // ────────────────────────────────────────────────────────────────────────

    /// Fetch transactions of `trans_type` for the last `milliseconds` ms.
    ///
    /// Uses `GET /history/transactions/{type}/{milliseconds}` (v1).
    ///
    /// Type values: [`TransactionType::All`], [`TransactionType::AllDeal`],
    /// [`TransactionType::Deposit`], [`TransactionType::Withdrawal`].
    #[tracing::instrument(skip_all, fields(?trans_type, milliseconds))]
    pub async fn transactions_by_period_v1(
        &self,
        trans_type: TransactionType,
        milliseconds: u64,
    ) -> Result<Vec<Transaction>> {
        let type_str = transaction_type_str(trans_type);
        let path = format!("history/transactions/{type_str}/{milliseconds}");

        #[derive(serde::Deserialize)]
        struct Envelope {
            transactions: Vec<Transaction>,
        }

        let env: Envelope = self
            .client
            .transport
            .request(
                Method::GET,
                &path,
                Some(1),
                None::<&()>,
                &self.client.session,
            )
            .await?;
        Ok(env.transactions)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────────

/// Serialize a [`TransactionType`] to the `SCREAMING_SNAKE_CASE` string IG expects.
fn transaction_type_str(tt: TransactionType) -> String {
    serde_json::to_value(tt)
        .ok()
        .and_then(|v| v.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "ALL".to_owned())
}

/// Build a request path with an appended query string from `params`.
/// If `params` is empty, returns `base` unchanged.
fn build_path_with_params(base: &str, params: &[(String, String)]) -> String {
    if params.is_empty() {
        return base.to_owned();
    }
    let qs = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding_simple(v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{base}?{qs}")
}

/// Parse query parameters from a next-page URL returned by IG.
///
/// The URL may be absolute (e.g. `https://api.ig.com/gateway/deal/…?…`) or
/// relative (e.g. `history/activity?…`). In both cases we extract only the
/// query string — the path component is always `history/activity` and the host
/// is replaced with the configured environment base URL by `Transport`.
fn extract_query_params(url: &str) -> Result<Vec<(String, String)>> {
    // Try parsing as an absolute URL first; fall back to prepending a dummy
    // scheme+host so the relative form `path/to?k=v` also parses.
    let parsed = url::Url::parse(url).or_else(|_| {
        let with_base = if url.starts_with('/') {
            format!("http://ig.invalid{url}")
        } else {
            format!("http://ig.invalid/{url}")
        };
        url::Url::parse(&with_base)
    })?;

    Ok(parsed
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect())
}

/// Minimal percent-encoding for query-string values.
///
/// Only encodes characters that MUST be encoded in a query value; leaves
/// unreserved characters (RFC 3986 §2.3), digits, and common safe characters
/// unencoded. This avoids a dependency on the `percent-encoding` crate.
fn urlencoding_simple(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            // Unreserved per RFC 3986 plus characters safe in query values.
            // Colons are left unencoded because ISO-8601 timestamps contain them.
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~'
            | b':'
            | b'!'
            | b'*'
            | b'\'' => out.push(b as char),
            b' ' => out.push('+'),
            other => {
                out.push('%');
                out.push(HEX_CHARS[(other >> 4) as usize]);
                out.push(HEX_CHARS[(other & 0xF) as usize]);
            }
        }
    }
    out
}

const HEX_CHARS: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
];
