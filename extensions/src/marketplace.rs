use std::collections::HashMap;

use pot_o_core::TokenType;
use serde::{Deserialize, Serialize};

/// Asset identifier — either a ledger token or a bridged Solana mint.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketAsset {
    /// A native ledger token.
    Token(TokenType),
    /// A bridged Solana SPL token mint address.
    Spl(String),
    /// Native SOL (bridged).
    Sol,
}

impl std::fmt::Display for MarketAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketAsset::Token(t) => write!(f, "{t:?}"),
            MarketAsset::Sol => write!(f, "SOL"),
            MarketAsset::Spl(mint) => write!(f, "spl:{mint}"),
        }
    }
}

/// Buy or sell side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "buy"),
            OrderSide::Sell => write!(f, "sell"),
        }
    }
}

/// Order status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
}

/// A single order in the marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub maker: String,
    pub side: OrderSide,
    pub sell_asset: MarketAsset,
    pub buy_asset: MarketAsset,
    /// Total amount of sell_asset the maker wants to sell.
    pub amount: u64,
    /// Price in buy_asset per unit of sell_asset (fixed-price limit order).
    pub price: u64,
    pub filled: u64,
    pub status: OrderStatus,
    pub timestamp: u64,
}

impl Order {
    pub fn remaining(&self) -> u64 {
        self.amount.saturating_sub(self.filled)
    }
}

/// A filled trade record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub buy_order_id: String,
    pub sell_order_id: String,
    pub maker_buy: String,
    pub maker_sell: String,
    pub sell_asset: MarketAsset,
    pub buy_asset: MarketAsset,
    /// Amount of sell_asset filled.
    pub amount: u64,
    /// Price at execution.
    pub price: u64,
    /// Total cost in buy_asset.
    pub total: u64,
    pub timestamp: u64,
}

/// Order book for a specific asset pair (sell_asset → buy_asset).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub sell_asset: MarketAsset,
    pub buy_asset: MarketAsset,
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
}

/// The marketplace engine.
pub struct Marketplace {
    orders: HashMap<String, Order>,
    trades: Vec<Trade>,
    next_id: u64,
    /// Fee in basis points (e.g. 25 = 0.25%) charged to the filled side.
    pub fee_bps: u64,
    /// Protocol address that collects marketplace fees.
    pub protocol_fee_address: String,
}

impl Marketplace {
    pub fn new(fee_bps: u64, protocol_fee_address: String) -> Self {
        Self {
            orders: HashMap::new(),
            trades: Vec::new(),
            next_id: 1,
            fee_bps,
            protocol_fee_address,
        }
    }

    fn generate_id(&mut self) -> String {
        let id = self.next_id;
        self.next_id += 1;
        format!("order-{id}")
    }

    /// Place a limit order. Returns the order ID.
    pub fn place_order(
        &mut self,
        maker: &str,
        side: OrderSide,
        sell_asset: MarketAsset,
        buy_asset: MarketAsset,
        amount: u64,
        price: u64,
    ) -> String {
        let id = self.generate_id();
        let order = Order {
            id: id.clone(),
            maker: maker.to_string(),
            side,
            sell_asset,
            buy_asset,
            amount,
            price,
            filled: 0,
            status: OrderStatus::Open,
            timestamp: chrono::Utc::now().timestamp() as u64,
        };
        self.orders.insert(id.clone(), order);
        id
    }

    /// Cancel an open order. Returns `true` if found and cancelled.
    pub fn cancel_order(&mut self, order_id: &str) -> bool {
        if let Some(order) = self.orders.get_mut(order_id) {
            if order.status == OrderStatus::Open || order.status == OrderStatus::PartiallyFilled {
                order.status = OrderStatus::Cancelled;
                return true;
            }
        }
        false
    }

    /// Get a single order.
    pub fn get_order(&self, order_id: &str) -> Option<&Order> {
        self.orders.get(order_id)
    }

    /// Get the order book for a given asset pair.
    pub fn order_book(&self, sell_asset: &MarketAsset, buy_asset: &MarketAsset) -> OrderBook {
        let mut bids = Vec::new();
        let mut asks = Vec::new();
        for order in self.orders.values() {
            if order.status == OrderStatus::Cancelled || order.status == OrderStatus::Filled {
                continue;
            }
            if order.sell_asset != *sell_asset || order.buy_asset != *buy_asset {
                continue;
            }
            match order.side {
                OrderSide::Buy => bids.push(order.clone()),
                OrderSide::Sell => asks.push(order.clone()),
            }
        }
        // Sort bids descending by price, asks ascending by price
        bids.sort_by_key(|b| std::cmp::Reverse(b.price));
        asks.sort_by_key(|a| a.price);
        OrderBook {
            sell_asset: sell_asset.clone(),
            buy_asset: buy_asset.clone(),
            bids,
            asks,
        }
    }

    /// Get all orders for a maker.
    pub fn orders_for_maker(&self, maker: &str) -> Vec<&Order> {
        self.orders.values().filter(|o| o.maker == maker).collect()
    }

    /// Try to match a new sell order against existing buy orders (and vice versa).
    /// Returns the trades that were executed.
    /// This is a simple price-time priority matching engine.
    pub fn try_match(
        &mut self,
        maker: &str,
        side: OrderSide,
        sell_asset: &MarketAsset,
        buy_asset: &MarketAsset,
        mut amount: u64,
        price: u64,
    ) -> Vec<Trade> {
        let mut executed_trades = Vec::new();
        let now = chrono::Utc::now().timestamp() as u64;

        // Collect matching counter-orders from the same asset pair
        let mut counter_orders: Vec<String> = self
            .orders
            .iter()
            .filter(|(_, o)| {
                o.maker != maker
                    && (o.status == OrderStatus::Open || o.status == OrderStatus::PartiallyFilled)
                    && o.sell_asset == *sell_asset
                    && o.buy_asset == *buy_asset
                    && match (side, o.side) {
                        (OrderSide::Sell, OrderSide::Buy) => o.price >= price,
                        (OrderSide::Buy, OrderSide::Sell) => o.price <= price,
                        _ => false,
                    }
            })
            .map(|(id, _)| id.clone())
            .collect();

        // Sort by price priority, then timestamp (older first)
        match side {
            OrderSide::Sell => {
                // Counter is Buy orders: highest price first, then oldest
                counter_orders.sort_by(|a, b| {
                    let oa = &self.orders[a];
                    let ob = &self.orders[b];
                    ob.price
                        .cmp(&oa.price)
                        .then_with(|| oa.timestamp.cmp(&ob.timestamp))
                });
            }
            OrderSide::Buy => {
                // Counter is Sell orders: lowest price first, then oldest
                counter_orders.sort_by(|a, b| {
                    let oa = &self.orders[a];
                    let ob = &self.orders[b];
                    oa.price
                        .cmp(&ob.price)
                        .then_with(|| oa.timestamp.cmp(&ob.timestamp))
                });
            }
        }

        for counter_id in counter_orders {
            if amount == 0 {
                break;
            }
            let filled_amount;
            let exec_price;
            {
                let counter = self.orders.get(&counter_id).unwrap();
                let available = counter.remaining();
                if available == 0 {
                    continue;
                }
                filled_amount = amount.min(available);
                exec_price = counter.price;
            }

            // Deduct from counter order
            if let Some(counter) = self.orders.get_mut(&counter_id) {
                counter.filled = counter.filled.saturating_add(filled_amount);
                if counter.filled >= counter.amount {
                    counter.status = OrderStatus::Filled;
                } else {
                    counter.status = OrderStatus::PartiallyFilled;
                }
            }

            let total_cost = filled_amount.saturating_mul(exec_price);

            let (buy_order_id, sell_order_id, maker_buy, maker_sell) = match side {
                OrderSide::Sell => {
                    let co = &self.orders[&counter_id];
                    (
                        counter_id.clone(),
                        String::new(),
                        co.maker.clone(),
                        maker.to_string(),
                    )
                }
                OrderSide::Buy => {
                    let co = &self.orders[&counter_id];
                    (
                        String::new(),
                        counter_id.clone(),
                        maker.to_string(),
                        co.maker.clone(),
                    )
                }
            };

            let trade = Trade {
                buy_order_id,
                sell_order_id,
                maker_buy,
                maker_sell,
                sell_asset: sell_asset.clone(),
                buy_asset: buy_asset.clone(),
                amount: filled_amount,
                price: exec_price,
                total: total_cost,
                timestamp: now,
            };
            executed_trades.push(trade);
            amount = amount.saturating_sub(filled_amount);
        }

        self.trades.extend(executed_trades.clone());
        executed_trades
    }

    /// Place an order and immediately attempt to match it.
    /// Returns the order ID and any trades that were executed.
    pub fn place_and_match(
        &mut self,
        maker: &str,
        side: OrderSide,
        sell_asset: MarketAsset,
        buy_asset: MarketAsset,
        amount: u64,
        price: u64,
    ) -> (String, Vec<Trade>) {
        let trades = self.try_match(maker, side, &sell_asset, &buy_asset, amount, price);
        let filled_in_match: u64 = trades.iter().map(|t| t.amount).sum();
        let remaining = amount.saturating_sub(filled_in_match);

        let order_id = if remaining > 0 {
            let id = self.generate_id();
            let order = Order {
                id: id.clone(),
                maker: maker.to_string(),
                side,
                sell_asset,
                buy_asset,
                amount: remaining,
                price,
                filled: 0,
                status: OrderStatus::Open,
                timestamp: chrono::Utc::now().timestamp() as u64,
            };
            self.orders.insert(id.clone(), order);
            id
        } else {
            String::new()
        };

        (order_id, trades)
    }

    /// Get all trades.
    pub fn trades(&self) -> &[Trade] {
        &self.trades
    }

    /// Get all open orders.
    pub fn open_orders(&self) -> Vec<&Order> {
        self.orders
            .values()
            .filter(|o| o.status == OrderStatus::Open || o.status == OrderStatus::PartiallyFilled)
            .collect()
    }
}

pub fn parse_market_asset(s: &str) -> Result<MarketAsset, String> {
    let lower = s.to_lowercase();
    if lower == "sol" {
        return Ok(MarketAsset::Sol);
    }
    if let Some(mint) = lower.strip_prefix("spl:") {
        return Ok(MarketAsset::Spl(mint.to_string()));
    }
    match lower.as_str() {
        "tribechain" | "native" => Ok(MarketAsset::Token(TokenType::TribeChain)),
        "pttc" => Ok(MarketAsset::Token(TokenType::PTtC)),
        "nmtc" => Ok(MarketAsset::Token(TokenType::NMTC)),
        "stomp" => Ok(MarketAsset::Token(TokenType::STOMP)),
        "aum" => Ok(MarketAsset::Token(TokenType::AUM)),
        "ai3" => Ok(MarketAsset::Token(TokenType::AI3)),
        _ => Err(format!("Unknown asset: {s}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_sell_asset() -> MarketAsset {
        MarketAsset::Token(TokenType::TribeChain)
    }

    fn test_buy_asset() -> MarketAsset {
        MarketAsset::Token(TokenType::PTtC)
    }

    #[test]
    fn test_place_and_get_order() {
        let mut m = Marketplace::new(25, "fee_addr".into());
        let id = m.place_order(
            "alice",
            OrderSide::Sell,
            test_sell_asset(),
            test_buy_asset(),
            100,
            10,
        );
        let order = m.get_order(&id).unwrap();
        assert_eq!(order.maker, "alice");
        assert_eq!(order.amount, 100);
        assert_eq!(order.remaining(), 100);
    }

    #[test]
    fn test_cancel_order() {
        let mut m = Marketplace::new(25, "fee_addr".into());
        let id = m.place_order(
            "alice",
            OrderSide::Sell,
            test_sell_asset(),
            test_buy_asset(),
            100,
            10,
        );
        assert!(m.cancel_order(&id));
        assert_eq!(m.get_order(&id).unwrap().status, OrderStatus::Cancelled);
        // Second cancel should fail
        assert!(!m.cancel_order(&id));
    }

    #[test]
    fn test_order_book() {
        let mut m = Marketplace::new(25, "fee_addr".into());
        m.place_order(
            "alice",
            OrderSide::Sell,
            test_sell_asset(),
            test_buy_asset(),
            100,
            10,
        );
        m.place_order(
            "bob",
            OrderSide::Buy,
            test_sell_asset(),
            test_buy_asset(),
            50,
            9,
        );
        let ob = m.order_book(&test_sell_asset(), &test_buy_asset());
        assert_eq!(ob.asks.len(), 1);
        assert_eq!(ob.bids.len(), 1);
    }

    #[test]
    fn test_orders_for_maker() {
        let mut m = Marketplace::new(25, "".into());
        m.place_order(
            "alice",
            OrderSide::Sell,
            test_sell_asset(),
            test_buy_asset(),
            100,
            10,
        );
        m.place_order(
            "alice",
            OrderSide::Buy,
            test_sell_asset(),
            test_buy_asset(),
            50,
            9,
        );
        m.place_order(
            "bob",
            OrderSide::Sell,
            test_sell_asset(),
            test_buy_asset(),
            30,
            8,
        );
        let alice_orders = m.orders_for_maker("alice");
        assert_eq!(alice_orders.len(), 2);
        let bob_orders = m.orders_for_maker("bob");
        assert_eq!(bob_orders.len(), 1);
    }

    #[test]
    fn test_matching_buy_sell() {
        let mut m = Marketplace::new(25, "".into());
        // Pair: TribeChain/PTtC (sell_asset=TribeChain=base, buy_asset=PTtC=quote)
        // alice buys 50 TribeChain at price 10 (pays 10 PTtC per TribeChain)
        m.place_order(
            "alice",
            OrderSide::Buy,
            test_sell_asset(),
            test_buy_asset(),
            50,
            10,
        );
        // bob sells 30 TribeChain at price 10 (receives 10 PTtC per TribeChain)
        let trades = m.try_match(
            "bob",
            OrderSide::Sell,
            &test_sell_asset(),
            &test_buy_asset(),
            30,
            10,
        );
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].amount, 30);
        // alice's order should be partially filled (bought 30 of her 50)
        assert_eq!(m.orders_for_maker("alice").len(), 1);
        let alice_order = m.orders_for_maker("alice")[0];
        assert_eq!(alice_order.filled, 30);
        assert_eq!(alice_order.status, OrderStatus::PartiallyFilled);
    }

    #[test]
    fn test_matching_no_match_when_price_wrong() {
        let mut m = Marketplace::new(25, "".into());
        m.place_order(
            "alice",
            OrderSide::Buy,
            test_sell_asset(),
            test_buy_asset(),
            50,
            10,
        );
        // bob wants to sell at 11, but highest buy is 10 — no match
        let trades = m.try_match(
            "bob",
            OrderSide::Sell,
            &test_sell_asset(),
            &test_buy_asset(),
            30,
            11,
        );
        assert_eq!(trades.len(), 0);
    }

    #[test]
    fn test_place_and_match_full_fill() {
        let mut m = Marketplace::new(25, "".into());
        m.place_order(
            "alice",
            OrderSide::Buy,
            test_sell_asset(),
            test_buy_asset(),
            50,
            10,
        );
        let (order_id, trades) = m.place_and_match(
            "bob",
            OrderSide::Sell,
            test_sell_asset(),
            test_buy_asset(),
            50,
            10,
        );
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].amount, 50);
        assert!(order_id.is_empty());
    }

    #[test]
    fn test_place_and_match_partial_fill() {
        let mut m = Marketplace::new(25, "".into());
        m.place_order(
            "alice",
            OrderSide::Buy,
            test_sell_asset(),
            test_buy_asset(),
            50,
            10,
        );
        let (order_id, trades) = m.place_and_match(
            "bob",
            OrderSide::Sell,
            test_sell_asset(),
            test_buy_asset(),
            100,
            10,
        );
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].amount, 50);
        let order = m.get_order(&order_id).unwrap();
        assert_eq!(order.amount, 50);
        assert_eq!(order.status, OrderStatus::Open);
    }

    #[test]
    fn test_parse_market_asset() {
        assert!(matches!(
            parse_market_asset("tribechain").unwrap(),
            MarketAsset::Token(TokenType::TribeChain)
        ));
        assert!(matches!(
            parse_market_asset("SOL").unwrap(),
            MarketAsset::Sol
        ));
        assert!(
            matches!(parse_market_asset("spl:abc123").unwrap(), MarketAsset::Spl(m) if m == "abc123")
        );
        assert!(parse_market_asset("unknown").is_err());
    }

    #[test]
    fn test_parse_market_asset_case_insensitive() {
        assert!(matches!(
            parse_market_asset("TribeChain").unwrap(),
            MarketAsset::Token(TokenType::TribeChain)
        ));
        assert!(matches!(
            parse_market_asset("sol").unwrap(),
            MarketAsset::Sol
        ));
        assert!(
            matches!(parse_market_asset("SPL:XYZ").unwrap(), MarketAsset::Spl(m) if m == "xyz")
        );
    }
}
