//! The `string_enum!` macro: one table per enum gives the variants, their stable snake_case
//! strings, `ALL`, `STRS`, `as_str()`, `Display`, `FromStr` and serde, so the wire strings have a
//! single source of truth.

/// Defines a fieldless enum whose variants serialise as fixed snake_case strings.
///
/// ```ignore
/// string_enum! {
///     /// Docs for the enum.
///     pub enum Setting: "setting" {
///         /// Docs for the variant.
///         Urban = "urban",
///     }
/// }
/// ```
///
/// The string after the colon names the kind of value in error messages ("unknown setting").
/// Declaration order is the order of `ALL` and of the derived `Ord`. A variant may carry
/// `#[deprecated]` (a retired id kept so old plans parse): the generated tables name every
/// variant, so they allow the lint.
macro_rules! string_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident : $what:literal {
            $(
                $(#[$vmeta:meta])*
                $variant:ident = $s:literal
            ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $vis enum $name {
            $(
                $(#[$vmeta])*
                $variant,
            )+
        }

        #[allow(deprecated)]
        impl $name {
            /// Every value, in declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$variant,)+];

            /// The stable string of every value, in the same order as `ALL`.
            pub const STRS: &'static [&'static str] = &[$($s,)+];

            /// The stable snake_case string used in JSON, TOML, the CLI and the web app.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $s,)+
                }
            }
        }

        #[allow(deprecated)]
        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        #[allow(deprecated)]
        impl ::core::str::FromStr for $name {
            type Err = $crate::ids::ParseIdError;

            fn from_str(s: &str) -> ::core::result::Result<Self, Self::Err> {
                match s {
                    $($s => Ok(Self::$variant),)+
                    _ => Err($crate::ids::ParseIdError::new($what, s, Self::STRS)),
                }
            }
        }

        #[allow(deprecated)]
        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(
                &self,
                serializer: S,
            ) -> ::core::result::Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        #[allow(deprecated)]
        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(
                deserializer: D,
            ) -> ::core::result::Result<Self, D::Error> {
                struct StrVisitor;

                impl ::serde::de::Visitor<'_> for StrVisitor {
                    type Value = $name;

                    fn expecting(
                        &self,
                        f: &mut ::core::fmt::Formatter<'_>,
                    ) -> ::core::fmt::Result {
                        write!(f, "a {} string", $what)
                    }

                    fn visit_str<E: ::serde::de::Error>(
                        self,
                        v: &str,
                    ) -> ::core::result::Result<$name, E> {
                        v.parse::<$name>()
                            .map_err(|_| E::unknown_variant(v, $name::STRS))
                    }
                }

                deserializer.deserialize_str(StrVisitor)
            }
        }
    };
}

/// Defines a string newtype for open-ended identifiers (citation ids, item ids): serde
/// transparent, ordered, hashable, borrowable as `&str`.
macro_rules! string_newtype {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident;
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, ::serde::Serialize, ::serde::Deserialize)]
        #[serde(transparent)]
        $vis struct $name(String);

        impl $name {
            /// Wraps a string. No check is made; see [`Self::is_well_formed`].
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            /// The identifier as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Unwraps into the owned string.
            pub fn into_string(self) -> String {
                self.0
            }

            /// True when the identifier follows the project convention: a lowercase ASCII letter,
            /// then lowercase letters, digits or underscores, at most 64 characters.
            pub fn is_well_formed(&self) -> bool {
                $crate::ids::is_well_formed_id(&self.0)
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl ::core::str::FromStr for $name {
            type Err = ::core::convert::Infallible;

            fn from_str(s: &str) -> ::core::result::Result<Self, Self::Err> {
                Ok(Self(s.to_owned()))
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_owned())
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl ::core::borrow::Borrow<str> for $name {
            fn borrow(&self) -> &str {
                &self.0
            }
        }

        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.0 == *other
            }
        }
    };
}
