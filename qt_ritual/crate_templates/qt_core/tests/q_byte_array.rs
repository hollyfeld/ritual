use cpp_core::vector_ops::{VectorAsMutSlice, VectorAsSlice};

#[test]
fn qrect() {
    unsafe {
        let mut array = qt_core::QByteArray::new();

        assert!(array.vector_as_slice().is_empty());
        array.append_char(42);
        array.append_char(46);

        assert_eq!(array.vector_as_slice(), &[42, 46]);

        let slice = array.vector_as_mut_slice();
        slice[1] = 47;
        drop(slice);

        assert_eq!(array.index_int(0), 42);
        assert_eq!(array.index_int(1), 47);
    }
}
