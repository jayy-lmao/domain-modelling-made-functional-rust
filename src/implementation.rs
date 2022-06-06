use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_fn_traits::AsyncFn1;
use async_fn_traits::AsyncFnOnce1;
use futures_util::future::try_join_all;

use crate::compound_types::*;
use crate::internal_types::*;
use crate::public_types::*;
use crate::simple_types::*;

// ======================================================
// Section 1 : Define each step in the workflow using types
// ======================================================

pub(crate) trait CheckProductCodeExists:
    AsyncFn1<ProductCode, Output = Result<()>> + Copy
{
}
impl<T> CheckProductCodeExists for T where T: AsyncFn1<ProductCode, Output = Result<()>> + Copy {}

pub(crate) trait CheckAddressExists: AsyncFn1<Address, Output = Result<()>> + Copy {}
impl<T> CheckAddressExists for T where T: AsyncFn1<Address, Output = Result<()>> + Copy {}

pub(crate) trait GetProductPrice:
    AsyncFn1<ProductCode, Output = Result<Price>> + Copy
{
}
impl<T> GetProductPrice for T where T: AsyncFn1<ProductCode, Output = Result<Price>> + Copy {}

pub(crate) trait CalculateShippingCost:
    AsyncFn1<PricedOrder, Output = Result<Price>> + Copy
{
}
impl<T> CalculateShippingCost for T where T: AsyncFn1<PricedOrder, Output = Result<Price>> + Copy {}

pub(crate) trait CreateAcknowledgmentLetter:
    AsyncFn1<PricedOrderWithShipping, Output = Result<Letter>> + Copy
{
}
impl<T> CreateAcknowledgmentLetter for T where
    T: AsyncFn1<PricedOrderWithShipping, Output = Result<Letter>> + Copy
{
}

pub(crate) trait SendOrderAcknowledgement:
    AsyncFn1<Acknowledgment, Output = Result<SendResult>> + Copy
{
}
impl<T> SendOrderAcknowledgement for T where
    T: AsyncFn1<Acknowledgment, Output = Result<SendResult>> + Copy
{
}

pub(crate) trait PlaceOrder:
    AsyncFn1<UnvalidatedOrder, Output = Result<Vec<PlaceOrderEvent>>> + Copy
{
}
impl<T> PlaceOrder for T where
    T: AsyncFn1<UnvalidatedOrder, Output = Result<Vec<PlaceOrderEvent>>> + Copy
{
}

// ======================================================
// Section 2 : Implementation
// ======================================================

// ---------------------------
// ValidateOrder step
// ---------------------------

async fn to_valid_order_line(
    check_product_code_exists: impl CheckProductCodeExists,
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
    let fake_code = ProductCode::new("fake-code");
    let codes = Arc::new(vec![fake_code]);
    let codes_ref = &codes;
    let check_product_code_exists = move |code: ProductCode| async move {
        let codes = codes_ref.clone();
        if codes.contains(&code) {
            return Ok(());
        }
        return Err(anyhow!("Arg"));
    };
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

async fn validate_order(
    check_product_exists: impl CheckProductCodeExists,
    check_address_exists: impl CheckAddressExists,
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

// ---------------------------
// PriceOrder step
// ---------------------------

async fn to_priced_order_line(
    get_product_price: impl GetProductPrice,
    validated_order_line: ValidatedOrderLine,
) -> Result<PricedOrderLine> {
    let line_price = get_product_price(validated_order_line.product_code).await?;

    let priced_order_line = PricedOrderLine {
        order_line_id: validated_order_line.order_line_id,
        line_price,
    };
    Ok(priced_order_line)
}

async fn price_order(
    get_product_price: impl GetProductPrice,
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
// ---------------------------
// Shipping step
// ---------------------------

async fn add_shipping_info_to_order(
    calculate_shipping_cost: impl CalculateShippingCost,
    priced_order: PricedOrder,
) -> Result<PricedOrderWithShipping> {
    let price = calculate_shipping_cost(priced_order.clone()).await?;

    let shipping = ShippingInfo {
        method: ShippingMethod::Postal,
        price,
    };

    let priced_order_with_shipping = PricedOrderWithShipping {
        priced_order,
        shipping,
    };

    Ok(priced_order_with_shipping)
}

// ---------------------------
// AcknowledgeOrder step
// ---------------------------

async fn acknowledge_order(
    create_acknowledgment_letter: impl CreateAcknowledgmentLetter,
    send_order_acknowledgement: impl SendOrderAcknowledgement,
    priced_order: PricedOrderWithShipping,
) -> Result<Option<OrderId>> {
    let letter = create_acknowledgment_letter(priced_order.clone()).await?;
    let acknowledgement = Acknowledgment { letter };
    match send_order_acknowledgement(acknowledgement).await? {
        SendResult::Sent => Ok(Some(priced_order.priced_order.order_id)),
        SendResult::NotSent => Ok(None),
    }
}

// ---------------------------
// Create events
// ---------------------------

fn create_shipping_event(placed_order: PricedOrder) -> PlaceOrderEvent {
    ShippableOrderPlaced {
        order_id: placed_order.order_id,
    }
    .into()
}
fn create_billing_event(placed_order: PricedOrder) -> PlaceOrderEvent {
    BillableOrderPlaced {
        order_id: placed_order.order_id,
        amount_to_bill: placed_order.amount_to_bill.value(),
    }
    .into()
}
fn create_acknowledgment_event(order_id: OrderId) -> PlaceOrderEvent {
    AcknowledgmentSent { order_id }.into()
}

fn create_events(
    priced_order: PricedOrderWithShipping,
    acknowledgment_option: Option<OrderId>,
) -> Vec<PlaceOrderEvent> {
    let acknowledgment_events = acknowledgment_option
        .map(create_acknowledgment_event)
        .map(|e| vec![e])
        .unwrap_or(vec![]);

    let billing_events = vec![create_billing_event(priced_order.priced_order.clone())];
    let shipping_events = vec![create_shipping_event(priced_order.priced_order.clone())];

    [acknowledgment_events, billing_events, shipping_events].concat()
}

// ---------------------------
// overall workflow
// ---------------------------

/// A workflow to place an order and return events, in the flavor of Scott Wlaschins Domain Modelling Made Functional
pub(crate) async fn place_order(
    check_product_exists: impl CheckProductCodeExists,
    check_address_exists: impl CheckAddressExists,
    get_product_price: impl GetProductPrice,
    calculate_shipping_cost: impl CalculateShippingCost,
    create_acknowledgment_letter: impl CreateAcknowledgmentLetter,
    send_order_acknowledgement: impl SendOrderAcknowledgement,
    // unvalidated_order: UnvalidatedOrder,
) -> impl PlaceOrder {
    move |unvalidated_order: UnvalidatedOrder| async move {
        let validated_order = validate_order(
            check_product_exists,
            check_address_exists,
            unvalidated_order,
        )
        .await?;

        let priced_order = price_order(get_product_price, validated_order).await?;

        let priced_order_with_shipping =
            add_shipping_info_to_order(calculate_shipping_cost, priced_order).await?;

        let acknowledgment_option = acknowledge_order(
            create_acknowledgment_letter,
            send_order_acknowledgement,
            priced_order_with_shipping.clone(),
        )
        .await?;
        let events = create_events(priced_order_with_shipping, acknowledgment_option);
        Ok(events)
    }
}
