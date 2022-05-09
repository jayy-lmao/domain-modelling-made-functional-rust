use anyhow::{anyhow, Result};

pub mod dto {
    pub struct OrderDto;
    pub struct WorkflowResultDto;
}
use dto::*;

use crate::{
    compound_types::{Acknowledgment, Address, Letter},
    internal_types::{PricedOrder, PricedOrderWithShipping, SendResult},
    simple_types::{Price, ProductCode},
};

async fn place_order(order: OrderDto) -> Result<()> {
    let product_exists = true;

    let check_product_exists = |code: ProductCode| async {
        if product_exists {
            Ok(())
        } else {
            Err(anyhow!("Product does not exist"))
        }
    };
    let check_address_exists = |address: Address| async { Ok(()) };
    let get_product_price = |p: ProductCode| async { Ok(Price::new(5.)) };
    let calculate_shipping_cost = |p: PricedOrder| async { Ok(Price::new(2.)) };
    let create_acknowledgment_letter = |p: PricedOrderWithShipping| async {
        Ok(Letter {
            content: "Foo".into(),
        })
    };
    let send_order_acknowledgement = |a: Acknowledgment| async { Ok(SendResult::Sent) };

    let workflow = crate::implementation::place_order(
        check_product_exists,
        check_address_exists,
        get_product_price,
        calculate_shipping_cost,
        create_acknowledgment_letter,
        send_order_acknowledgement,
    );

    Ok(())
}
