use crate::common::simple_types::*;


#[derive(Clone)]
pub(crate) struct AcknowledgmentSent {
    pub(crate) order_id: OrderId,
}
#[derive(Clone)]
pub(crate) struct ShippableOrderPlaced {
    pub(crate) order_id: OrderId,
}

#[derive(Clone)]
pub(crate) struct BillableOrderPlaced {
    pub(crate) order_id: OrderId,
    pub(crate) amount_to_bill: Price,
}

#[derive(Clone)]
pub(crate) enum PlaceOrderEvent {
    AcknowledgmentSent(AcknowledgmentSent),
    ShippableOrderPlaced(ShippableOrderPlaced),
    BillableOrderPlaced(BillableOrderPlaced),
}

impl From<AcknowledgmentSent> for PlaceOrderEvent {
    fn from(v: AcknowledgmentSent) -> Self {
        Self::AcknowledgmentSent(v)
    }
}

impl From<BillableOrderPlaced> for PlaceOrderEvent {
    fn from(v: BillableOrderPlaced) -> Self {
        Self::BillableOrderPlaced(v)
    }
}

impl From<ShippableOrderPlaced> for PlaceOrderEvent {
    fn from(v: ShippableOrderPlaced) -> Self {
        Self::ShippableOrderPlaced(v)
    }
}
