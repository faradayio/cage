//! Helper functions for use with `serde`.

use serde::{self, Deserialize, Deserializer};
use serde::de::Visitor;
use std::fmt::Display;
use std::marker::PhantomData;
use std::str::FromStr;

/// Deserialize a type that we can parse using `FromStr`.
pub fn deserialize_parsable<D, T>(deserializer: &mut D) -> Result<T, D::Error>
    where D: Deserializer,
          T: FromStr,
          <T as FromStr>::Err: Display
{
    try!(String::deserialize(deserializer))
        .parse()
        .map_err(|e| serde::Error::custom(format!("{}", e)))
}

/// Deserialize an `Option` wrapping a type that we can parse using
/// `std::FromStr`.
///
/// There may be a better way to do this.  See [serde issue #576][issue].
///
/// [issue]: https://github.com/serde-rs/serde/issues/576
pub fn deserialize_parsable_opt<D, T>(deserializer: &mut D)
                                      -> Result<Option<T>, D::Error>
    where D: Deserializer,
          T: FromStr,
          <T as FromStr>::Err: Display
{
    /// A wrapper type that allows us to declare `deserialize` for
    /// something carrying an `Option<T>` value.
    struct Wrap<T>(Option<T>);

    #[allow(unused_qualifications)]
    impl<T> Deserialize for Wrap<T>
        where T: FromStr,
              <T as FromStr>::Err: Display
    {
        fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
            where D: Deserializer
        {
            /// Declare an internal visitor type to handle our input.
            struct OptVisitor<T>(PhantomData<T>);

            impl<T> Visitor for OptVisitor<T>
                where T: FromStr,
                      <T as FromStr>::Err: Display
            {
                type Value = Wrap<T>;

                fn visit_none<E>(&mut self) -> Result<Self::Value, E>
                    where E: serde::Error
                {
                    Ok(Wrap(None))
                }

                fn visit_some<D>(&mut self,
                                 deserializer: &mut D)
                                 -> Result<Self::Value, D::Error>
                    where D: Deserializer
                {
                    deserialize_parsable(deserializer).map(|v| Wrap(Some(v)))
                }
            }

            deserializer.deserialize_option(OptVisitor(PhantomData))
        }
    }

    Wrap::deserialize(deserializer).map(|wrap| wrap.0)
}
