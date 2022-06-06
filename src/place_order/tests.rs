use anyhow::anyhow;
use std::sync::Arc;

use crate::common::simple_types::{OrderLineId, ProductCode};

use super::implementation::to_valid_order_line;
use super::internal_types::{UnvalidatedOrderLine, ValidatedOrderLine};

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
