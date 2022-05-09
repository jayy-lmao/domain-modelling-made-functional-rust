use std::{future::Future, iter::Sum, pin::Pin};

use anyhow::Result;
use futures_util::future::try_join_all;
use simple_types::*;

mod simple_types {
    use std::iter::Sum;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct OrderLineId {
        value: String,
    }

    impl OrderLineId {
        pub(crate) fn new(id: impl Into<String>) -> Self {
            Self { value: id.into() }
        }
    }
    #[derive(Clone)]
    pub(crate) struct OrderId {
        value: String,
    }

    impl OrderId {
        pub(crate) fn new(id: String) -> Self {
            Self { value: id }
        }
    }

    #[derive(Clone, PartialEq, Eq, Debug)]
    pub(crate) struct ProductCode {
        value: String,
    }

    impl ProductCode {
        pub(crate) fn new(code: String) -> Self {
            Self { value: code }
        }
    }

    #[derive(Clone, Copy, Debug)]
    pub(crate) struct Price {
        value: f64,
    }

    impl Price {
        fn value(&self) -> f64 {
            self.value
        }
        fn new(value: f64) -> Self {
            Self { value }
        }
    }

    impl std::ops::Add for Price {
        type Output = Price;

        fn add(self, rhs: Self) -> Self::Output {
            Self {
                value: self.value + rhs.value,
            }
        }
    }

    impl Sum<Self> for Price {
        fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
            iter.fold(Price::new(0.), |a, b| Price::new(a.value + b.value))
        }
    }

    #[derive(Clone)]
    pub(crate) struct BillingAmount {
        value: Price,
    }

    impl BillingAmount {
        pub(crate) fn value(&self) -> Price {
            self.value
        }
        pub(crate) fn sum_prices(prices: impl Iterator<Item = Price>) -> BillingAmount {
            let sum = prices.sum();
            Self { value: sum }
        }
    }
}

// struct Product {
//     id: String,
// }

mod compound_types {
    pub(crate) struct Address {
        street: String,
    }
}

struct Letter {
    content: String,
}

mod internal_types {
    use crate::simple_types::*;

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
}

pub mod implementation {

    use std::pin::Pin;

    use futures_util::Future;
    use anyhow::Result;
    use futures_util::future::try_join_all;

    use crate::Letter;
    use crate::compound_types::*;
    use crate::internal_types::*;
    use crate::simple_types::*;

    // ======================================================
    // Section 2 : Implementation
    // ======================================================

    // ---------------------------
    // ValidateOrder step
    // ---------------------------

    async fn to_valid_order_line<Fut1: Future<Output = Result<()>>>(
        check_product_code_exists: fn(ProductCode) -> Fut1,
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

    async fn validate_order<
        Fut1: Future<Output = Result<()>>,
        Fut2: Future<Output = Result<()>>,
    >(
        check_product_exists: fn(ProductCode) -> Fut1,
        check_address_exists: fn(Address) -> Fut2,
        unvalidated_order: UnvalidatedOrder,
    ) -> Result<ValidatedOrder> {
        let order_id = OrderId::new(unvalidated_order.id);

        let lines =
            try_join_all(unvalidated_order.lines.into_iter().map(|unvalidated_line| {
                to_valid_order_line(check_product_exists, unvalidated_line)
            }))
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

    async fn to_priced_order_line<Fut1: Future<Output = Result<Price>>>(
        get_product_price: fn(ProductCode) -> Fut1,
        validated_order_line: ValidatedOrderLine,
    ) -> Result<PricedOrderLine> {
        let line_price = get_product_price(validated_order_line.product_code).await?;

        let priced_order_line = PricedOrderLine {
            order_line_id: validated_order_line.order_line_id,
            line_price,
        };
        Ok(priced_order_line)
    }

    async fn price_order<Fut: Future<Output = Result<Price>>>(
        get_product_price: fn(ProductCode) -> Fut,
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

    async fn add_shipping_info_to_order<Fut: Future<Output = Result<Price>>>(
        calculate_shipping_cost: fn(PricedOrder) -> Fut,
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
    struct Acknowledgment {
        // email_address,
        letter: Letter,
    }

    async fn acknowledge_order<
        Fut1: Future<Output = Result<Letter>>,
        Fut2: Future<Output = Result<SendResult>>,
    >(
        create_acknowledgment_letter: fn(PricedOrderWithShipping) -> Fut1,
        send_order_acknowledgement: fn(Acknowledgment) -> Fut2,
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

    #[derive(Clone)]
    struct AcknowledgmentSent {
        order_id: OrderId,
    }
    #[derive(Clone)]
    struct ShippableOrderPlaced {
        order_id: OrderId,
    }

    #[derive(Clone)]
    struct BillableOrderPlaced {
        order_id: OrderId,
        amount_to_bill: Price,
    }

    #[derive(Clone)]
    enum PlaceOrderEvent {
        AcknowledgmentSent(AcknowledgmentSent),
        ShippableOrderPlaced(ShippableOrderPlaced),
        BillableOrderPlaced(BillableOrderPlaced),
    }

    fn create_shipping_event(placed_order: PricedOrder) -> PlaceOrderEvent {
        PlaceOrderEvent::ShippableOrderPlaced(ShippableOrderPlaced {
            order_id: placed_order.order_id,
        })
    }
    fn create_billing_event(placed_order: PricedOrder) -> PlaceOrderEvent {
        PlaceOrderEvent::BillableOrderPlaced(BillableOrderPlaced {
            order_id: placed_order.order_id,
            amount_to_bill: placed_order.amount_to_bill.value(),
        })
    }
    fn create_acknowledgment_event(order_id: OrderId) -> PlaceOrderEvent {
        PlaceOrderEvent::AcknowledgmentSent(AcknowledgmentSent { order_id })
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

    async fn place_order<
        'a,
        Fut1: 'a + Future<Output = Result<()>>,
        Fut2: 'a + Future<Output = Result<()>>,
        Fut3: 'a + Future<Output = Result<Price>>,
        Fut4: 'a + Future<Output = Result<Price>>,
        Fut5: 'a + Future<Output = Result<Letter>>,
        Fut6: 'a + Future<Output = Result<SendResult>>,
    >(
        check_product_exists: fn(ProductCode) -> Fut1,
        check_address_exists: fn(Address) -> Fut2,
        get_product_price: fn(ProductCode) -> Fut3,
        calculate_shipping_cost: fn(PricedOrder) -> Fut4,
        create_acknowledgment_letter: fn(PricedOrderWithShipping) -> Fut5,
        send_order_acknowledgement: fn(Acknowledgment) -> Fut6,
        // unvalidated_order: UnvalidatedOrder,
    ) -> impl Fn(UnvalidatedOrder) -> Pin<Box<dyn Future<Output = Result<Vec<PlaceOrderEvent>>> + 'a>>
    {
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
}
