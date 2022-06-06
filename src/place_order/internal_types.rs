use crate::common::simple_types::*;

#[derive(Clone)]
pub(crate) enum SendResult {
    Sent,
    NotSent,
}

#[derive(Clone)]
pub(crate) enum ShippingMethod {
    Postal,
    Fedex,
}
#[derive(Clone)]
pub(crate) struct ShippingInfo {
    pub(crate) method: ShippingMethod,
    pub(crate) price: Price,
}

#[derive(Clone)]
pub(crate) struct PricedOrderWithShipping {
    pub(crate) priced_order: PricedOrder,
    pub(crate) shipping: ShippingInfo,
}

#[derive(Clone)]
pub(crate) struct PricedOrderLine {
    pub(crate) order_line_id: OrderLineId,
    pub(crate) line_price: Price,
}

#[derive(Clone)]
pub(crate) struct PricedOrder {
    pub(crate) order_id: OrderId,
    pub(crate) amount_to_bill: BillingAmount,
    pub(crate) lines: Vec<PricedOrderLine>,
}

pub(crate) struct UnvalidatedOrderLine {
    pub(crate) order_line_id: String,
    pub(crate) product_code: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ValidatedOrderLine {
    pub(crate) order_line_id: OrderLineId,
    pub(crate) product_code: ProductCode,
}
pub(crate) struct UnvalidatedOrder {
    pub(crate) id: String,
    pub(crate) lines: Vec<UnvalidatedOrderLine>,
}
pub(crate) struct ValidatedOrder {
    pub(crate) id: OrderId,
    pub(crate) lines: Vec<ValidatedOrderLine>,
}
