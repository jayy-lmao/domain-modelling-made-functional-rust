use anyhow::{anyhow, Result};

pub mod dto {
    pub struct OrderDto;
    pub struct WorkflowResultDto;
}
use dto::*;

use crate::{
    common::{
        compound_types::{Acknowledgment, Address, Letter},
        simple_types::{Price, ProductCode},
    },
    place_order::internal_types::{PricedOrder, PricedOrderWithShipping, SendResult},
};

async fn place_order(_order: OrderDto) -> Result<()> {
    let product_exists = true;

    let check_product_exists = |_code: ProductCode| async {
        if product_exists {
            Ok(())
        } else {
            Err(anyhow!("Product does not exist"))
        }
    };
    let check_address_exists = |_address: Address| async { Ok(()) };
    let get_product_price = |_p: ProductCode| async { Ok(Price::new(5.)) };
    let calculate_shipping_cost = |_p: PricedOrder| async { Ok(Price::new(2.)) };
    let create_acknowledgment_letter = |_p: PricedOrderWithShipping| async {
        Ok(Letter {
            content: "Foo".into(),
        })
    };
    let send_order_acknowledgement = |_a: Acknowledgment| async { Ok(SendResult::Sent) };

    let _workflow = crate::place_order::implementation::place_order(
        check_product_exists,
        check_address_exists,
        get_product_price,
        calculate_shipping_cost,
        create_acknowledgment_letter,
        send_order_acknowledgement,
    );

    Ok(())
}
