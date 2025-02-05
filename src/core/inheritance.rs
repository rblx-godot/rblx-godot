use std::{
    any::{Any, TypeId},
    collections::HashMap,
    mem::transmute,
};

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct InheritanceCastError;

macro_rules! inheritance_cast_to {
    ($from: expr, $type: ty) => {{
        let p = $from;
        let ptrs = p.inheritance_table().index::<$type>();
        if ptrs.is_ok() {
            let (fn_call, fn_ptr) = ptrs.unwrap().0;
            Ok(fn_call(p, fn_ptr))
        } else {
            Err(crate::core::InheritanceCastError)
        }
    }};
}
macro_rules! inheritance_cast_to_mut {
    ($from: expr, $type: ty) => {{
        let p = $from;
        let ptrs = p.inheritance_table().index::<$type>();
        if ptrs.is_ok() {
            let (fn_call, fn_ptr) = ptrs.unwrap().1;
            Ok(fn_call(p, fn_ptr))
        } else {
            Err(crate::core::InheritanceCastError)
        }
    }};
}
macro_rules! inheritance_is_of_type {
    ($from: expr, $type: ty) => {
        $from.inheritance_table().has::<$type>()
    };
}

pub(crate) use inheritance_cast_to;
pub(crate) use inheritance_cast_to_mut;
pub(crate) use inheritance_is_of_type;

#[doc(hidden)]
pub struct InheritanceTable {
    // Function pointers. Their actual return values and arguments are not known inside the struct.
    v: HashMap<TypeId, ((fn(), fn()), (fn(), fn()))>,
}
#[derive(Default, Debug)]
pub struct InheritanceTableBuilder {
    v: HashMap<TypeId, ((fn(), fn()), (fn(), fn()))>,
}

impl InheritanceTable {
    #[doc(hidden)]
    pub fn index<'a, T: 'static + ?Sized>(
        &self,
    ) -> Result<
        (
            (fn(&'a dyn InheritanceBase, fn()) -> &'a T, fn()),
            (fn(&'a mut dyn InheritanceBase, fn()) -> &'a mut T, fn()),
        ),
        InheritanceCastError,
    > {
        let typeid: TypeId = TypeId::of::<T>();
        match self.v.get(&typeid) {
            None => Err(InheritanceCastError),
            Some(p) => {
                let new_ptrs: (
                    (fn(&'a dyn InheritanceBase, fn()) -> &'a T, fn()),
                    (fn(&'a mut dyn InheritanceBase, fn()) -> &'a mut T, fn()),
                ) = unsafe { transmute(*p) };
                Ok(new_ptrs)
            }
        }
    }
    #[doc(hidden)]
    pub fn has<T: 'static + ?Sized>(&self) -> bool {
        let typeid: TypeId = TypeId::of::<T>();
        self.v.contains_key(&typeid)
    }
}

pub trait InheritanceBase: Any {
    #[allow(private_interfaces)]
    fn inheritance_table(&self) -> InheritanceTable;
}

impl InheritanceTableBuilder {
    pub fn new() -> Self {
        Self { v: HashMap::new() }
    }
    fn fn_cast_ptr<'a, T: 'static + InheritanceBase, To: 'static + ?Sized>(
        obj: &'a dyn InheritanceBase,
        inherit: fn() -> (),
    ) -> &'a To {
        let ptr = unsafe { &*(obj as *const dyn InheritanceBase as *const T) };
        let inherit: fn(&'a T) -> &'a To = unsafe { transmute(inherit) };
        inherit(ptr)
    }
    fn fn_cast_ptr_mut<'a, T: 'static + InheritanceBase, To: 'static + ?Sized>(
        obj: &'a mut dyn InheritanceBase,
        inherit_mut: fn() -> (),
    ) -> &'a mut To {
        let ptr = unsafe { &mut *(obj as *mut dyn InheritanceBase as *mut T) };
        let inherit_mut: fn(&'a mut T) -> &'a mut To = unsafe { transmute(inherit_mut) };
        inherit_mut(ptr)
    }

    pub fn insert_type<'a, T: 'static + InheritanceBase, To: 'static + ?Sized>(
        mut self,
        inherit: fn(&'a T) -> &'a To,
        inherit_mut: fn(&'a mut T) -> &'a mut To,
    ) -> Self {
        self.v.insert(TypeId::of::<To>(), unsafe {
            transmute((
                (
                    Self::fn_cast_ptr::<T, To> as fn(&dyn InheritanceBase, fn()) -> &To,
                    inherit,
                ),
                (
                    Self::fn_cast_ptr_mut::<T, To> as fn(&mut dyn InheritanceBase, fn()) -> &mut To,
                    inherit_mut,
                ),
            ))
        });
        self
    }
    pub fn output(self) -> InheritanceTable {
        InheritanceTable { v: self.v }
    }
}
