pub mod interfaces;


#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
}
