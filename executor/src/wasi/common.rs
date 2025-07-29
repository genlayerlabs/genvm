use wiggle::{GuestError, GuestMemory, GuestPtr};

pub fn read_string(memory: &GuestMemory<'_>, ptr: GuestPtr<str>) -> Result<String, GuestError> {
    Ok(memory.as_cow_str(ptr)?.into_owned())
}
