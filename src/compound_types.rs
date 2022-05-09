pub(crate) struct Address {
    street: String,
}

pub(crate) struct Letter {
    content: String,
}

pub(crate) struct Acknowledgment {
    // email_address,
    pub(crate) letter: Letter,
}
