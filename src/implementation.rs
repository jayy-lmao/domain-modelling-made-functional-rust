use std::borrow::Cow;
use std::rc::Rc;
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
// Section 2 : Implementation
// ======================================================

// ---------------------------
// ValidateOrder step
// ---------------------------

async fn to_valid_order_line<'a>(
    check_product_code_exists: impl AsyncFn1<ProductCode<'a>, Output = Result<()>>,
    unvalidated_line: UnvalidatedOrderLine<'a>,
) -> Result<ValidatedOrderLine<'a>> {
    let product_code = ProductCode::new(unvalidated_line.product_code);
    let order_line_id = OrderLineId::new(unvalidated_line.order_line_id);

    check_product_code_exists(product_code).await?;

    Ok(ValidatedOrderLine {
        product_code,
        order_line_id,
    })
}
#[tokio::test]
async fn converts_to_order_line<'a>() {
    let fake_code = ProductCode::new("fake-code");
    let codes = Arc::new(vec![fake_code]);
    let codes_ref = &codes;
    let check_product_code_exists = move |code: ProductCode<'a>| async move {
        let codes = codes_ref.clone();
        if codes.contains(&code) {
            return Ok(());
        }
        return Err(anyhow!("Arg"));
    };
    let code = "fake-code";
    let line_id = "some-id";

    let unvalidated_line = UnvalidatedOrderLine {
        product_code: code,
        order_line_id: line_id,
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

async fn validate_order<'a>(
    check_product_exists: impl AsyncFn1<ProductCode<'a>, Output = Result<()>> + Copy,
    _check_address_exists: impl AsyncFn1<Address, Output = Result<()>>,
    unvalidated_order: UnvalidatedOrder<'a>,
) -> Result<ValidatedOrder<'a>> {
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

async fn to_priced_order_line<'a>(
    get_product_price: impl AsyncFn1<ProductCode<'a>, Output = Result<Price>> + Copy,
    validated_order_line: ValidatedOrderLine<'a>,
) -> Result<PricedOrderLine<'a>> {
    let line_price = get_product_price(validated_order_line.product_code).await?;

    let priced_order_line = PricedOrderLine {
        order_line_id: validated_order_line.order_line_id,
        line_price,
    };
    Ok(priced_order_line)
}

async fn price_order<'a>(
    get_product_price: impl AsyncFn1<ProductCode<'a>, Output = Result<Price>> + Copy,
    validated_order: ValidatedOrder<'a>,
) -> Result<PricedOrder<'a>> {
    let lines = try_join_all(
        validated_order
            .lines
            .into_iter()
            .map(|validated_line| to_priced_order_line(get_product_price, validated_line)),
    )
    .await?;

    let amount_to_bill = BillingAmount::sum_prices(lines.iter().map(|p| p.line_price));

    let priced_order = PricedOrder {
        order_id: validated_order.id,
        amount_to_bill,
        lines: Cow::from(lines),
    };
    Ok(priced_order)
}
// ---------------------------
// Shipping step
// ---------------------------

async fn add_shipping_info_to_order<'a>(
    calculate_shipping_cost: impl AsyncFn1<PricedOrder<'a>, Output = Result<Price>>,
    priced_order: PricedOrder<'a>,
) -> Result<PricedOrderWithShipping<'a>> {
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

async fn acknowledge_order<'a>(
    create_acknowledgment_letter: impl AsyncFn1<PricedOrderWithShipping<'a>, Output = Result<Letter>>,
    send_order_acknowledgement: impl AsyncFn1<Acknowledgment, Output = Result<SendResult>>,
    priced_order: PricedOrderWithShipping<'a>,
) -> Result<Option<OrderId<'a>>> {
    let id = priced_order.priced_order.order_id;
    let letter = create_acknowledgment_letter(priced_order).await?;
    let acknowledgement = Acknowledgment { letter };
    match send_order_acknowledgement(acknowledgement).await? {
        SendResult::Sent => Ok(Some(id)),
        SendResult::NotSent => Ok(None),
    }
}

// ---------------------------
// Create events
// ---------------------------

fn create_shipping_event<'a>(placed_order: &PricedOrder<'a>) -> PlaceOrderEvent<'a> {
    ShippableOrderPlaced {
        order_id: placed_order.order_id,
    }
    .into()
}
fn create_billing_event<'a>(placed_order: &PricedOrder<'a>) -> PlaceOrderEvent<'a> {
    BillableOrderPlaced {
        order_id: placed_order.order_id,
        amount_to_bill: placed_order.amount_to_bill.value(),
    }
    .into()
}
fn create_acknowledgment_event(order_id: OrderId) -> PlaceOrderEvent {
    AcknowledgmentSent { order_id }.into()
}

fn create_events<'a>(
    priced_order: PricedOrderWithShipping<'a>,
    acknowledgment_option: Option<OrderId<'a>>,
) -> Vec<PlaceOrderEvent<'a>> {
    let acknowledgment_events = acknowledgment_option
        .map(create_acknowledgment_event)
        .map(|e| vec![e])
        .unwrap_or_default();

    let billing_events = vec![create_billing_event(&priced_order.priced_order)];
    let shipping_events = vec![create_shipping_event(&priced_order.priced_order)];

    [acknowledgment_events, billing_events, shipping_events].concat()
}

// ---------------------------
// overall workflow
// ---------------------------

/// A workflow to place an order and return events, in the flavor of Scott Wlaschins Domain Modelling Made Functional
pub(crate) async fn place_order<'a>(
    check_product_exists: impl AsyncFn1<ProductCode<'a>, Output = Result<()>> + Copy,
    check_address_exists: impl AsyncFn1<Address, Output = Result<()>>,
    get_product_price: impl AsyncFn1<ProductCode<'a>, Output = Result<Price>> + Copy,
    calculate_shipping_cost: impl AsyncFn1<PricedOrder<'a>, Output = Result<Price>>,
    create_acknowledgment_letter: impl AsyncFn1<PricedOrderWithShipping<'a>, Output = Result<Letter>>,
    send_order_acknowledgement: impl AsyncFn1<Acknowledgment, Output = Result<SendResult>>,
    // unvalidated_order: UnvalidatedOrder,
) -> impl AsyncFnOnce1<UnvalidatedOrder<'a>, Output = Result<Vec<PlaceOrderEvent<'a>>>> {
    move |unvalidated_order: UnvalidatedOrder<'a>| async move {
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
