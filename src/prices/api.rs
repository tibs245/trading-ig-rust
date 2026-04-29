//! Endpoint methods for the historical prices domain.

use std::time::Duration;

use chrono::NaiveDateTime;
use http::Method;
use tracing::info;

use crate::Result;
use crate::models::common::Epic;
use crate::time::{self, ApiVersion};

use super::models::{HistoricalPrices, HistoricalPricesRequest, Resolution};
use super::PricesApi;

impl PricesApi<'_> {
    /// Fetch one page of historical prices using the v3 flexible endpoint.
    ///
    /// `GET /prices/{epic}?resolution=…&from=…&to=…&max=…&pageSize=…&pageNumber=…`
    ///
    /// All request fields are optional. The server defaults to `MINUTE`
    /// resolution and page 1 with 20 points per page.
    ///
    /// # Errors
    ///
    /// Returns `Error::Api` when IG rejects the request (e.g.
    /// `error.public-api.exceeded-account-historical-data-allowance`).
    #[tracing::instrument(skip_all, fields(epic = %epic))]
    pub async fn history_v3(
        &self,
        epic: &Epic,
        req: HistoricalPricesRequest,
    ) -> Result<HistoricalPrices> {
        let result = self.fetch_v3_page(epic, &req).await?;
        info!(
            remaining_allowance = result.metadata.allowance.remaining_allowance,
            allowance_expiry    = result.metadata.allowance.allowance_expiry,
            "prices v3 allowance"
        );
        Ok(result)
    }

    /// Fetch **all** pages of a v3 historical-prices query, combining them
    /// into a single [`HistoricalPrices`] value.
    ///
    /// Pages are fetched sequentially with a 1-second delay between requests
    /// (matching the Python library's default). The `metadata` field of the
    /// returned value reflects the **last** page fetched; `prices` contains
    /// the combined points from all pages.
    ///
    /// Shares the same underlying page-fetch logic as [`Self::history_v3`].
    ///
    /// # Errors
    ///
    /// Returns `Error::Api` on the first failed page fetch.
    #[tracing::instrument(skip_all, fields(epic = %epic))]
    pub async fn history_v3_all(
        &self,
        epic: &Epic,
        req: HistoricalPricesRequest,
    ) -> Result<HistoricalPrices> {
        // Always start from page 1.
        let first_req = HistoricalPricesRequest {
            page_number: Some(1),
            ..req
        };
        let first = self.fetch_v3_page(epic, &first_req).await?;
        info!(
            remaining_allowance = first.metadata.allowance.remaining_allowance,
            allowance_expiry    = first.metadata.allowance.allowance_expiry,
            page = 1,
            "prices v3 allowance"
        );

        let total_pages = first.metadata.page_data.total_pages;
        if total_pages <= 1 {
            return Ok(first);
        }

        // instrument_type is the same on every page; capture it from page 1.
        let instrument_type = first.instrument_type.clone();
        let mut all_prices = first.prices;
        let mut last_meta = first.metadata;

        for page_num in 2..=total_pages {
            tokio::time::sleep(Duration::from_secs(1)).await;

            let page_req = HistoricalPricesRequest {
                page_number: Some(page_num),
                ..req
            };
            let page = self.fetch_v3_page(epic, &page_req).await?;
            info!(
                remaining_allowance = page.metadata.allowance.remaining_allowance,
                allowance_expiry    = page.metadata.allowance.allowance_expiry,
                page = page_num,
                "prices v3 allowance"
            );
            all_prices.extend(page.prices);
            last_meta = page.metadata;
        }

        Ok(HistoricalPrices {
            instrument_type,
            prices: all_prices,
            metadata: last_meta,
        })
    }

    /// Internal: fetch a single v3 page. Shared by [`Self::history_v3`] and
    /// [`Self::history_v3_all`].
    async fn fetch_v3_page(
        &self,
        epic: &Epic,
        req: &HistoricalPricesRequest,
    ) -> Result<HistoricalPrices> {
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(r) = req.resolution {
            params.push(("resolution", r.to_string()));
        }
        if let Some(dt) = req.from {
            params.push(("from", time::format(dt, ApiVersion::V3)));
        }
        if let Some(dt) = req.to {
            params.push(("to", time::format(dt, ApiVersion::V3)));
        }
        if let Some(m) = req.max {
            params.push(("max", m.to_string()));
        }
        if let Some(ps) = req.page_size {
            params.push(("pageSize", ps.to_string()));
        }
        if let Some(pn) = req.page_number {
            params.push(("pageNumber", pn.to_string()));
        }

        let path = build_query_path(&format!("prices/{epic}"), &params);
        self.client
            .transport
            .request(Method::GET, &path, Some(3), None::<&()>, &self.client.session)
            .await
    }

    /// Fetch up to `num_points` bars at the given `resolution` (v2 endpoint).
    ///
    /// `GET /prices/{epic}/{resolution}/{numpoints}`
    ///
    /// # Errors
    ///
    /// Returns `Error::Api` when IG rejects the request.
    #[tracing::instrument(skip_all, fields(epic = %epic, resolution = %resolution, num_points))]
    pub async fn history_by_num_points_v2(
        &self,
        epic: &Epic,
        resolution: Resolution,
        num_points: u32,
    ) -> Result<HistoricalPrices> {
        let path = format!("prices/{epic}/{resolution}/{num_points}");
        let result: HistoricalPrices = self
            .client
            .transport
            .request(Method::GET, &path, Some(2), None::<&()>, &self.client.session)
            .await?;
        info!(
            remaining_allowance = result.metadata.allowance.remaining_allowance,
            allowance_expiry    = result.metadata.allowance.allowance_expiry,
            "prices v2 (num-points) allowance"
        );
        Ok(result)
    }

    /// Fetch bars within a date range at the given `resolution` (v2 endpoint).
    ///
    /// `GET /prices/{epic}/{resolution}/{startDate}/{endDate}`
    ///
    /// Dates are formatted as `YYYY/MM/DD HH:MM:SS` (IG v2 format) and placed
    /// directly in the URL path.
    ///
    /// # Errors
    ///
    /// Returns `Error::Api` when IG rejects the request.
    #[tracing::instrument(skip_all, fields(epic = %epic, resolution = %resolution))]
    pub async fn history_by_date_range_v2(
        &self,
        epic: &Epic,
        resolution: Resolution,
        from: NaiveDateTime,
        to: NaiveDateTime,
    ) -> Result<HistoricalPrices> {
        let from_s = time::format(from, ApiVersion::V2);
        let to_s = time::format(to, ApiVersion::V2);
        // The v2 date format includes slashes (YYYY/MM/DD) and a space before
        // the time component.  Both are valid when percent-encoded in a URL
        // path segment; reqwest will encode them as needed.
        let path = format!("prices/{epic}/{resolution}/{from_s}/{to_s}");
        let result: HistoricalPrices = self
            .client
            .transport
            .request(Method::GET, &path, Some(2), None::<&()>, &self.client.session)
            .await?;
        info!(
            remaining_allowance = result.metadata.allowance.remaining_allowance,
            allowance_expiry    = result.metadata.allowance.allowance_expiry,
            "prices v2 (date-range) allowance"
        );
        Ok(result)
    }

    /// Fetch bars within a date range at the given `resolution` (v1 endpoint).
    ///
    /// `GET /prices/{epic}/{resolution}?startdate=…&enddate=…`
    ///
    /// Note: the query-parameter names are lowercase (`startdate`, `enddate`),
    /// and dates are formatted as `YYYY:MM:DD-HH:MM:SS` (IG v1 format).
    ///
    /// # Errors
    ///
    /// Returns `Error::Api` when IG rejects the request.
    #[tracing::instrument(skip_all, fields(epic = %epic, resolution = %resolution))]
    pub async fn history_by_date_range_v1(
        &self,
        epic: &Epic,
        resolution: Resolution,
        from: NaiveDateTime,
        to: NaiveDateTime,
    ) -> Result<HistoricalPrices> {
        let from_s = time::format(from, ApiVersion::V1);
        let to_s = time::format(to, ApiVersion::V1);
        let params: &[(&str, String)] = &[
            ("startdate", from_s),
            ("enddate", to_s),
        ];
        let base = format!("prices/{epic}/{resolution}");
        let path = build_query_path(&base, params);
        let result: HistoricalPrices = self
            .client
            .transport
            .request(Method::GET, &path, Some(1), None::<&()>, &self.client.session)
            .await?;
        info!(
            remaining_allowance = result.metadata.allowance.remaining_allowance,
            allowance_expiry    = result.metadata.allowance.allowance_expiry,
            "prices v1 (date-range) allowance"
        );
        Ok(result)
    }
}

/// Append `?key=value&…` to `base` when `params` is non-empty.
fn build_query_path<K, V>(base: &str, params: &[(K, V)]) -> String
where
    K: AsRef<str>,
    V: AsRef<str>,
{
    if params.is_empty() {
        return base.to_owned();
    }
    let mut out = base.to_owned();
    out.push('?');
    for (i, (k, v)) in params.iter().enumerate() {
        if i > 0 {
            out.push('&');
        }
        out.push_str(k.as_ref());
        out.push('=');
        out.push_str(&percent_encode(v.as_ref()));
    }
    out
}

/// Percent-encode characters that are unsafe in URL query-string values.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            ' ' => out.push_str("%20"),
            '+' => out.push_str("%2B"),
            _ => out.push(c),
        }
    }
    out
}
