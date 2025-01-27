use serde_derive::Serialize;
#[derive(PartialEq, Clone, Copy, Serialize)]
#[repr(u8)]
pub enum TemplateId {
    Comparative = 0,
    NonComparative = 1,
    NonComparativeLeader = 2,
}

impl TryFrom<u8> for TemplateId {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, ()> {
        match value {
            0 => Ok(TemplateId::Comparative),
            1 => Ok(TemplateId::NonComparative),
            2 => Ok(TemplateId::NonComparativeLeader),
            _ => Err(()),
        }
    }
}
