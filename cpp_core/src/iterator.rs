use crate::ops::{Begin, BeginMut, Decrement, End, EndMut, Increment, Indirection};
use crate::{CppBox, CppDeletable, MutPtr, MutRef, Ptr, Ref};
use std::os::raw::c_char;

/// `Iterator` and `DoubleEndedIterator` backed by C++ iterators.
///
/// This object is produced by `IntoIterator` implementations on  pointer types
/// (`&CppBox`, `&mut CppBox`, `Ptr`, `MutPtr`, `Ref`, `MutRef`). You can also use
/// `cpp_iter` function to construct it manually from two C++ iterator objects.
pub struct CppIterator<T1, T2>
where
    T1: CppDeletable,
    T2: CppDeletable,
{
    begin: CppBox<T1>,
    end: CppBox<T2>,
}

/// Constructs a Rust-style iterator from C++ iterators pointing to begin and end
/// of the collection.
///
/// ### Safety
///
/// `begin` and `end` must be valid. It's not possible to make any guarantees about safety, since
/// `CppIterator` will call arbitrary C++ library code when used.
pub unsafe fn cpp_iter<T1, T2>(begin: CppBox<T1>, end: CppBox<T2>) -> CppIterator<T1, T2>
where
    T1: CppDeletable,
    T2: CppDeletable,
{
    CppIterator { begin, end }
}

impl<T1, T2> Iterator for CppIterator<T1, T2>
where
    T1: CppDeletable + PartialEq<Ref<T2>> + Indirection + Increment,
    T2: CppDeletable,
{
    type Item = <T1 as Indirection>::Output;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if self.begin == self.end.as_ref() {
                None
            } else {
                let inner = &mut *self.begin.as_mut_raw_ptr();
                let value = inner.indirection();
                let inner = &mut *self.begin.as_mut_raw_ptr();
                inner.inc();
                Some(value)
            }
        }
    }
}

impl<T1, T2> DoubleEndedIterator for CppIterator<T1, T2>
where
    T1: CppDeletable + PartialEq<Ref<T2>> + Indirection + Increment,
    T2: CppDeletable + Decrement + Indirection<Output = <T1 as Indirection>::Output>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        unsafe {
            if self.begin == self.end.as_ref() {
                None
            } else {
                let inner = &mut *self.end.as_mut_raw_ptr();
                inner.dec();
                let inner = &mut *self.end.as_mut_raw_ptr();
                let value = inner.indirection();
                Some(value)
            }
        }
    }
}

impl<T, T1, T2> IntoIterator for Ptr<T>
where
    T: Begin<Output = CppBox<T1>> + End<Output = CppBox<T2>>,
    T1: CppDeletable + PartialEq<Ref<T2>> + Increment + Indirection,
    T2: CppDeletable,
{
    type Item = <T1 as Indirection>::Output;
    type IntoIter = CppIterator<T1, T2>;

    fn into_iter(self) -> Self::IntoIter {
        unsafe { cpp_iter(self.begin(), self.end()) }
    }
}

impl<T, T1, T2> IntoIterator for MutPtr<T>
where
    T: BeginMut<Output = CppBox<T1>> + EndMut<Output = CppBox<T2>>,
    T1: CppDeletable + PartialEq<Ref<T2>> + Indirection + Increment,
    T2: CppDeletable,
{
    type Item = <T1 as Indirection>::Output;
    type IntoIter = CppIterator<T1, T2>;

    fn into_iter(mut self) -> Self::IntoIter {
        unsafe { cpp_iter(self.begin_mut(), self.end_mut()) }
    }
}

impl<T, T1, T2> IntoIterator for Ref<T>
where
    T: Begin<Output = CppBox<T1>> + End<Output = CppBox<T2>>,
    T1: CppDeletable + PartialEq<Ref<T2>> + Indirection + Increment,
    T2: CppDeletable,
{
    type Item = <T1 as Indirection>::Output;
    type IntoIter = CppIterator<T1, T2>;

    fn into_iter(self) -> Self::IntoIter {
        unsafe { cpp_iter(self.begin(), self.end()) }
    }
}

impl<T, T1, T2> IntoIterator for MutRef<T>
where
    T: BeginMut<Output = CppBox<T1>> + EndMut<Output = CppBox<T2>>,
    T1: CppDeletable + PartialEq<Ref<T2>> + Indirection + Increment,
    T2: CppDeletable,
{
    type Item = <T1 as Indirection>::Output;
    type IntoIter = CppIterator<T1, T2>;

    fn into_iter(mut self) -> Self::IntoIter {
        unsafe { cpp_iter(self.begin_mut(), self.end_mut()) }
    }
}

impl<'a, T, T1, T2> IntoIterator for &'a CppBox<T>
where
    T: CppDeletable + Begin<Output = CppBox<T1>> + End<Output = CppBox<T2>>,
    T1: CppDeletable + PartialEq<Ref<T2>> + Indirection + Increment,
    T2: CppDeletable,
{
    type Item = <T1 as Indirection>::Output;
    type IntoIter = CppIterator<T1, T2>;

    fn into_iter(self) -> Self::IntoIter {
        unsafe { cpp_iter(self.begin(), self.end()) }
    }
}

impl<'a, T, T1, T2> IntoIterator for &'a mut CppBox<T>
where
    T: CppDeletable + BeginMut<Output = CppBox<T1>> + EndMut<Output = CppBox<T2>>,
    T1: CppDeletable + PartialEq<Ref<T2>> + Indirection + Increment,
    T2: CppDeletable,
{
    type Item = <T1 as Indirection>::Output;
    type IntoIter = CppIterator<T1, T2>;

    fn into_iter(self) -> Self::IntoIter {
        unsafe { cpp_iter(self.begin_mut(), self.end_mut()) }
    }
}

pub trait SliceAsBeginEnd {
    type Item;
    fn begin_ptr(&self) -> Ptr<Self::Item>;
    fn end_ptr(&self) -> Ptr<Self::Item>;
}

impl<'a, T> SliceAsBeginEnd for &'a [T] {
    type Item = T;
    fn begin_ptr(&self) -> Ptr<T> {
        unsafe { Ptr::from_raw(self.as_ptr()) }
    }
    fn end_ptr(&self) -> Ptr<T> {
        unsafe { Ptr::from_raw(self.as_ptr().add(self.len())) }
    }
}

impl<'a> SliceAsBeginEnd for &'a str {
    type Item = c_char;
    fn begin_ptr(&self) -> Ptr<c_char> {
        unsafe { Ptr::from_raw(self.as_ptr() as *const c_char) }
    }
    fn end_ptr(&self) -> Ptr<c_char> {
        unsafe { Ptr::from_raw(self.as_ptr().add(self.len()) as *const c_char) }
    }
}
