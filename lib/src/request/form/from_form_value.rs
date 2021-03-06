use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6, SocketAddr};
use std::str::FromStr;

use error::Error;
use http::uri::URI;

/// Trait to create instance of some type from a form value; expected from field
/// types in structs deriving `FromForm`.
///
/// When deriving the `FromForm` trait, Rocket uses the `FromFormValue`
/// implementation of each field's type to validate the form input. To
/// illustrate, consider the following structure:
///
/// ```rust,ignore
/// #[derive(FromForm)]
/// struct Person {
///     name: String,
///     age: u16
/// }
/// ```
///
/// The `FromForm` implementation generated by Rocket will call
/// `String::from_form_value` for the `name` field, and `u16::from_form_value`
/// for the `age` field. The `Person` structure can only be created from a form
/// if both calls return successfully.
///
/// ## Catching Validation Errors
///
/// Sometimes you want to be informed of validation errors. When this is
/// desired, types of `Option<T>` or `Result<T, T::Error>` can be used. These
/// types implement `FromFormValue` themselves. Their implementations always
/// return successfully, so their validation never fails. They can be used to
/// determine if the `from_form_value` call failed and to retrieve the error
/// value from the failed call.
///
/// For instance, if we wanted to know if a user entered an invalid `age` in the
/// form corresponding to the `Person` structure above, we could use the
/// following structure:
///
/// ```rust
/// # #[allow(dead_code)]
/// struct Person<'r> {
///     name: String,
///     age: Result<u16, &'r str>
/// }
/// ```
///
/// The `Err` value in this case is `&str` since `u16::from_form_value` returns
/// a `Result<u16, &str>`.
///
/// # Provided Implementations
///
/// Rocket implements `FromFormValue` for many standard library types. Their
/// behavior is documented here.
///
///   * **f32, f64, isize, i8, i16, i32, i64, usize, u8, u16, u32, u64**
///
///   **IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6, SocketAddr**
///
///     A value is validated successfully if the `from_str` method for the given
///     type returns successfully. Otherwise, the raw form value is returned as
///     the `Err` value.
///
///   * **bool**
///
///     A value is validated successfully as `true` if the the form value is
///     `"true"` or `"on"`, and as a `false` value if the form value is
///     `"false"`, `"off"`, or not present. In any other case, the raw form
///     value is returned in the `Err` value.
///
///   * **str**
///
///     _This implementation always returns successfully._
///
///     The raw, undecoded string is returned directly without modification.
///
///   * **String**
///
///     URL decodes the form value. If the decode is successful, the decoded
///     string is returned. Otherwise, an `Err` with the original form value is
///     returned.
///
///   * **Option&lt;T>** _where_ **T: FromFormValue**
///
///     _This implementation always returns successfully._
///
///     The form value is validated by `T`'s `FromFormValue` implementation. If
///     the validation succeeds, a `Some(validated_value)` is returned.
///     Otherwise, a `None` is returned.
///
///   * **Result&lt;T, T::Error>** _where_ **T: FromFormValue**
///
///     _This implementation always returns successfully._
///
///     The from value is validated by `T`'s `FromFormvalue` implementation. The
///     returned `Result` value is returned.
///
/// # Example
///
/// This trait is generally implemented to parse and validate form values. While
/// Rocket provides parsing and validation for many of the standard library
/// types such as `u16` and `String`, you can implement `FromFormValue` for a
/// custom type to get custom validation.
///
/// Imagine you'd like to verify that some user is over some age in a form. You
/// might define a new type and implement `FromFormValue` as follows:
///
/// ```rust
/// use rocket::request::FromFormValue;
///
/// struct AdultAge(usize);
///
/// impl<'v> FromFormValue<'v> for AdultAge {
///     type Error = &'v str;
///
///     fn from_form_value(form_value: &'v str) -> Result<AdultAge, &'v str> {
///         match usize::from_form_value(form_value) {
///             Ok(age) if age >= 21 => Ok(AdultAge(age)),
///             _ => Err(form_value),
///         }
///     }
/// }
/// ```
///
/// The type can then be used in a `FromForm` struct as follows:
///
/// ```rust,ignore
/// #[derive(FromForm)]
/// struct Person {
///     name: String,
///     age: AdultAge
/// }
/// ```
///
/// A form using the `Person` structure as its target will only parse and
/// validate if the `age` field contains a `usize` greater than `21`.
pub trait FromFormValue<'v>: Sized {
    /// The associated error which can be returned from parsing. It is a good
    /// idea to have the return type be or contain an `&'v str` so that the
    /// unparseable string can be examined after a bad parse.
    type Error;

    /// Parses an instance of `Self` from an HTTP form field value or returns an
    /// `Error` if one cannot be parsed.
    fn from_form_value(form_value: &'v str) -> Result<Self, Self::Error>;

    /// Returns a default value to be used when the form field does not exist.
    /// If this returns `None`, then the field is required. Otherwise, this
    /// should return `Some(default_value)`. The default implementation simply
    /// returns `None`.
    fn default() -> Option<Self> {
        None
    }
}

impl<'v> FromFormValue<'v> for &'v str {
    type Error = Error;

    // This just gives the raw string.
    fn from_form_value(v: &'v str) -> Result<Self, Self::Error> {
        Ok(v)
    }
}

impl<'v> FromFormValue<'v> for String {
    type Error = &'v str;

    // This actually parses the value according to the standard.
    fn from_form_value(v: &'v str) -> Result<Self, Self::Error> {
        let replaced = v.replace("+", " ");
        match URI::percent_decode(replaced.as_bytes()) {
            Err(_) => Err(v),
            Ok(string) => Ok(string.into_owned())
        }
    }
}

impl<'v> FromFormValue<'v> for bool {
    type Error = &'v str;

    fn from_form_value(v: &'v str) -> Result<Self, Self::Error> {
        match v {
            "on" | "true" => Ok(true),
            "off" | "false" => Ok(false),
            _ => Err(v),
        }
    }

    fn default() -> Option<bool> {
        Some(false)
    }
}

macro_rules! impl_with_fromstr {
    ($($T:ident),+) => ($(
        impl<'v> FromFormValue<'v> for $T {
            type Error = &'v str;
            fn from_form_value(v: &'v str) -> Result<Self, Self::Error> {
                $T::from_str(v).map_err(|_| v)
            }
        }
    )+)
}

impl_with_fromstr!(f32, f64, isize, i8, i16, i32, i64, usize, u8, u16, u32, u64,
    IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6, SocketAddr);

impl<'v, T: FromFormValue<'v>> FromFormValue<'v> for Option<T> {
    type Error = Error;

    fn from_form_value(v: &'v str) -> Result<Self, Self::Error> {
        match T::from_form_value(v) {
            Ok(v) => Ok(Some(v)),
            Err(_) => Ok(None),
        }
    }

    fn default() -> Option<Option<T>> {
        Some(None)
    }
}

// TODO: Add more useful implementations (range, regex, etc.).
impl<'v, T: FromFormValue<'v>> FromFormValue<'v> for Result<T, T::Error> {
    type Error = Error;

    fn from_form_value(v: &'v str) -> Result<Self, Self::Error> {
        match T::from_form_value(v) {
            ok@Ok(_) => Ok(ok),
            e@Err(_) => Ok(e),
        }
    }
}

