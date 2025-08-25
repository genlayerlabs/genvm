use wiggle::{GuestError, GuestMemory, GuestPtr};

pub fn read_string(memory: &GuestMemory<'_>, ptr: GuestPtr<str>) -> Result<String, GuestError> {
    Ok(memory.as_cow_str(ptr)?.into_owned())
}

pub fn align_slice(slice: &mut [u8], alignment: usize) -> &mut [u8] {
    let ptr = slice.as_ptr() as usize;
    let aligned = (ptr + alignment - 1) & !(alignment - 1);
    let offset = aligned - ptr;

    if offset >= slice.len() {
        return &mut [];
    }

    &mut slice[offset..]
}
