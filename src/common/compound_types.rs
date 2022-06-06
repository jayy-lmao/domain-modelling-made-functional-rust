pub(crate) struct Address {
    street: String,
}

pub(crate) struct Letter {
    pub(crate) content: String,
}

pub(crate) struct Acknowledgment {
    // email_address,
    pub(crate) letter: Letter,
}
