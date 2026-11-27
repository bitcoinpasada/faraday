use core::marker::PhantomData;
use faraday_kernel::Never;

/// The page under the sheets: one variant per page.
pub enum Body<'s> {
    Spend(Box<faraday_spend::Model<'s>>),
    Decode(faraday_decode::Model<'s>),
    NonceCheck(faraday_decode::Model<'s>),
    SignerCheck(faraday_decode::Model<'s>),
    Message(faraday_messages::Model<'s>),
    CheckMessage(faraday_messages::Model<'s>),
}

/// The sheets over a page.
pub enum Over<'s> {
    #[doc(hidden)]
    None(Never, PhantomData<&'s ()>),
}
