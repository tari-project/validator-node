use anyhow::{anyhow, Error as AnyhowError};
use postgres::types::*;
use postgres_types::{private::BytesMut, FromSql, IsNull, ToSql};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, error::Error, fmt, str, str::FromStr};

macro_rules! string_enum {
    ($name:ident [$($value:ident),+]) => {
        #[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug, Eq, Hash)]
        pub enum $name {
            $(
                $value,
            )*
        }

        impl Ord for $name {
            fn cmp(&self, other: &$name) -> Ordering {
                self.to_string().cmp(&other.to_string())
            }
        }

        impl PartialOrd  for $name {
             fn partial_cmp(&self, other: &$name) -> Option<Ordering> {
                 Some(self.cmp(&other))
             }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
             let s = match self {
                  $(
                    $name::$value => stringify!($value),
                   )*
                };
                write!(f, "{}", s)
            }
        }

        impl FromStr for $name {
            type Err = AnyhowError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
               $(
                  if s.eq_ignore_ascii_case(stringify!($value)) {
                     return Ok($name::$value);
                  }
               )*

               Err(anyhow!("Unable to parse {} value: {}", stringify!($name).to_string(), s.to_string()))
            }
        }

        impl<'a> ToSql for $name {
            fn to_sql(&self, ty: &Type, w: &mut BytesMut,) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
                <&str as ToSql>::to_sql(&format!("{}", self).as_str(), ty, w)
            }

            accepts!(VARCHAR, TEXT);
            to_sql_checked!();
        }

        impl<'a> FromSql<'a> for $name {
            fn from_sql(_: &Type, raw: &'a [u8]) -> Result<$name, Box<dyn Error + Sync + Send>> {
                Ok(str::from_utf8(raw)?.parse()?)
            }
            accepts!(VARCHAR, TEXT);
        }
    }
}

string_enum! { AssetStatus [Active, Retired]}
string_enum! { CommitteeMode [Public, Creator]}
string_enum! { TokenStatus [Active, Retired]}
string_enum! { AccessResource [Api, Wallet]}

impl Default for CommitteeMode {
    fn default() -> CommitteeMode {
        CommitteeMode::Public
    }
}

#[test]
fn display() {
    assert_eq!(AssetStatus::Active.to_string(), "Active");
    assert_eq!(AssetStatus::Retired.to_string(), "Retired");
    assert_eq!(AccessResource::Api.to_string(), "Api");
}

#[test]
fn parse() {
    assert_eq!(AssetStatus::Active, "Active".parse().unwrap());
    assert_eq!(AssetStatus::Retired, "Retired".parse().unwrap());
    assert_eq!(AssetStatus::Retired, "retired".parse().unwrap());
    assert!("Invalid".parse::<AssetStatus>().is_err());
    assert_eq!(AccessResource::Wallet, "wallet".parse().unwrap());
}
