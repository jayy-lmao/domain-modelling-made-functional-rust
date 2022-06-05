use std::iter::Sum;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OrderLineId<'a> {
    value: &'a str,
}

impl<'a> OrderLineId<'a> {
    pub(crate) fn new(id: &'a str) -> Self {
        Self { value: id }
    }
}
#[derive(Clone, Copy)]
pub(crate) struct OrderId<'a> {
    value: &'a str,
}

impl<'a> OrderId<'a> {
    pub(crate) fn new(id: &'a str) -> Self {
        Self { value: id }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ProductCode<'a> {
    value: &'a str,
}

impl<'a> ProductCode<'a> {
    pub(crate) fn new(code: &'a str) -> Self {
        Self { value: code }
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

#[derive(Clone, Copy)]
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
