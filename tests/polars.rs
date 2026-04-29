//! Round-trip tests for the `polars` cargo feature.
//!
//! Each test constructs a `Vec<T>` (or other applicable type), calls
//! `.to_dataframe()`, and asserts the expected row count and column dtypes.
//!
//! All tests are gated behind `#[cfg(feature = "polars")]` so that the default
//! test run (no feature) remains at 153 tests.

#![cfg(feature = "polars")]

use chrono::NaiveDateTime;
use trading_ig::{
    accounts::models::{Account, AccountBalance, AccountType},
    client_sentiment::models::Sentiment,
    dataframe::IntoDataFrame,
    dealing::positions::models::PositionV2,
    dealing::working_orders::models::{WorkingOrderDataV2, WorkingOrderV2},
    history::models::{Activity, ActivityChannel, ActivityStatus, ActivityType, Transaction},
    markets::models::MarketSummary,
    models::common::{
        Currency, DealId, DealReference, Direction, Epic, InstrumentType, MarketSnapshot,
        MarketStatus, OrderType, TimeInForce,
    },
    prices::models::{
        HistoricalPrices, PageData, PriceAllowance, PriceCandle, PricePoint, PricesMetadata,
    },
};

// ── helpers ─────────────────────────────────────────────────────────────────

fn dt(s: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").unwrap()
}

fn make_market_snapshot() -> MarketSnapshot {
    MarketSnapshot {
        epic: Some(Epic::new("CS.D.GBPUSD.TODAY.IP")),
        instrument_name: Some("GBP/USD".into()),
        expiry: Some("DFB".into()),
        instrument_type: Some("CURRENCIES".into()),
        lot_size: None,
        bid: Some(1.2345),
        offer: Some(1.2355),
        high: Some(1.24),
        low: Some(1.23),
        percentage_change: Some(0.1),
        net_change: Some(0.001),
        update_time: Some("09:00:00".into()),
        update_time_utc: Some("09:00:00".into()),
        market_status: MarketStatus::Tradeable,
        delay_time: None,
        binary_odds: None,
        decimal_places_factor: None,
        scaling_factor: None,
        controlled_risk_extra_spread: None,
    }
}

// ── Vec<Account> ─────────────────────────────────────────────────────────────

#[test]
fn accounts_to_dataframe_height_and_schema() {
    let accounts = vec![
        Account {
            account_id: "ABC123".into(),
            account_alias: Some("Main".into()),
            account_type: AccountType::Cfd,
            account_name: "Demo CFD".into(),
            can_transfer_to_ma: Some(true),
            can_transfer_from_ma: Some(false),
            default_account: true,
            preferred: true,
            balance: AccountBalance {
                balance: Some(10_000.0),
                deposit: Some(500.0),
                profit_loss: Some(50.0),
                available_cash: Some(9_500.0),
            },
        },
        Account {
            account_id: "DEF456".into(),
            account_alias: None,
            account_type: AccountType::Spreadbet,
            account_name: "Demo SB".into(),
            can_transfer_to_ma: None,
            can_transfer_from_ma: None,
            default_account: false,
            preferred: false,
            balance: AccountBalance {
                balance: None,
                deposit: None,
                profit_loss: None,
                available_cash: None,
            },
        },
    ];

    let df = accounts.to_dataframe().expect("conversion should succeed");

    assert_eq!(df.height(), 2, "should have 2 rows");
    assert_eq!(df.width(), 12, "should have 12 columns");

    // Check specific columns exist and have the right dtype family.
    let account_id_col = df.column("account_id").expect("account_id column");
    assert!(
        matches!(account_id_col.dtype(), polars::prelude::DataType::String),
        "account_id should be Utf8/String"
    );

    let balance_col = df.column("balance").expect("balance column");
    assert!(
        matches!(balance_col.dtype(), polars::prelude::DataType::Float64),
        "balance should be Float64"
    );

    let default_account_col = df
        .column("default_account")
        .expect("default_account column");
    assert!(
        matches!(
            default_account_col.dtype(),
            polars::prelude::DataType::Boolean
        ),
        "default_account should be Boolean"
    );

    // Second row's balance should be null.
    let balance_vals = balance_col.f64().unwrap();
    assert!(
        balance_vals.get(0).is_some(),
        "first row balance should be Some"
    );
    assert!(
        balance_vals.get(1).is_none(),
        "second row balance should be null"
    );
}

#[test]
fn accounts_empty_to_dataframe() {
    let empty: Vec<Account> = vec![];
    let df = empty
        .to_dataframe()
        .expect("empty conversion should succeed");
    assert_eq!(df.height(), 0);
    assert_eq!(df.width(), 12);
}

// ── Vec<PositionV2> ──────────────────────────────────────────────────────────

#[test]
fn positions_v2_to_dataframe_height_and_schema() {
    let positions = vec![
        PositionV2 {
            deal_id: DealId::new("DIAAA001"),
            deal_reference: DealReference::new("ref-001"),
            direction: Direction::Buy,
            size: 1.0,
            level: 1.2345,
            limit_level: Some(1.25),
            stop_level: None,
            controlled_risk: false,
            contract_size: 1.0,
            created_date: "2024/01/01 09:30:00".into(),
            created_date_utc: Some(dt("2024-01-01T09:30:00")),
            trailing_step: None,
            trailing_stop_distance: None,
            market: make_market_snapshot(),
        },
        PositionV2 {
            deal_id: DealId::new("DIAAA002"),
            deal_reference: DealReference::new("ref-002"),
            direction: Direction::Sell,
            size: 2.0,
            level: 1.23,
            limit_level: None,
            stop_level: Some(1.21),
            controlled_risk: true,
            contract_size: 1.0,
            created_date: "2024/01/02 10:00:00".into(),
            created_date_utc: None,
            trailing_step: Some(5.0),
            trailing_stop_distance: Some(20.0),
            market: make_market_snapshot(),
        },
    ];

    let df = positions.to_dataframe().expect("conversion should succeed");

    assert_eq!(df.height(), 2);
    assert_eq!(df.width(), 17);

    let direction_col = df.column("direction").expect("direction column");
    assert!(
        matches!(direction_col.dtype(), polars::prelude::DataType::String),
        "direction should be String"
    );

    let size_col = df.column("size").expect("size column");
    assert!(
        matches!(size_col.dtype(), polars::prelude::DataType::Float64),
        "size should be Float64"
    );

    // created_date_utc should be Datetime type.
    let cdu_col = df
        .column("created_date_utc")
        .expect("created_date_utc column");
    assert!(
        matches!(cdu_col.dtype(), polars::prelude::DataType::Datetime(_, _)),
        "created_date_utc should be Datetime, got {:?}",
        cdu_col.dtype()
    );

    // Second row created_date_utc should be null.
    let cdu_vals = cdu_col.datetime().unwrap();
    assert!(
        cdu_vals.get(0).is_some(),
        "first row should have created_date_utc"
    );
    assert!(
        cdu_vals.get(1).is_none(),
        "second row created_date_utc should be null"
    );
}

#[test]
fn positions_v2_empty_to_dataframe() {
    let empty: Vec<PositionV2> = vec![];
    let df = empty
        .to_dataframe()
        .expect("empty conversion should succeed");
    assert_eq!(df.height(), 0);
    assert_eq!(df.width(), 17);
}

// ── Vec<WorkingOrderV2> ──────────────────────────────────────────────────────

fn make_working_order_v2(deal_id: &str) -> WorkingOrderV2 {
    WorkingOrderV2 {
        order_data: WorkingOrderDataV2 {
            created_date: Some("2024/01/01 09:00:00".into()),
            created_date_utc: Some("2024-01-01T09:00:00".into()),
            currency_code: Some(Currency::new("EUR")),
            deal_id: DealId::new(deal_id),
            direction: Direction::Buy,
            dma: Some(false),
            epic: Epic::new("CS.D.GBPUSD.TODAY.IP"),
            good_till_date: None,
            good_till_date_iso: None,
            guaranteed_stop: false,
            limit_distance: Some(10.0),
            limit_level: None,
            order_level: 1.2345,
            order_size: 1.0,
            order_type: OrderType::Limit,
            stop_distance: Some(20.0),
            stop_level: None,
            time_in_force: TimeInForce::GoodTillCancelled,
        },
        market: make_market_snapshot(),
    }
}

#[test]
fn working_orders_v2_to_dataframe_height_and_schema() {
    let orders = vec![
        make_working_order_v2("DIAAA001"),
        make_working_order_v2("DIAAA002"),
        make_working_order_v2("DIAAA003"),
    ];

    let df = orders.to_dataframe().expect("conversion should succeed");

    assert_eq!(df.height(), 3);
    assert_eq!(df.width(), 18);

    let deal_id_col = df.column("deal_id").expect("deal_id column");
    assert!(
        matches!(deal_id_col.dtype(), polars::prelude::DataType::String),
        "deal_id should be String"
    );

    let order_level_col = df.column("order_level").expect("order_level column");
    assert!(
        matches!(order_level_col.dtype(), polars::prelude::DataType::Float64),
        "order_level should be Float64"
    );

    let guaranteed_stop_col = df
        .column("guaranteed_stop")
        .expect("guaranteed_stop column");
    assert!(
        matches!(
            guaranteed_stop_col.dtype(),
            polars::prelude::DataType::Boolean
        ),
        "guaranteed_stop should be Boolean"
    );
}

#[test]
fn working_orders_v2_empty_to_dataframe() {
    let empty: Vec<WorkingOrderV2> = vec![];
    let df = empty
        .to_dataframe()
        .expect("empty conversion should succeed");
    assert_eq!(df.height(), 0);
    assert_eq!(df.width(), 18);
}

// ── HistoricalPrices ──────────────────────────────────────────────────────────

fn make_price_point(ts: &str) -> PricePoint {
    PricePoint {
        snapshot_time: "2024/01/01 09:00:00".into(),
        snapshot_time_utc: dt(ts),
        open_price: PriceCandle {
            bid: Some(1.23),
            ask: Some(1.24),
            last_traded: None,
        },
        close_price: PriceCandle {
            bid: Some(1.235),
            ask: Some(1.245),
            last_traded: None,
        },
        high_price: PriceCandle {
            bid: Some(1.25),
            ask: Some(1.26),
            last_traded: None,
        },
        low_price: PriceCandle {
            bid: Some(1.22),
            ask: Some(1.23),
            last_traded: None,
        },
        last_traded_volume: Some(1000),
    }
}

fn make_historical_prices(n: usize) -> HistoricalPrices {
    HistoricalPrices {
        instrument_type: "CURRENCIES".into(),
        prices: (0..n)
            .map(|i| make_price_point(&format!("2024-01-01T{:02}:00:00", i % 24)))
            .collect(),
        metadata: PricesMetadata {
            page_data: PageData {
                page_number: 1,
                page_size: 20,
                total_pages: 1,
            },
            allowance: PriceAllowance {
                remaining_allowance: 9999,
                total_allowance: 10000,
                allowance_expiry: 3600,
            },
        },
    }
}

#[test]
fn historical_prices_to_dataframe_height_and_schema() {
    let prices = make_historical_prices(5);
    let df = prices.to_dataframe().expect("conversion should succeed");

    assert_eq!(df.height(), 5);
    assert_eq!(df.width(), 11);

    let snapshot_time_col = df.column("snapshot_time").expect("snapshot_time column");
    assert!(
        matches!(snapshot_time_col.dtype(), polars::prelude::DataType::String),
        "snapshot_time should be String"
    );

    let snapshot_time_utc_col = df
        .column("snapshot_time_utc")
        .expect("snapshot_time_utc column");
    assert!(
        matches!(
            snapshot_time_utc_col.dtype(),
            polars::prelude::DataType::Datetime(_, _)
        ),
        "snapshot_time_utc should be Datetime, got {:?}",
        snapshot_time_utc_col.dtype()
    );

    let open_bid_col = df.column("open_bid").expect("open_bid column");
    assert!(
        matches!(open_bid_col.dtype(), polars::prelude::DataType::Float64),
        "open_bid should be Float64"
    );

    let volume_col = df
        .column("last_traded_volume")
        .expect("last_traded_volume column");
    assert!(
        matches!(volume_col.dtype(), polars::prelude::DataType::UInt64),
        "last_traded_volume should be UInt64"
    );
}

#[test]
fn historical_prices_empty_to_dataframe() {
    let prices = make_historical_prices(0);
    let df = prices
        .to_dataframe()
        .expect("empty conversion should succeed");
    assert_eq!(df.height(), 0);
    assert_eq!(df.width(), 11);
}

// ── Vec<Activity> ────────────────────────────────────────────────────────────

fn make_activity(deal_id: &str) -> Activity {
    Activity {
        date: dt("2024-01-01T09:30:00"),
        epic: Epic::new("CS.D.GBPUSD.TODAY.IP"),
        period: "DFB".into(),
        deal_id: DealId::new(deal_id),
        channel: ActivityChannel::PublicWebApi,
        activity_type: ActivityType::Position,
        status: ActivityStatus::Accepted,
        description: "Position opened".into(),
        details: None,
    }
}

#[test]
fn activities_to_dataframe_height_and_schema() {
    let activities = vec![make_activity("DIAAA001"), make_activity("DIAAA002")];

    let df = activities
        .to_dataframe()
        .expect("conversion should succeed");

    assert_eq!(df.height(), 2);
    assert_eq!(df.width(), 8);

    let date_col = df.column("date").expect("date column");
    assert!(
        matches!(date_col.dtype(), polars::prelude::DataType::Datetime(_, _)),
        "date should be Datetime, got {:?}",
        date_col.dtype()
    );

    let epic_col = df.column("epic").expect("epic column");
    assert!(
        matches!(epic_col.dtype(), polars::prelude::DataType::String),
        "epic should be String"
    );

    let status_col = df.column("status").expect("status column");
    assert!(
        matches!(status_col.dtype(), polars::prelude::DataType::String),
        "status should be String"
    );

    // Check value
    let statuses = status_col.str().unwrap();
    assert_eq!(statuses.get(0), Some("ACCEPTED"));
}

#[test]
fn activities_empty_to_dataframe() {
    let empty: Vec<Activity> = vec![];
    let df = empty
        .to_dataframe()
        .expect("empty conversion should succeed");
    assert_eq!(df.height(), 0);
    assert_eq!(df.width(), 8);
}

// ── Vec<Transaction> ─────────────────────────────────────────────────────────

fn make_transaction(reference: &str) -> Transaction {
    Transaction {
        date: "2024/01/01".into(),
        date_utc: dt("2024-01-01T09:30:00"),
        instrument_name: "GBP/USD".into(),
        period: "DFB".into(),
        profit_and_loss: "EUR1234.56".into(),
        transaction_type: "TRADE".into(),
        reference: reference.into(),
        open_level: "1.2345".into(),
        close_level: "1.2400".into(),
        size: "1".into(),
        currency: Currency::new("EUR"),
        cash_transaction: false,
    }
}

#[test]
fn transactions_to_dataframe_height_and_schema() {
    let transactions = vec![make_transaction("REF001"), make_transaction("REF002")];

    let df = transactions
        .to_dataframe()
        .expect("conversion should succeed");

    assert_eq!(df.height(), 2);
    assert_eq!(df.width(), 16);

    // Raw string columns.
    let profit_and_loss_col = df
        .column("profit_and_loss")
        .expect("profit_and_loss column");
    assert!(
        matches!(
            profit_and_loss_col.dtype(),
            polars::prelude::DataType::String
        ),
        "profit_and_loss should be String"
    );

    // Parsed value column.
    let profit_and_loss_value_col = df
        .column("profit_and_loss_value")
        .expect("profit_and_loss_value column");
    assert!(
        matches!(
            profit_and_loss_value_col.dtype(),
            polars::prelude::DataType::Float64
        ),
        "profit_and_loss_value should be Float64"
    );

    let values = profit_and_loss_value_col.f64().unwrap();
    let v = values.get(0).expect("should have a parsed value");
    assert!(
        (v - 1234.56).abs() < 1e-6,
        "parsed P&L should be 1234.56, got {v}"
    );

    // date_utc should be Datetime.
    let date_utc_col = df.column("date_utc").expect("date_utc column");
    assert!(
        matches!(
            date_utc_col.dtype(),
            polars::prelude::DataType::Datetime(_, _)
        ),
        "date_utc should be Datetime, got {:?}",
        date_utc_col.dtype()
    );

    // Boolean column.
    let cash_col = df
        .column("cash_transaction")
        .expect("cash_transaction column");
    assert!(
        matches!(cash_col.dtype(), polars::prelude::DataType::Boolean),
        "cash_transaction should be Boolean"
    );
}

#[test]
fn transactions_with_unparseable_numeric_gives_null() {
    let mut tx = make_transaction("REF999");
    tx.profit_and_loss = "N/A".into(); // cannot be parsed as f64
    let transactions = vec![tx];

    let df = transactions
        .to_dataframe()
        .expect("conversion should succeed");
    let values = df.column("profit_and_loss_value").unwrap().f64().unwrap();
    assert!(values.get(0).is_none(), "unparseable P&L should yield null");
}

#[test]
fn transactions_empty_to_dataframe() {
    let empty: Vec<Transaction> = vec![];
    let df = empty
        .to_dataframe()
        .expect("empty conversion should succeed");
    assert_eq!(df.height(), 0);
    assert_eq!(df.width(), 16);
}

// ── Vec<MarketSummary> ───────────────────────────────────────────────────────

fn make_market_summary(epic: &str) -> MarketSummary {
    MarketSummary {
        epic: Epic::new(epic),
        instrument_name: "GBP/USD".into(),
        instrument_type: InstrumentType::Currencies,
        expiry: "DFB".into(),
        bid: Some(1.2345),
        offer: Some(1.2355),
        market_status: MarketStatus::Tradeable,
        streaming_prices_available: true,
        high: Some(1.24),
        low: Some(1.23),
        net_change: Some(0.001),
        percentage_change: Some(0.08),
        update_time: Some("09:00:00".into()),
        update_time_utc: Some("09:00:00".into()),
        delay_time: Some(0),
        scaling_factor: Some(10000),
    }
}

#[test]
fn market_summaries_to_dataframe_height_and_schema() {
    let markets = vec![
        make_market_summary("CS.D.GBPUSD.TODAY.IP"),
        make_market_summary("CS.D.EURUSD.TODAY.IP"),
        make_market_summary("CS.D.USDJPY.TODAY.IP"),
    ];

    let df = markets.to_dataframe().expect("conversion should succeed");

    assert_eq!(df.height(), 3);
    assert_eq!(df.width(), 14);

    let epic_col = df.column("epic").expect("epic column");
    assert!(
        matches!(epic_col.dtype(), polars::prelude::DataType::String),
        "epic should be String"
    );

    let bid_col = df.column("bid").expect("bid column");
    assert!(
        matches!(bid_col.dtype(), polars::prelude::DataType::Float64),
        "bid should be Float64"
    );

    let streaming_col = df
        .column("streaming_prices_available")
        .expect("streaming_prices_available column");
    assert!(
        matches!(streaming_col.dtype(), polars::prelude::DataType::Boolean),
        "streaming_prices_available should be Boolean"
    );

    let epics = epic_col.str().unwrap();
    assert_eq!(epics.get(0), Some("CS.D.GBPUSD.TODAY.IP"));
}

#[test]
fn market_summaries_empty_to_dataframe() {
    let empty: Vec<MarketSummary> = vec![];
    let df = empty
        .to_dataframe()
        .expect("empty conversion should succeed");
    assert_eq!(df.height(), 0);
    assert_eq!(df.width(), 14);
}

// ── Vec<Sentiment> ───────────────────────────────────────────────────────────

#[test]
fn sentiment_to_dataframe_height_and_schema() {
    let sentiments = vec![
        Sentiment {
            market_id: "CC.D.LCO.UNC.IP".into(),
            long_position_percentage: 60.0,
            short_position_percentage: 40.0,
        },
        Sentiment {
            market_id: "CS.D.GBPUSD.TODAY.IP".into(),
            long_position_percentage: 45.5,
            short_position_percentage: 54.5,
        },
    ];

    let df = sentiments
        .to_dataframe()
        .expect("conversion should succeed");

    assert_eq!(df.height(), 2);
    assert_eq!(df.width(), 3);

    let market_id_col = df.column("market_id").expect("market_id column");
    assert!(
        matches!(market_id_col.dtype(), polars::prelude::DataType::String),
        "market_id should be String"
    );

    let long_col = df
        .column("long_position_percentage")
        .expect("long_position_percentage column");
    assert!(
        matches!(long_col.dtype(), polars::prelude::DataType::Float64),
        "long_position_percentage should be Float64"
    );

    let vals = long_col.f64().unwrap();
    assert!((vals.get(0).unwrap() - 60.0).abs() < 1e-9);
    assert!((vals.get(1).unwrap() - 45.5).abs() < 1e-9);
}

#[test]
fn sentiment_empty_to_dataframe() {
    let empty: Vec<Sentiment> = vec![];
    let df = empty
        .to_dataframe()
        .expect("empty conversion should succeed");
    assert_eq!(df.height(), 0);
    assert_eq!(df.width(), 3);
}
