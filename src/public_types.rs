use crate::simple_types::*;

#[derive(Clone)]
pub(crate) struct AcknowledgmentSent<'a> {
    pub(crate) order_id: OrderId<'a>,
}
#[derive(Clone)]
pub(crate) struct ShippableOrderPlaced<'a> {
    pub(crate) order_id: OrderId<'a>,
}

#[derive(Clone)]
pub(crate) struct BillableOrderPlaced<'a> {
    pub(crate) order_id: OrderId<'a>,
    pub(crate) amount_to_bill: Price,
}

#[derive(Clone)]
pub(crate) enum PlaceOrderEvent<'a> {
    AcknowledgmentSent(AcknowledgmentSent<'a>),
    ShippableOrderPlaced(ShippableOrderPlaced<'a>),
    BillableOrderPlaced(BillableOrderPlaced<'a>),
}

impl<'a> From<AcknowledgmentSent<'a>> for PlaceOrderEvent<'a> {
    fn from(v: AcknowledgmentSent<'a>) -> Self {
        Self::AcknowledgmentSent(v)
    }
}

impl<'a> From<BillableOrderPlaced<'a>> for PlaceOrderEvent<'a> {
    fn from(v: BillableOrderPlaced<'a>) -> Self {
        Self::BillableOrderPlaced(v)
    }
}

impl<'a> From<ShippableOrderPlaced<'a>> for PlaceOrderEvent<'a> {
    fn from(v: ShippableOrderPlaced<'a>) -> Self {
        Self::ShippableOrderPlaced(v)
    }
}
