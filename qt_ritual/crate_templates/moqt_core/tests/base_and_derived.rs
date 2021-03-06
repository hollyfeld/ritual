use cpp_core::{CppBox, MutRef, NullPtr, Ref};
use moqt_core::{
    AbstractBaseClass1, BaseClass1, DerivedClass1, DerivedClass2, DerivedClass3, DerivedSubClass1,
};

#[test]
fn casts() {
    unsafe {
        let mut derived: CppBox<DerivedClass1> = DerivedClass1::new();
        assert_eq!(derived.base_function(), 1);

        let mut base: MutRef<BaseClass1> = derived.static_upcast_mut();
        assert_eq!(base.base_function(), 2);

        let base_const: Ref<BaseClass1> = derived.static_upcast();
        assert_eq!(base_const.base_const_function(), 2);

        let mut derived1: MutRef<DerivedClass1> = base.dynamic_cast_mut().unwrap();
        assert_eq!(derived1.base_function(), 3);

        let derived1: Ref<DerivedClass1> = base.dynamic_cast().unwrap();
        assert_eq!(derived1.base_const_function(), 3);

        let derived2: Option<MutRef<DerivedClass2>> = base.dynamic_cast_mut();
        assert!(derived2.is_none());

        let mut derived1: MutRef<DerivedClass1> = base.static_downcast_mut();
        assert_eq!(derived1.base_function(), 4);

        let derived1: Ref<DerivedClass1> = base.static_downcast();
        assert_eq!(derived1.base_const_function(), 4);
    }
}

#[test]
fn indirect_casts() {
    unsafe {
        let mut derived: CppBox<DerivedSubClass1> = DerivedSubClass1::new();
        assert_eq!(derived.base_function(), 1);

        let mut base: MutRef<BaseClass1> = derived.static_upcast_mut();
        assert_eq!(base.base_function(), 2);
    }

    unsafe {
        let mut derived: CppBox<DerivedSubClass1> = DerivedSubClass1::new();
        let mut base: CppBox<BaseClass1> = BaseClass1::new();
        // TODO: `set_parent` should accept without `cast_into`
        base.set_parent(derived.as_mut_ptr());
        base.set_parent(&mut derived);
        base.set_parent(NullPtr);
    }
}

#[test]
fn virtual_functions() {
    unsafe {
        let derived: CppBox<DerivedClass1> = DerivedClass1::new();
        assert_eq!(derived.virtual_function(), 43);

        let base: Ref<BaseClass1> = derived.static_upcast();
        assert_eq!(base.virtual_function(), 43);

        let base: CppBox<BaseClass1> = BaseClass1::new();
        assert_eq!(base.virtual_function(), 42);

        let derived: CppBox<DerivedClass2> = DerivedClass2::new();
        assert_eq!(derived.virtual_function(), 44);

        let base: Ref<BaseClass1> = derived.static_upcast();
        assert_eq!(base.virtual_function(), 44);
    }
}

#[test]
fn pure_virtual_functions() {
    unsafe {
        let mut derived: CppBox<DerivedClass3> = DerivedClass3::new();
        assert_eq!(*derived.virtual_function(), 45);

        let mut base: MutRef<AbstractBaseClass1> = derived.static_upcast_mut();
        assert_eq!(*base.virtual_function(), 45);
    }
}
