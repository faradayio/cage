//! Helper functions for use with `serde`.

use serde::de::{DeserializeOwned, Visitor};
use serde::{self, Deserialize, Deserializer, Serialize};
use serde_yaml;
use std::fmt::{self, Display};
use std::fs;
use std::io;
use std::marker::PhantomData;
use std::path::Path;
use std::str::FromStr;

use crate::errors::{self, ChainErr, ErrorKind};
use crate::util::ConductorPathExt;

/// Load a YAML file using `serde`, and generate the best error we can if
/// it fails.
pub fn load_yaml<T>(path: &Path) -> Result<T, errors::Error>
where
    T: DeserializeOwned,
{
    let mkerr = || ErrorKind::CouldNotReadFile(path.to_owned());
    let f = fs::File::open(&path).chain_err(&mkerr)?;
    serde_yaml::from_reader(io::BufReader::new(f)).chain_err(&mkerr)
}

/// Write `data` to `path` in YAML format.
pub fn dump_yaml<T>(path: &Path, data: &T) -> Result<(), errors::Error>
where
    T: Serialize,
{
    let mkerr = || ErrorKind::CouldNotWriteFile(path.to_owned());
    path.with_guaranteed_parent().chain_err(&mkerr)?;
    let f = fs::File::create(&path).chain_err(&mkerr)?;
    serde_yaml::to_writer(&mut io::BufWriter::new(f), data).chain_err(&mkerr)
}

/// Deserialize a type that we can parse using `FromStr`.
pub fn deserialize_parsable<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: Display,
{
    String::deserialize(deserializer)?
        .parse()
        .map_err(|e| serde::de::Error::custom(format!("{}", e)))
}

/// Deserialize an `Option` wrapping a type that we can parse using
/// `std::FromStr`.
///
/// There may be a better way to do this.  See [serde issue #576][issue].
///
/// [issue]: https://github.com/serde-rs/serde/issues/576
pub fn deserialize_parsable_opt<'de, D, T>(
    deserializer: D,
) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: Display,
{
    /// A wrapper type that allows us to declare `deserialize` for
    /// something carrying an `Option<T>` value.
    struct Wrap<T>(Option<T>);

    #[allow(unused_qualifications)]
    impl<'de, T> Deserialize<'de> for Wrap<T>
    where
        T: FromStr,
        <T as FromStr>::Err: Display,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            /// Declare an internal visitor type to handle our input.
            struct OptVisitor<T>(PhantomData<T>);

            impl<'de, T> Visitor<'de> for OptVisitor<T>
            where
                T: FromStr,
                <T as FromStr>::Err: Display,
            {
                type Value = Wrap<T>;

                fn visit_none<E>(self) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(Wrap(None))
                }

                fn visit_some<D>(
                    self,
                    deserializer: D,
                ) -> Result<Self::Value, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    deserialize_parsable(deserializer).map(|v| Wrap(Some(v)))
                }

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a string or null")
                }
            }

            deserializer.deserialize_option(OptVisitor(PhantomData))
        }
    }

    Wrap::deserialize(deserializer).map(|wrap| wrap.0)
}
