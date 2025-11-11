//! Helper functions for use with `serde`.

use serde::de::{DeserializeOwned, Visitor};
use serde::{self, Deserialize, Deserializer, Serialize};
use std::fmt::{self, Display};
use std::fs;
use std::io;
use std::marker::PhantomData;
use std::path::Path;
use std::str::FromStr;

use crate::errors::{Error, Result};
use crate::util::ConductorPathExt;

/// Load a YAML file using `serde`, and generate the best error we can if
/// it fails.
pub fn load_yaml<T>(path: &Path) -> Result<T>
where
    T: DeserializeOwned,
{
    let f = fs::File::open(path).map_err(|e| {
        anyhow::Error::new(e).context(Error::CouldNotReadFile(path.to_owned()))
    })?;
    serde_yaml::from_reader(io::BufReader::new(f)).map_err(|e| {
        anyhow::Error::new(e).context(Error::CouldNotReadFile(path.to_owned()))
    })
}

/// Write `data` to `path` in YAML format.
pub fn dump_yaml<T>(path: &Path, data: &T) -> Result<()>
where
    T: Serialize,
{
    path.with_guaranteed_parent()
        .map_err(|e| e.context(Error::CouldNotWriteFile(path.to_owned())))?;
    let f = fs::File::create(path).map_err(|e| {
        anyhow::Error::new(e).context(Error::CouldNotWriteFile(path.to_owned()))
    })?;
    serde_yaml::to_writer(&mut io::BufWriter::new(f), data).map_err(|e| {
        anyhow::Error::new(e).context(Error::CouldNotWriteFile(path.to_owned()))
    })
}

/// Deserialize a type that we can parse using `FromStr`.
pub fn deserialize_parsable<'de, D, T>(
    deserializer: D,
) -> std::result::Result<T, D::Error>
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
) -> std::result::Result<Option<T>, D::Error>
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
        fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
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

                fn visit_none<E>(self) -> std::result::Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(Wrap(None))
                }

                fn visit_some<D>(
                    self,
                    deserializer: D,
                ) -> std::result::Result<Self::Value, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    deserialize_parsable(deserializer).map(|v| Wrap(Some(v)))
                }

                fn expecting(
                    &self,
                    formatter: &mut fmt::Formatter<'_>,
                ) -> fmt::Result {
                    formatter.write_str("a string or null")
                }
            }

            deserializer.deserialize_option(OptVisitor(PhantomData))
        }
    }

    Wrap::deserialize(deserializer).map(|wrap| wrap.0)
}

/// Tools for (de)serializing `std::time::SystemTime` as whole seconds from the Unix epoch.
///
/// This discards any fractional seconds without rounding.
pub(crate) mod seconds_since_epoch {
    use serde::{self, ser, Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    /// Deserialize a number of seconds since the Unix epoch as a system time.
    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_secs(secs))
    }

    /// Deserialize a system time as a number of seconds since the Unix epoch.
    pub(crate) fn serialize<S>(
        time: &SystemTime,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let secs = time
            .duration_since(UNIX_EPOCH)
            .map_err(<S::Error as ser::Error>::custom)?
            .as_secs();
        secs.serialize(serializer)
    }
}
