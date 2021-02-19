use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6, SocketAddr};
use std::num::{
    NonZeroIsize, NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128,
    NonZeroUsize, NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128,
};

use time::{Date, PrimitiveDateTime};

use crate::data::Capped;
use crate::http::uncased::AsUncased;
use crate::form::prelude::*;

/// Trait to parse a typed value from a form value.
///
/// This trait is used by Rocket's code generation in two places:
///
///   1. Fields in structs deriving [`FromForm`](crate::request::FromForm) are
///      required to implement this trait.
///   2. Types of dynamic query parameters (`?<param>`) are required to
///      implement this trait.
///
/// # `FromForm` Fields
///
/// When deriving the `FromForm` trait, Rocket uses the `FromFormValue`
/// implementation of each field's type to validate the form input. To
/// illustrate, consider the following structure:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
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
/// # Dynamic Query Parameters
///
/// Types of dynamic query parameters are required to implement this trait. The
/// `FromFormValue` implementation is used to parse and validate each parameter
/// according to its target type:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # type Size = String;
/// #[get("/item?<id>&<size>")]
/// fn item(id: usize, size: Size) { /* ... */ }
/// # fn main() { }
/// ```
///
/// To generate values for `id` and `size`, Rocket calls
/// `usize::from_form_value()` and `Size::from_form_value()`, respectively.
///
/// # Validation Errors
///
/// It is sometimes desired to prevent a validation error from forwarding a
/// request to another route. The `FromFormValue` implementation for `Option<T>`
/// and `Result<T, T::Error>` make this possible. Their implementations always
/// return successfully, effectively "catching" the error.
///
/// For instance, if we wanted to know if a user entered an invalid `age` in the
/// form corresponding to the `Person` structure in the first example, we could
/// use the following structure:
///
/// ```rust
/// # use rocket::http::RawStr;
/// struct Person<'r> {
///     name: String,
///     age: Result<u16, &'r RawStr>
/// }
/// ```
///
/// The `Err` value in this case is `&RawStr` since `u16::from_form_value`
/// returns a `Result<u16, &RawStr>`.
///
/// # Provided Implementations
///
/// Rocket implements `FromFormValue` for many standard library types. Their
/// behavior is documented here.
///
///   *
///       * Primitive types: **f32, f64, isize, i8, i16, i32, i64, i128,
///         usize, u8, u16, u32, u64, u128**
///       * `IpAddr` and `SocketAddr` types: **IpAddr, Ipv4Addr, Ipv6Addr,
///         SocketAddrV4, SocketAddrV6, SocketAddr**
///       * `NonZero*` types: **NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64,
///         NonZeroI128, NonZeroIsize, NonZeroU8, NonZeroU16, NonZeroU32,
///         NonZeroU64, NonZeroU128, NonZeroUsize**
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
///   * **[`&RawStr`](RawStr)**
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
/// use rocket::http::RawStr;
///
/// struct AdultAge(usize);
///
/// impl<'v> FromFormValue<'v> for AdultAge {
///     type Error = &'v RawStr;
///
///     fn from_form_value(form_value: &'v RawStr) -> Result<AdultAge, &'v RawStr> {
///         match form_value.parse::<usize>() {
///             Ok(age) if age >= 21 => Ok(AdultAge(age)),
///             _ => Err(form_value),
///         }
///     }
/// }
/// ```
///
/// The type can then be used in a `FromForm` struct as follows:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # type AdultAge = usize;
/// #[derive(FromForm)]
/// struct Person {
///     name: String,
///     age: AdultAge
/// }
/// ```
///
/// A form using the `Person` structure as its target will only parse and
/// validate if the `age` field contains a `usize` greater than `21`.
// Ideally, we would have two traits instead of this trait with two fallible
// methods: `FromFormValue` and `FromFormData`. This would be especially nice
// for use with query values, where `FromFormData` would make no sense.
//
// However, blanket implementations of `FromForm` for these traits would result
// in duplicate implementations of `FromForm`; we need specialization to resolve
// this concern. Thus, for now, we keep this as one trait.
#[crate::async_trait]
pub trait FromFormField<'v>: Send + Sized {
    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        Err(field.unexpected())?
    }

    async fn from_data(field: DataField<'v, '_>) -> Result<'v, Self> {
        Err(field.unexpected())?
    }

    /// Returns a default value to be used when the form field does not exist or
    /// parsing otherwise fails.
    ///
    /// If this returns `None`, the field is required. Otherwise, this should
    /// return `Some(default_value)`. The default implementation returns `None`.
    fn default() -> Option<Self> { None }
}

#[doc(hidden)]
pub struct FromFieldContext<'v, T: FromFormField<'v>> {
    field_name: Option<NameView<'v>>,
    field_value: Option<&'v str>,
    opts: Options,
    value: Option<Result<'v, T>>,
    pushes: usize
}

impl<'v, T: FromFormField<'v>> FromFieldContext<'v, T> {
    fn can_push(&mut self) -> bool {
        self.pushes += 1;
        self.value.is_none()
    }

    fn push(&mut self, name: NameView<'v>, result: Result<'v, T>) {
        let is_unexpected = |e: &Errors<'_>| e.last().map_or(false, |e| {
            if let ErrorKind::Unexpected = e.kind { true } else { false }
        });

        self.field_name = Some(name);
        match result {
            Err(e) if !self.opts.strict && is_unexpected(&e) => { /* ok */ },
            result => self.value = Some(result),
        }
    }
}

#[crate::async_trait]
impl<'v, T: FromFormField<'v>> FromForm<'v> for T {
    type Context = FromFieldContext<'v, T>;

    fn init(opts: Options) -> Self::Context {
        FromFieldContext {
            opts,
            field_name: None,
            field_value: None,
            value: None,
            pushes: 0,
        }
    }

    fn push_value(ctxt: &mut Self::Context, field: ValueField<'v>) {
        if ctxt.can_push() {
            ctxt.field_value = Some(field.value);
            ctxt.push(field.name, Self::from_value(field))
        }
    }

    async fn push_data(ctxt: &mut FromFieldContext<'v, T>, field: DataField<'v, '_>) {
        if ctxt.can_push() {
            ctxt.push(field.name, Self::from_data(field).await);
        }
    }

    fn finalize(ctxt: Self::Context) -> Result<'v, Self> {
        let mut errors = match ctxt.value {
            Some(Ok(val)) if !ctxt.opts.strict || ctxt.pushes <= 1 => return Ok(val),
            Some(Err(e)) => e,
            Some(Ok(_)) => Errors::from(ErrorKind::Duplicate),
            None => match <T as FromFormField>::default() {
                Some(default) => return Ok(default),
                None => Errors::from(ErrorKind::Missing)
            }
        };

        if let Some(name) = ctxt.field_name {
            errors.set_name(name);
        }

        if let Some(value) = ctxt.field_value {
            errors.set_value(value);
        }

        Err(errors)
    }
}

impl<'v> FromFormField<'v> for &'v str {
    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        Ok(field.value)
    }
}

#[crate::async_trait]
impl<'v> FromFormField<'v> for Capped<String> {
    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        Ok(Capped::from(field.value.to_string()))
    }

    async fn from_data(f: DataField<'v, '_>) -> Result<'v, Self> {
        use crate::data::{Capped, Outcome, FromData};

        match <Capped<String> as FromData>::from_data(f.request, f.data).await {
            Outcome::Success(p) => Ok(p),
            Outcome::Failure((_, e)) => Err(e)?,
            Outcome::Forward(..) => {
                Err(Error::from(ErrorKind::Unexpected).with_entity(Entity::DataField))?
            }
        }
    }
}

impl<'v> FromFormField<'v> for ValueField<'v> {
    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        Ok(field)
    }
}

#[crate::async_trait]
impl<'v> FromFormField<'v> for crate::data::Data {
    async fn from_data(field: DataField<'v, '_>) -> Result<'v, Self> {
        Ok(field.data)
    }
}

impl_strict_from_form_field_from_capped!(String);

impl<'v> FromFormField<'v> for bool {
    fn default() -> Option<Self> { Some(false) }

    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        match field.value.as_uncased() {
            v if v == "on" || v == "yes" || v == "true" => Ok(true),
            v if v == "off" || v == "no" || v == "false" => Ok(false),
            // force a `ParseBoolError`
            _ => Ok("".parse()?),
        }
    }
}

macro_rules! impl_with_parse {
    ($($T:ident),+ $(,)?) => ($(
        impl<'v> FromFormField<'v> for $T {
            #[inline(always)]
            fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
                Ok(field.value.parse()?)
            }
        }
    )+)
}

impl_with_parse!(
    f32, f64,
    isize, i8, i16, i32, i64, i128,
    usize, u8, u16, u32, u64, u128,
    NonZeroIsize, NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128,
    NonZeroUsize, NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128,
    Ipv4Addr, IpAddr, Ipv6Addr, SocketAddrV4, SocketAddrV6, SocketAddr
);

impl<'v> FromFormField<'v> for Date {
    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        let date = Self::parse(field.value, "%F")
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

        Ok(date)
    }
}

// TODO: Doc that we don't support %FT%T.millisecond version.
impl<'v> FromFormField<'v> for PrimitiveDateTime {
    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        let dt = Self::parse(field.value, "%FT%R")
            .or_else(|_| Self::parse(field.value, "%FT%T"))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

        Ok(dt)
    }
}
