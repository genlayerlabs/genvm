use wiggle::{GuestError, GuestMemory, GuestPtr};

pub fn read_string<'a>(
    memory: &'a GuestMemory<'_>,
    ptr: GuestPtr<str>,
) -> Result<String, GuestError> {
    Ok(memory.as_cow_str(ptr)?.into_owned())
}
