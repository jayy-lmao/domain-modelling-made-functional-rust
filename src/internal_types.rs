use crate::simple_types::*;

#[derive(Clone)]
pub(crate) enum SendResult {
    Sent,
    NotSent,
}

#[derive(Clone, Copy)]
pub(crate) enum ShippingMethod {
    Postal,
    Fedex,
}
#[derive(Clone, Copy)]
pub(crate) struct ShippingInfo {
    pub(crate) method: ShippingMethod,
    pub(crate) price: Price,
}

#[derive(Clone)]
pub(crate) struct PricedOrderWithShipping<'a> {
    pub(crate) priced_order: PricedOrder<'a>,
    pub(crate) shipping: ShippingInfo,
}

#[derive(Clone, Copy)]
pub(crate) struct PricedOrderLine<'a> {
    pub(crate) order_line_id: OrderLineId<'a>,
    pub(crate) line_price: Price,
}

#[derive(Clone)]
pub(crate) struct PricedOrder<'a> {
    pub(crate) order_id: OrderId<'a>,
    pub(crate) amount_to_bill: BillingAmount,
    pub(crate) lines: Vec<PricedOrderLine<'a>>,
}

pub(crate) struct UnvalidatedOrderLine<'a> {
    pub(crate) order_line_id: &'a str,
    pub(crate) product_code: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ValidatedOrderLine<'a> {
    pub(crate) order_line_id: OrderLineId<'a>,
    pub(crate) product_code: ProductCode,
}
pub(crate) struct UnvalidatedOrder<'a> {
    pub(crate) id: &'a str,
    pub(crate) lines: Vec<UnvalidatedOrderLine<'a>>,
}
pub(crate) struct ValidatedOrder<'a> {
    pub(crate) id: OrderId<'a>,
    pub(crate) lines: Vec<ValidatedOrderLine<'a>>,
}
