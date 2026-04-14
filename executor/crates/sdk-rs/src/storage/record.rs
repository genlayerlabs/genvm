//! The `record!` macro for defining storage record types.

/// Define a storage record type.
///
/// # Examples
///
/// Without generics:
/// ```ignore
/// record!(Root {
///     code: Indirection<VLA<u8>>,
///     major: u8,
/// });
/// ```
///
/// With generics:
/// ```ignore
/// record!(Pair[T, V] {
///     first: T,
///     second: V,
/// });
/// ```
#[macro_export]
macro_rules! record {
    // Name with generics: record!(Foo[T, V] { ... })
    ($name:ident [$($gen:ident),* $(,)?] { $($field:ident : $ty:ty),* $(,)? }) => {
        $crate::record!(@impl [$($gen),*]($name) { $($field: $ty,)* });
    };

    // Name without generics: record!(Foo { ... })
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        $crate::record!(@impl []($name) { $($field: $ty,)* });
    };

    (@impl [$($gen:ident),*]($name:ident) { $($field:ident : $ty:ty,)* }) => {
        pub struct $name<$($gen,)*> {
            pub slot: $crate::storage::Slot,
            pub offset: u32,
            _marker: ::core::marker::PhantomData<($($gen,)*)>,
        }

        impl<$($gen,)*> ::core::clone::Clone for $name<$($gen,)*> {
            fn clone(&self) -> Self { *self }
        }
        impl<$($gen,)*> ::core::marker::Copy for $name<$($gen,)*> {}

        impl<$($gen: $crate::storage::StorageType,)*> $name<$($gen,)*> {
            $crate::record!(@methods []; $($field: $ty,)*);
        }

        impl<$($gen: $crate::storage::StorageType,)*> $crate::storage::StorageType for $name<$($gen,)*> {
            const SIZE: u32 = 0 $(+ <$ty as $crate::storage::StorageType>::SIZE)*;
            type Handle = Self;
            fn handle_at(slot: $crate::storage::Slot, offset: u32) -> Self::Handle {
                Self { slot, offset, _marker: ::core::marker::PhantomData }
            }
        }
    };

    (@methods [$($prev_ty:ty),*]; ) => {};
    (@methods [$($prev_ty:ty),*]; $field:ident: $ty:ty, $($rest:tt)*) => {
        pub fn $field(&self) -> <$ty as $crate::storage::StorageType>::Handle {
            <$ty as $crate::storage::StorageType>::handle_at(
                self.slot,
                self.offset $(+ <$prev_ty as $crate::storage::StorageType>::SIZE)*
            )
        }
        $crate::record!(@methods [$($prev_ty,)* $ty]; $($rest)*);
    };
}
