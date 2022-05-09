use std::{future::Future, iter::Sum};

use anyhow::Result;
use futures_util::future::try_join_all;

#[derive(Clone, Copy, Debug)]
struct Price {
    value: f64,
}

impl Price {
    fn new(value: f64) -> Self {
        Self { value }
    }
}

impl std::ops::Add for Price {
    type Output = Price;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            value: self.value + rhs.value,
        }
    }
}

impl Sum<Self> for Price {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Price::new(0.), |a, b| Price::new(a.value + b.value))
    }
}

struct BillingAmount {
    value: Price,
}

impl BillingAmount {
    fn sum_prices(prices: impl Iterator<Item = Price>) -> BillingAmount {
        let sum = prices.sum();
        Self { value: sum }
    }
}

struct PricedOrderLine {
    order_line_id: OrderLineId,
    line_price: Price,
}

struct PricedOrder {
    order_id: OrderId,
    amount_to_bill: BillingAmount,
    lines: Vec<PricedOrderLine>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OrderLineId {
    value: String,
}

impl OrderLineId {
    fn new(id: impl Into<String>) -> Self {
        Self { value: id.into() }
    }
}
struct OrderId {
    value: String,
}

impl OrderId {
    fn new(id: String) -> Self {
        Self { value: id }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct ProductCode {
    value: String,
}

impl ProductCode {
    fn new(code: String) -> Self {
        Self { value: code }
    }
}

pub struct UnvalidatedOrderLine {
    order_line_id: String,
    product_code: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedOrderLine {
    order_line_id: OrderLineId,
    product_code: ProductCode,
}
pub struct UnvalidatedOrder {
    id: String,
    lines: Vec<UnvalidatedOrderLine>,
}
struct ValidatedOrder {
    id: OrderId,
    lines: Vec<ValidatedOrderLine>,
}

struct Product {
    id: String,
}
struct Address {
    street: String,
}

struct Letter {
    content: String,
}

enum Event {
    ProductSent,
}

async fn to_valid_order_line<Fut1: Future<Output = Result<()>>>(
    check_product_code_exists: fn(ProductCode) -> Fut1,
    unvalidated_line: UnvalidatedOrderLine,
) -> Result<ValidatedOrderLine> {
    let product_code = ProductCode::new(unvalidated_line.product_code);
    let order_line_id = OrderLineId::new(unvalidated_line.order_line_id);

    check_product_code_exists(product_code.clone()).await?;

    Ok(ValidatedOrderLine {
        product_code,
        order_line_id,
    })
}
#[tokio::test]
async fn converts_to_order_line() {
    async fn check_product_code_exists(_code: ProductCode) -> Result<()> {
        Ok(())
    }
    let code = String::from("fake-code");
    let line_id = String::from("some-id");

    let unvalidated_line = UnvalidatedOrderLine {
        product_code: code.clone(),
        order_line_id: line_id.clone(),
    };
    let valid_line = to_valid_order_line(check_product_code_exists, unvalidated_line)
        .await
        .unwrap();
    let valid_order_line = ValidatedOrderLine {
        product_code: ProductCode::new(code),
        order_line_id: OrderLineId::new(line_id),
    };
    assert_eq!(valid_line, valid_order_line)
}

async fn validate_order<Fut1: Future<Output = Result<()>>, Fut2: Future<Output = Result<()>>>(
    check_product_exists: fn(ProductCode) -> Fut1,
    check_address_exists: fn(Address) -> Fut2,
    unvalidated_order: UnvalidatedOrder,
) -> Result<ValidatedOrder> {
    let order_id = OrderId::new(unvalidated_order.id);

    let lines = try_join_all(
        unvalidated_order
            .lines
            .into_iter()
            .map(|unvalidated_line| to_valid_order_line(check_product_exists, unvalidated_line)),
    )
    .await?;

    let order = ValidatedOrder {
        id: order_id,
        lines,
    };
    Ok(order)
}

async fn to_priced_order_line<Fut1: Future<Output = Result<Price>>>(
    get_product_price: fn(ProductCode) -> Fut1,
    validated_order_line: ValidatedOrderLine,
) -> Result<PricedOrderLine> {
    let line_price = get_product_price(validated_order_line.product_code).await?;

    let priced_order_line = PricedOrderLine {
        order_line_id: validated_order_line.order_line_id,
        line_price,
    };
    Ok(priced_order_line)
}

async fn price_order<Fut1: Future<Output = Result<Price>>>(
    get_product_price: fn(ProductCode) -> Fut1,
    validated_order: ValidatedOrder,
) -> Result<PricedOrder> {
    let lines = try_join_all(
        validated_order
            .lines
            .into_iter()
            .map(|validated_line| to_priced_order_line(get_product_price, validated_line)),
    )
    .await?;

    let amount_to_bill = BillingAmount::sum_prices(lines.iter().map(|p| p.line_price.clone()));

    let priced_order = PricedOrder {
        order_id: validated_order.id,
        amount_to_bill,
        lines,
    };
    Ok(priced_order)
}

// type CheckProductExists<Fut1: Future<Output=Result<()>>> = fn(ProductCode) -> Fut1;

async fn place_order<
    Fut1: Future<Output = Result<()>>,
    Fut2: Future<Output = Result<()>>,
    Fut3: Future<Output = Result<Price>>,
    Fut4: Future<Output = Result<Letter>>,
>(
    check_product_exists: fn(ProductCode) -> Fut1,
    check_address_exists: fn(Address) -> Fut2,
    get_product_price: fn(ProductCode) -> Fut3,
    create_order_acknowledgment_letter: fn(product: Product) -> Fut4,
    unvalidated_order: UnvalidatedOrder,
) -> Result<Vec<Event>> {
    let validated_order = validate_order(
        check_product_exists,
        check_address_exists,
        unvalidated_order,
    )
    .await?;

    let priced_order = price_order(get_product_price, validated_order).await?;
    todo!();
}
