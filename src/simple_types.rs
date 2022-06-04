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
    pub(crate) fn new(code: impl Into<String>) -> Self {
        Self { value: code.into() }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Price {
    value: f64,
}

impl Price {
    pub(crate) fn value(&self) -> f64 {
        self.value
    }
    pub(crate) fn new(value: f64) -> Self {
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
