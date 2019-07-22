use crate::{CppBox, CppDeletable, MutPtr, MutRef, Ptr, Ref, StaticUpcast};

/// Used to do value-to-value conversions while consuming the input value.
pub trait CastFrom<T>: Sized {
    /// Performs the conversion.
    unsafe fn cast_from(_: T) -> Self;
}

pub trait CastInto<T>: Sized {
    unsafe fn cast_into(self) -> T;
}

impl<T, U: CastFrom<T>> CastInto<U> for T {
    unsafe fn cast_into(self) -> U {
        U::cast_from(self)
    }
}

impl<T, U> CastFrom<MutPtr<U>> for Ptr<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: MutPtr<U>) -> Self {
        <U as StaticUpcast<T>>::static_upcast_mut(value).as_ptr()
    }
}

impl<T, U> CastFrom<MutRef<U>> for MutPtr<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: MutRef<U>) -> Self {
        StaticUpcast::static_upcast_mut(value.as_mut_ptr())
    }
}

impl<T, U> CastFrom<Ref<U>> for Ptr<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: Ref<U>) -> Self {
        StaticUpcast::static_upcast(value.as_ptr())
    }
}

impl<T, U> CastFrom<MutRef<U>> for Ref<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: MutRef<U>) -> Self {
        StaticUpcast::static_upcast(value.as_ptr())
            .as_ref()
            .expect("StaticUpcast returned null on Ref input")
    }
}

impl<T, U> CastFrom<MutRef<U>> for Ptr<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: MutRef<U>) -> Self {
        StaticUpcast::static_upcast(value.as_ptr())
    }
}

impl<'a, T, U: CppDeletable> CastFrom<&'a CppBox<U>> for Ptr<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: &'a CppBox<U>) -> Self {
        StaticUpcast::static_upcast(value.as_ptr())
    }
}

impl<'a, T, U: CppDeletable> CastFrom<&'a mut CppBox<U>> for MutPtr<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: &'a mut CppBox<U>) -> Self {
        StaticUpcast::static_upcast_mut(value.as_mut_ptr())
    }
}

impl<'a, T, U: CppDeletable> CastFrom<&'a CppBox<U>> for Ref<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: &'a CppBox<U>) -> Self {
        StaticUpcast::static_upcast(value.as_ptr())
            .as_ref()
            .expect("StaticUpcast returned null on CppBox input")
    }
}

impl<'a, T, U: CppDeletable> CastFrom<&'a mut CppBox<U>> for MutRef<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: &'a mut CppBox<U>) -> Self {
        StaticUpcast::static_upcast_mut(value.as_mut_ptr())
            .as_mut_ref()
            .expect("StaticUpcast returned null on CppBox input")
    }
}

impl<T, U> CastFrom<Ptr<U>> for Ptr<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: Ptr<U>) -> Self {
        StaticUpcast::static_upcast(value)
    }
}

impl<T, U> CastFrom<MutPtr<U>> for MutPtr<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: MutPtr<U>) -> Self {
        StaticUpcast::static_upcast_mut(value)
    }
}

impl<T, U> CastFrom<Ref<U>> for Ref<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: Ref<U>) -> Self {
        StaticUpcast::static_upcast(value.as_ptr())
            .as_ref()
            .expect("StaticUpcast returned null on Ref input")
    }
}

impl<T, U> CastFrom<MutRef<U>> for MutRef<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: MutRef<U>) -> Self {
        StaticUpcast::static_upcast_mut(value.as_mut_ptr())
            .as_mut_ref()
            .expect("StaticUpcast returned null on Ref input")
    }
}