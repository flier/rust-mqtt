use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::result::Result;

use errors::{Error, ErrorKind};

pub trait Authenticator: Clone {
    type Profile;
    type Error;

    fn auth<'a>(
        &mut self,
        client_id: Cow<'a, str>,
        username: Option<Cow<'a, str>>,
        password: Option<Cow<'a, [u8]>>,
    ) -> Result<Self::Profile, Self::Error>;
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryAuthenticator {
    users: HashMap<String, Vec<u8>>,
}

impl Deref for InMemoryAuthenticator {
    type Target = HashMap<String, Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        &self.users
    }
}

impl DerefMut for InMemoryAuthenticator {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.users
    }
}

impl Authenticator for InMemoryAuthenticator {
    type Profile = ();
    type Error = Error;

    fn auth<'a>(
        &mut self,
        client_id: Cow<'a, str>,
        username: Option<Cow<'a, str>>,
        password: Option<Cow<'a, [u8]>>,
    ) -> Result<(), Self::Error> {
        match username {
            Some(ref u)
                if self.users.get(u.as_ref()).map_or(false, |pass| {
                    password.map_or(true, |p| pass.as_slice() == p.as_ref())
                }) => Ok(()),
            _ => bail!(ErrorKind::BadUserNameOrPassword),
        }
    }
}


#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_in_memory_authenticator() {
        let mut auth = InMemoryAuthenticator::default();

        assert!(auth.is_empty());

        assert_matches!(auth.auth("client".into(), Some("user".into()), Some(Cow::from(&b"pass"[..]))),
                        Err(Error(ErrorKind::BadUserNameOrPassword, _)));

        auth.insert("foo".to_owned(), Vec::from(&b"bar"[..]));
    }
}
