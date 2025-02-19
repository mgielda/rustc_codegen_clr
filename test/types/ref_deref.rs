#![feature(lang_items,adt_const_params,associated_type_defaults,core_intrinsics,start)]
#![allow(internal_features,incomplete_features,unused_variables,dead_code)]
#![no_std]
include!("../common.rs");
extern "C" {
    static __rust_no_alloc_shim_is_unstable:u8;
}

fn test_ref_deref(){
    unsafe{core::ptr::read_volatile(&__rust_no_alloc_shim_is_unstable)};
}
fn main(){
    black_box(test_ref_deref());
}

