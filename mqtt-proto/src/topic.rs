use std::convert::{AsRef, Into};
use std::fmt::{self, Display, Formatter, Write};
use std::io;
use std::iter::Iterator;
use std::ops::{Deref, DerefMut, Div, DivAssign};
use std::str::FromStr;

use slab::Slab;

use errors::{Error, ErrorKind, Result};

fn is_metadata<T: AsRef<str>>(s: T) -> bool {
    s.as_ref().chars().nth(0) == Some('$')
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum Level {
    Normal(String),
    Metadata(String), // $SYS
    Blank,
    SingleWildcard, // Single level wildcard +
    MultiWildcard, // Multi-level wildcard #
}

unsafe impl Send for Level {}
unsafe impl Sync for Level {}

impl Level {
    pub fn normal<T: AsRef<str>>(s: T) -> Result<Level> {
        let s = s.as_ref();

        if s.contains(|c| c == '+' || c == '#') {
            bail!(ErrorKind::InvalidTopic(
                format!("invalid normal level `{}` contains +|#", s),
            ));
        }

        if s.chars().nth(0) == Some('$') {
            bail!(ErrorKind::InvalidTopic(
                format!("invalid normal level `{}` starts with $", s),
            ))
        }

        Ok(Level::Normal(s.to_owned()))
    }

    pub fn metadata<T: AsRef<str>>(s: T) -> Result<Level> {
        let s = s.as_ref();

        if s.contains(|c| c == '+' || c == '#') {
            bail!(ErrorKind::InvalidTopic(format!(
                "invalid metadata level `{}` contains +|#",
                s,
            )));
        }

        if s.chars().nth(0) != Some('$') {
            bail!(ErrorKind::InvalidTopic(format!(
                "invalid metadata level `{}` not starts with $",
                s,
            )));
        }

        Ok(Level::Metadata(s.to_owned()))
    }

    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Level::Normal(ref s) |
            Level::Metadata(ref s) => Some(s),
            _ => None,
        }
    }

    pub fn is_normal(&self) -> bool {
        if let Level::Normal(_) = *self {
            true
        } else {
            false
        }
    }

    pub fn is_metadata(&self) -> bool {
        if let Level::Metadata(_) = *self {
            true
        } else {
            false
        }
    }

    pub fn is_valid(&self) -> bool {
        match *self {
            Level::Normal(ref s) => {
                s.chars().nth(0) != Some('$') && !s.contains(|c| c == '+' || c == '#')
            }
            Level::Metadata(ref s) => {
                s.chars().nth(0) == Some('$') && !s.contains(|c| c == '+' || c == '#')
            }
            _ => true,
        }
    }
}

#[macro_export]
macro_rules! normal {
    ($level:expr) => ($crate::Level::normal($level).unwrap())
}

#[macro_export]
macro_rules! metadata {
    ($level:expr) => ($crate::Level::metadata($level).unwrap())
}

#[derive(Debug, Eq, PartialEq, Clone, Default, Hash)]
pub struct Filter(Vec<Level>);

unsafe impl Send for Filter {}
unsafe impl Sync for Filter {}

impl Filter {
    pub fn levels(&self) -> &[Level] {
        &self.0
    }

    pub fn is_valid(&self) -> bool {
        self.0
            .iter()
            .position(|level| !level.is_valid())
            .or_else(|| {
                self.0.iter().enumerate().position(
                    |(pos, level)| match *level {
                        Level::MultiWildcard => pos != self.0.len() - 1,
                        Level::Metadata(_) => pos != 0,
                        _ => false,
                    },
                )
            })
            .is_none()
    }
}

impl<'a> From<&'a [Level]> for Filter {
    fn from(s: &[Level]) -> Self {
        let mut v = vec![];

        v.extend_from_slice(s);

        Filter(v)
    }
}

impl From<Vec<Level>> for Filter {
    fn from(v: Vec<Level>) -> Self {
        Filter(v)
    }
}

impl Into<Vec<Level>> for Filter {
    fn into(self) -> Vec<Level> {
        self.0
    }
}

impl Deref for Filter {
    type Target = Vec<Level>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Filter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[macro_export]
macro_rules! topic_filter {
    ($s:expr) => ($s.parse::<Filter>().unwrap());
}

pub trait MatchLevel {
    fn match_level(&self, level: &Level) -> bool;
}

impl MatchLevel for Level {
    fn match_level(&self, level: &Level) -> bool {
        match *level {
            Level::Normal(ref lhs) => {
                if let Level::Normal(ref rhs) = *self {
                    lhs == rhs
                } else {
                    false
                }
            }
            Level::Metadata(ref lhs) => {
                if let Level::Metadata(ref rhs) = *self {
                    lhs == rhs
                } else {
                    false
                }
            }
            Level::Blank => *self == *level,
            Level::SingleWildcard | Level::MultiWildcard => !self.is_metadata(),
        }
    }
}

impl<T: AsRef<str>> MatchLevel for T {
    fn match_level(&self, level: &Level) -> bool {
        match *level {
            Level::Normal(ref lhs) => !is_metadata(self) && lhs == self.as_ref(),
            Level::Metadata(ref lhs) => is_metadata(self) && lhs == self.as_ref(),
            Level::Blank => self.as_ref().is_empty(),
            Level::SingleWildcard | Level::MultiWildcard => !is_metadata(self),
        }
    }
}

macro_rules! match_topic {
    ($filter:expr, $levels:expr) => ({
        let mut lhs = $filter.0.iter();

        for rhs in $levels {
            match lhs.next() {
                Some(&Level::SingleWildcard) => {
                    if !rhs.match_level(&Level::SingleWildcard) {
                        break
                    }
                },
                Some(&Level::MultiWildcard) => {
                    return rhs.match_level(&Level::MultiWildcard);
                }
                Some(level) if rhs.match_level(level) => continue,
                _ => return false,
            }
        }

        match lhs.next() {
            Some(&Level::MultiWildcard) => true,
            Some(_) => false,
            None => true,
        }
    })
}

pub trait MatchTopic {
    fn match_topic(&self, filter: &Filter) -> bool;
}

impl MatchTopic for Filter {
    fn match_topic(&self, filter: &Filter) -> bool {
        match_topic!(filter, &self.0)
    }
}

impl<T: AsRef<str>> MatchTopic for T {
    fn match_topic(&self, filter: &Filter) -> bool {
        match_topic!(filter, self.as_ref().split('/'))
    }
}

impl FromStr for Level {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "+" => Ok(Level::SingleWildcard),
            "#" => Ok(Level::MultiWildcard),
            "" => Ok(Level::Blank),
            _ if s.contains(|c| c == '+' || c == '#') => {
                bail!(ErrorKind::InvalidTopic(s.to_owned()))
            }
            level => Ok(if level.as_bytes()[0] == b'$' {
                Level::Metadata(s.into())
            } else {
                Level::Normal(level.into())
            }),
        }
    }
}

impl<S: AsRef<str>> From<S> for Level {
    fn from(s: S) -> Self {
        s.as_ref().parse().unwrap()
    }
}

impl FromStr for Filter {
    type Err = Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self> {
        s.split('/')
            .map(|level| level.parse())
            .collect::<Result<Vec<_>>>()
            .map(Filter)
            .and_then(|filter| if filter.is_valid() {
                Ok(filter)
            } else {
                bail!(ErrorKind::InvalidTopic(s.to_owned()))
            })
    }
}

impl Display for Level {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            Level::Normal(ref s) |
            Level::Metadata(ref s) => f.write_str(s.as_str()),
            Level::Blank => Ok(()),
            Level::SingleWildcard => f.write_char('+'),
            Level::MultiWildcard => f.write_char('#'),
        }
    }
}

impl Display for Filter {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut first = true;

        for level in &self.0 {
            if first {
                first = false;
            } else {
                f.write_char('/')?;
            }

            level.fmt(f)?;
        }

        Ok(())
    }
}

pub trait WriteTopicExt: io::Write {
    fn write_level(&mut self, level: &Level) -> io::Result<usize> {
        match *level {
            Level::Normal(ref s) |
            Level::Metadata(ref s) => self.write(s.as_str().as_bytes()),
            Level::Blank => Ok(0),
            Level::SingleWildcard => self.write(b"+"),
            Level::MultiWildcard => self.write(b"#"),
        }
    }

    fn write_topic(&mut self, filter: &Filter) -> io::Result<usize> {
        let mut n = 0;
        let mut first = true;

        for level in filter.levels() {
            if first {
                first = false;
            } else {
                n += self.write(b"/")?;
            }

            n += self.write_level(level)?;
        }

        Ok(n)
    }
}

impl<W: io::Write + ?Sized> WriteTopicExt for W {}

impl Div<Level> for Level {
    type Output = Filter;

    fn div(self, rhs: Level) -> Filter {
        Filter(vec![self, rhs])
    }
}

impl Div<Filter> for Level {
    type Output = Filter;

    fn div(self, rhs: Filter) -> Filter {
        let mut v = vec![self];
        v.append(&mut rhs.into());
        Filter(v)
    }
}

impl Div<Level> for Filter {
    type Output = Filter;

    fn div(self, rhs: Level) -> Filter {
        let mut v: Vec<Level> = self.into();
        v.push(rhs);
        Filter(v)
    }
}

impl Div<Filter> for Filter {
    type Output = Filter;

    fn div(self, rhs: Filter) -> Filter {
        let mut v: Vec<Level> = self.into();
        v.append(&mut rhs.into());
        Filter(v)
    }
}

impl DivAssign<Level> for Filter {
    fn div_assign(&mut self, rhs: Level) {
        self.0.push(rhs)
    }
}

impl DivAssign<Filter> for Filter {
    fn div_assign(&mut self, rhs: Filter) {
        self.0.append(&mut rhs.into())
    }
}

#[cfg(test)]
mod tests {
    extern crate env_logger;

    use super::*;

    #[test]
    fn test_level() {
        assert!(normal!("sport").is_normal());
        assert!(metadata!("$SYS").is_metadata());

        assert_eq!(normal!("sport").as_str(), Some("sport"));
        assert_eq!(metadata!("$SYS").as_str(), Some("$SYS"));

        assert_eq!(normal!("sport"), "sport".parse().unwrap());
        assert_eq!(metadata!("$SYS"), "$SYS".parse().unwrap());

        assert!(Level::Normal("sport".to_owned()).is_valid());
        assert!(Level::Metadata("$SYS".to_owned()).is_valid());

        assert!(!Level::Normal("$sport".to_owned()).is_valid());
        assert!(!Level::Metadata("SYS".to_owned()).is_valid());

        assert!(!Level::Normal("sport#".to_owned()).is_valid());
        assert!(!Level::Metadata("SYS+".to_owned()).is_valid());
    }

    #[test]
    fn test_valid_topic() {
        assert!(
            Filter(vec![
                normal!("sport"),
                normal!("tennis"),
                normal!("player1"),
            ]).is_valid()
        );

        assert!(
            Filter(vec![
                Level::normal("sport").unwrap(),
                Level::normal("tennis").unwrap(),
                Level::MultiWildcard,
            ]).is_valid()
        );
        assert!(
            Filter(vec![
                Level::metadata("$SYS").unwrap(),
                Level::normal("tennis").unwrap(),
                Level::MultiWildcard,
            ]).is_valid()
        );

        assert!(
            Filter(vec![
                Level::normal("sport").unwrap(),
                Level::SingleWildcard,
                Level::normal("player1").unwrap(),
            ]).is_valid()
        );

        assert!(!Filter(vec![
            Level::normal("sport").unwrap(),
            Level::MultiWildcard,
            Level::normal("player1").unwrap(),
        ]).is_valid());
        assert!(!Filter(vec![
            Level::normal("sport").unwrap(),
            Level::metadata("$SYS").unwrap(),
            Level::normal("player1").unwrap(),
        ]).is_valid());
    }

    #[test]
    fn test_parse_topic() {
        assert_eq!(
            topic_filter!("sport/tennis/player1"),
            vec![
                Level::normal("sport").unwrap(),
                Level::normal("tennis").unwrap(),
                Level::normal("player1").unwrap(),
            ].into()
        );

        assert_eq!(topic_filter!(""), Filter(vec![Level::Blank]));
        assert_eq!(
            topic_filter!("/finance"),
            vec![Level::Blank, Level::normal("finance").unwrap()].into()
        );

        assert_eq!(
            topic_filter!("$SYS"),
            vec![Level::metadata("$SYS").unwrap()].into()
        );

        assert!("sport/$SYS".parse::<Filter>().is_err());
    }

    #[test]
    fn test_multi_wildcard_topic() {
        assert_eq!(
            topic_filter!("sport/tennis/#"),
            vec![
                Level::normal("sport").unwrap(),
                Level::normal("tennis").unwrap(),
                Level::MultiWildcard,
            ].into()
        );

        assert_eq!(topic_filter!("#"), vec![Level::MultiWildcard].into());

        assert!("sport/tennis#".parse::<Filter>().is_err());
        assert!("sport/tennis/#/ranking".parse::<Filter>().is_err());
    }

    #[test]
    fn test_single_wildcard_topic() {
        assert_eq!(topic_filter!("+"), vec![Level::SingleWildcard].into());

        assert_eq!(
            topic_filter!("+/tennis/#"),
            vec![
                Level::SingleWildcard,
                Level::normal("tennis").unwrap(),
                Level::MultiWildcard,
            ].into()
        );

        assert_eq!(
            topic_filter!("sport/+/player1"),
            vec![
                Level::normal("sport").unwrap(),
                Level::SingleWildcard,
                Level::normal("player1").unwrap(),
            ].into()
        );

        assert!("sport+".parse::<Filter>().is_err());
    }

    #[test]
    fn test_write_topic() {
        let mut v = vec![];
        let t = vec![
            Level::SingleWildcard,
            Level::normal("tennis").unwrap(),
            Level::MultiWildcard,
        ].into();

        assert_eq!(v.write_topic(&t).unwrap(), 10);
        assert_eq!(v, b"+/tennis/#");

        assert_eq!(format!("{}", t), "+/tennis/#");
        assert_eq!(t.to_string(), "+/tennis/#");
    }

    #[test]
    fn test_match_topic() {
        assert!("test".match_level(&Level::normal("test").unwrap()));
        assert!("$SYS".match_level(&Level::metadata("$SYS").unwrap()));

        let t = "sport/tennis/player1/#".parse().unwrap();

        assert!("sport/tennis/player1".match_topic(&t));
        assert!("sport/tennis/player1/ranking".match_topic(&t));
        assert!("sport/tennis/player1/score/wimbledon".match_topic(&t));

        assert!("sport".match_topic(&"sport/#".parse().unwrap()));

        let t = "sport/tennis/+".parse().unwrap();

        assert!("sport/tennis/player1".match_topic(&t));
        assert!("sport/tennis/player2".match_topic(&t));
        assert!(!"sport/tennis/player1/ranking".match_topic(&t));

        let t = "sport/+".parse().unwrap();

        assert!(!"sport".match_topic(&t));
        assert!("sport/".match_topic(&t));

        assert!("/finance".match_topic(&"+/+".parse().unwrap()));
        assert!("/finance".match_topic(&"/+".parse().unwrap()));
        assert!(!"/finance".match_topic(&"+".parse().unwrap()));

        assert!(!"$SYS".match_topic(&"#".parse().unwrap()));
        assert!(!"$SYS/monitor/Clients".match_topic(
            &"+/monitor/Clients".parse().unwrap(),
        ));
        assert!("$SYS/".match_topic(&"$SYS/#".parse().unwrap()));
        assert!("$SYS/monitor/Clients".match_topic(
            &"$SYS/monitor/+".parse().unwrap(),
        ));
    }

    #[test]
    fn test_operators() {
        assert_eq!(
            normal!("sport") / normal!("tennis") / normal!("player1"),
            "sport/tennis/player1".parse().unwrap()
        );
        assert_eq!(
            topic_filter!("sport/tennis") / normal!("player1"),
            "sport/tennis/player1".parse().unwrap()
        );
        assert_eq!(
            normal!("sport") / topic_filter!("tennis/player1"),
            "sport/tennis/player1".parse().unwrap()
        );
        assert_eq!(
            topic_filter!("sport/tennis") / topic_filter!("player1/ranking"),
            "sport/tennis/player1/ranking".parse().unwrap()
        );

        let mut t = topic_filter!("sport/tennis");

        t /= normal!("player1");

        assert_eq!(t, "sport/tennis/player1".parse().unwrap());

        t /= topic_filter!("ranking");

        assert_eq!(t, "sport/tennis/player1/ranking".parse().unwrap());
    }
}
