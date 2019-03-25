use std::collections::HashMap;
use std::iter::{FromIterator, IntoIterator};
use std::result::Result as StdResult;
use std::str;

use failure::AsFail;
use pwhash::bcrypt;

use crate::errors::{Error, ErrorKind::*, Result};

pub trait Authenticator: Clone {
    type Profile;
    type Error: AsFail;

    fn authenticate<'a>(
        &mut self,
        client_id: &'a str,
        username: &'a str,
        password: Option<&'a [u8]>,
    ) -> StdResult<Self::Profile, Self::Error>;
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryAuthenticator {
    users: HashMap<String, Option<String>>,
}

impl InMemoryAuthenticator {
    pub fn is_empty(&self) -> bool {
        self.users.is_empty()
    }

    pub fn insert(
        &mut self,
        username: String,
        password: Option<Vec<u8>>,
    ) -> Result<Option<Option<String>>> {
        let value = if let Some(password) = password {
            Some(bcrypt::hash(unsafe {
                str::from_utf8_unchecked(&password)
            })?)
        } else {
            None
        };

        Ok(self.users.insert(username, value))
    }
}

impl FromIterator<(String, Option<Vec<u8>>)> for InMemoryAuthenticator {
    fn from_iter<T: IntoIterator<Item = (String, Option<Vec<u8>>)>>(iter: T) -> Self {
        let mut authenticator = InMemoryAuthenticator::default();

        for (username, password) in iter {
            authenticator.insert(username, password).unwrap();
        }

        authenticator
    }
}

impl Authenticator for InMemoryAuthenticator {
    type Profile = ();
    type Error = Error;

    fn authenticate<'a>(
        &mut self,
        _client_id: &'a str,
        username: &'a str,
        password: Option<&'a [u8]>,
    ) -> Result<()> {
        if self
            .users
            .get(username)
            .map_or(false, |hash| match (password, hash) {
                (Some(pass), &Some(ref hash)) => {
                    bcrypt::verify(unsafe { str::from_utf8_unchecked(pass) }, hash)
                }
                (_, &None) => true,
                _ => false,
            })
        {
            Ok(())
        } else {
            Err(BadUserNameOrPassword.into())
        }
    }
}

pub type MockAuthenticator = ();

impl Authenticator for MockAuthenticator {
    type Profile = ();
    type Error = Error;

    fn authenticate<'a>(
        &mut self,
        _client_id: &'a str,
        _username: &'a str,
        _password: Option<&'a [u8]>,
    ) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::errors::ErrorKind;

    #[test]
    fn test_in_memory_authenticator() {
        let mut auth = InMemoryAuthenticator::default();

        assert!(auth.is_empty());

        assert_matches!(
            auth.authenticate("client", "user", Some(&b"pass"[..]))
                .unwrap_err()
                .downcast_ref()
                .unwrap(),
            &ErrorKind::BadUserNameOrPassword
        );

        assert_eq!(
            auth.insert("user".to_owned(), Some(Vec::from(&b"pass"[..])))
                .unwrap(),
            None
        );

        assert_matches!(
            auth.authenticate("client", "user", Some(&b"pass"[..])),
            Ok(())
        );
    }
}
