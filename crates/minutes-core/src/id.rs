//! Raw and role-branded worklog identifiers.

use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// Raw identifier value shared by role-branded ID types.
///
/// Upstream time-tracking systems may expose IDs as either strings or integers;
/// this type preserves that distinction without forcing a conversion.
///
/// String and integer forms with the same digits are **not** equal
/// (`Id::from("42") != Id::from(42)`). Prefer [`ClientId`], [`ProjectId`], or
/// [`TaskId`] at API boundaries so roles cannot be mixed.
///
/// # Examples
///
/// ```
/// use minutes_core::Id;
///
/// assert_eq!(Id::from("acme").to_string(), "acme");
/// assert_eq!(Id::from(42).to_string(), "42");
/// assert_ne!(Id::from("42"), Id::from(42));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Id {
    /// Opaque string identifier (for example a UUID or slug).
    String(String),
    /// Numeric identifier.
    Integer(i64),
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(id) => write!(f, "{id}"),
            Self::Integer(id) => write!(f, "{id}"),
        }
    }
}

impl From<&str> for Id {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<String> for Id {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<i64> for Id {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

/// Marker role for [`ClientId`].
pub enum Client {}

/// Marker role for [`ProjectId`].
pub enum Project {}

/// Marker role for [`TaskId`].
pub enum Task {}

/// Role-branded identifier.
///
/// The `R` parameter is a zero-sized marker ([`Client`], [`Project`], or
/// [`Task`]) that makes otherwise identical IDs incompatible at compile time.
pub struct TypedId<R> {
    raw: Id,
    _role: PhantomData<R>,
}

impl<R> Clone for TypedId<R> {
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
            _role: PhantomData,
        }
    }
}

impl<R> std::fmt::Debug for TypedId<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TypedId").field(&self.raw).finish()
    }
}

impl<R> PartialEq for TypedId<R> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<R> Eq for TypedId<R> {}

impl<R> Hash for TypedId<R> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<R> std::fmt::Display for TypedId<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.raw.fmt(f)
    }
}

impl<R> AsRef<Id> for TypedId<R> {
    fn as_ref(&self) -> &Id {
        &self.raw
    }
}

impl<R> From<Id> for TypedId<R> {
    fn from(value: Id) -> Self {
        Self::new(value)
    }
}

impl<R> From<&str> for TypedId<R> {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl<R> From<String> for TypedId<R> {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl<R> From<i64> for TypedId<R> {
    fn from(value: i64) -> Self {
        Self::new(value)
    }
}

impl<R> TypedId<R> {
    /// Wraps a raw [`Id`] in this role-branded type.
    #[must_use]
    pub fn new(id: impl Into<Id>) -> Self {
        Self {
            raw: id.into(),
            _role: PhantomData,
        }
    }

    /// Borrows the underlying raw [`Id`].
    #[must_use]
    pub const fn as_inner(&self) -> &Id {
        &self.raw
    }

    /// Consumes the wrapper and returns the raw [`Id`].
    #[must_use]
    pub fn into_inner(self) -> Id {
        self.raw
    }

    /// Checks if the identifier is empty.
    ///
    /// An identifier is empty if it is a string and is empty, or if it is an integer and is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use minutes_core::ClientId;
    ///
    /// let id = ClientId::from("acme");
    /// assert!(!id.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        match self.raw {
            Id::String(ref s) => s.is_empty(),
            Id::Integer(ref i) => *i == 0,
        }
    }
}

/// Identifier of a client.
///
/// # Examples
///
/// ```
/// use minutes_core::ClientId;
///
/// let id = ClientId::from("acme");
/// assert_eq!(id.to_string(), "acme");
/// ```
///
/// Mixing roles fails to compile:
///
/// ```compile_fail
/// use minutes_core::{ClientId, Entry, ProjectId, TaskId, WorkInterval};
///
/// Entry::new(
///     TaskId::from(1),      // expected ClientId
///     ProjectId::from(2),
///     ClientId::from(3),    // expected TaskId
///     WorkInterval::default(),
/// );
/// ```
pub type ClientId = TypedId<Client>;

/// Identifier of a project under a client.
///
/// # Examples
///
/// ```
/// use minutes_core::ProjectId;
///
/// let id = ProjectId::from(7);
/// assert_eq!(id.to_string(), "7");
/// ```
pub type ProjectId = TypedId<Project>;

/// Identifier of a task within a project.
///
/// # Examples
///
/// ```
/// use minutes_core::TaskId;
///
/// let id = TaskId::from("api");
/// assert_eq!(id.to_string(), "api");
/// ```
pub type TaskId = TypedId<Task>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_display_string_and_integer() {
        assert_eq!(Id::from("acme").to_string(), "acme");
        assert_eq!(Id::from(42).to_string(), "42");
        assert_eq!(Id::from(-7_i64).to_string(), "-7");
    }

    #[test]
    fn id_equality_distinguishes_variants_with_same_digits() {
        assert_ne!(Id::from("42"), Id::from(42));
        assert_eq!(Id::from(42), Id::Integer(42));
        assert_eq!(Id::from("a"), Id::from(String::from("a")));
    }

    #[test]
    fn typed_ids_preserve_raw_representation() {
        let client = ClientId::from("acme");
        let project = ProjectId::from(7);
        let task = TaskId::new(Id::from("api"));

        assert_eq!(client.as_inner(), &Id::from("acme"));
        assert_eq!(project.into_inner(), Id::from(7));
        assert_eq!(task.as_ref(), &Id::from("api"));
    }

    #[test]
    fn typed_ids_share_raw_equality_across_roles() {
        let client = ClientId::from(42);
        let task = TaskId::from(42);
        assert_eq!(client.as_inner(), task.as_inner());
    }

    #[test]
    fn typed_ids_are_empty_when_raw_is_empty() {
        let client = ClientId::from("");
        let project = ProjectId::from(0);
        let task = TaskId::from("");
        assert!(client.is_empty());
        assert!(project.is_empty());
        assert!(task.is_empty());
    }
}
