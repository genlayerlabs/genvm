use serde_derive::Serialize;
#[derive(PartialEq, Clone, Copy, Serialize)]
#[repr(u8)]
pub enum TemplateId {
    Comparative = 0,
    NonComparativeValidator = 1,
    NonComparativeLeader = 2,
}

#[allow(dead_code)]
impl TemplateId {
    pub fn str_snake_case(self) -> &'static str {
        match self {
            TemplateId::Comparative => "comparative",
            TemplateId::NonComparativeValidator => "non_comparative_validator",
            TemplateId::NonComparativeLeader => "non_comparative_leader",
        }
    }
}

impl TryFrom<u8> for TemplateId {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, ()> {
        match value {
            0 => Ok(TemplateId::Comparative),
            1 => Ok(TemplateId::NonComparativeValidator),
            2 => Ok(TemplateId::NonComparativeLeader),
            _ => Err(()),
        }
    }
}
