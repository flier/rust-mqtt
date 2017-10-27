use std::borrow::Cow;
use std::result::Result;

pub trait Authenticator: Clone {
    type Error;

    fn auth<'a>(
        &mut self,
        client_id: Cow<'a, str>,
        username: Option<Cow<'a, str>>,
        password: Option<Cow<'a, [u8]>>,
    ) -> Result<(), Self::Error>;
}
