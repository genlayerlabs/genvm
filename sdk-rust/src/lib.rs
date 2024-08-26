mod generated {
    include! {
        concat!(env!("OUT_DIR"), "/generated.rs")
    }
}

#[allow(unused_imports)]
pub use generated::*;
