use std::pin::Pin;

use anyhow::Result;
use futures_util::future::try_join_all;
use futures_util::Future;

use crate::compound_types::*;
use crate::internal_types::*;
use crate::public_types::*;
use crate::simple_types::*;

// ======================================================
// Section 2 : Implementation
// ======================================================

// ---------------------------
// ValidateOrder step
// ---------------------------

async fn to_valid_order_line<Fn1: Fn(ProductCode) -> Fut1, Fut1: Future<Output = Result<()>>>(
    check_product_code_exists: Fn1,
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

async fn validate_order<Fn1, Fn2, Fut1, Fut2>(
    check_product_exists: Fn1,
    check_address_exists: Fn2,
    unvalidated_order: UnvalidatedOrder,
) -> Result<ValidatedOrder>
where
    Fn1: Fn(ProductCode) -> Fut1 + Clone + Copy,
    Fn2: Fn(Address) -> Fut2,
    Fut1: Future<Output = Result<()>>,
    Fut2: Future<Output = Result<()>>,
{
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

async fn to_priced_order_line<
    Fn1: Fn(ProductCode) -> Fut1,
    Fut1: Future<Output = Result<Price>>,
>(
    get_product_price: Fn1,
    validated_order_line: ValidatedOrderLine,
) -> Result<PricedOrderLine> {
    let line_price = get_product_price(validated_order_line.product_code).await?;

    let priced_order_line = PricedOrderLine {
        order_line_id: validated_order_line.order_line_id,
        line_price,
    };
    Ok(priced_order_line)
}

async fn price_order<
    Fn1: Fn(ProductCode) -> Fut + Clone + Copy,
    Fut: Future<Output = Result<Price>>,
>(
    get_product_price: Fn1,
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

async fn add_shipping_info_to_order<
    Fn1: Fn(PricedOrder) -> Fut,
    Fut: Future<Output = Result<Price>>,
>(
    calculate_shipping_cost: Fn1,
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

async fn acknowledge_order<
    Fn1: Fn(PricedOrderWithShipping) -> Fut1,
    Fn2: Fn(Acknowledgment) -> Fut2,
    Fut1: Future<Output = Result<Letter>>,
    Fut2: Future<Output = Result<SendResult>>,
>(
    create_acknowledgment_letter: Fn1,
    send_order_acknowledgement: Fn2,
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
pub async fn place_order<
    'a,
    Fn1: 'a + Fn(ProductCode) -> Fut1 + Clone + Copy,
    Fn2: 'a + Fn(Address) -> Fut2 + Copy + Clone,
    Fn3: 'a + Fn(ProductCode) -> Fut3 + Clone + Copy,
    Fn4: 'a + Fn(PricedOrder) -> Fut4 + Clone + Copy,
    Fn5: 'a + Fn(PricedOrderWithShipping) -> Fut5 + Clone + Copy,
    Fn6: 'a + Fn(Acknowledgment) -> Fut6 + Clone + Copy,
    Fut1: 'a + Future<Output = Result<()>>,
    Fut2: 'a + Future<Output = Result<()>>,
    Fut3: 'a + Future<Output = Result<Price>>,
    Fut4: 'a + Future<Output = Result<Price>>,
    Fut5: 'a + Future<Output = Result<Letter>>,
    Fut6: 'a + Future<Output = Result<SendResult>>,
>(
    check_product_exists: Fn1,
    check_address_exists: Fn2,
    get_product_price: Fn3,
    calculate_shipping_cost: Fn4,
    create_acknowledgment_letter: Fn5,
    send_order_acknowledgement: Fn6,
    // unvalidated_order: UnvalidatedOrder,
) -> impl Fn(UnvalidatedOrder) -> Pin<Box<dyn Future<Output = Result<Vec<PlaceOrderEvent>>> + 'a>> {
    move |unvalidated_order: UnvalidatedOrder| {
        Box::pin(async move {
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
        })
    }
}
