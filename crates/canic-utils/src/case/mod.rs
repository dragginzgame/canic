use derive_more::Display;

mod constant;
mod snake;
mod title;

pub use snake::to_snake_case;

///
/// case
///
/// all case operations come through here as we have to include
/// multiple crates to get the desired behaviour
///

// Case
#[derive(Clone, Copy, Debug, Display)]
pub enum Case {
    Camel,
    Constant,
    Kebab,
    Lower,
    Sentence,
    Snake,
    Title,
    Upper,
    UpperCamel,
    UpperSnake,
    UpperKebab,
}

///
/// Casing
///

pub trait Casing<T: std::fmt::Display> {
    fn to_case(&self, case: Case) -> String;
    fn is_case(&self, case: Case) -> bool;
}

impl<T: std::fmt::Display> Casing<T> for T
where
    String: PartialEq<T>,
{
    fn to_case(&self, case: Case) -> String {
        use convert_case as cc;
        let s = &self.to_string();

        match case {
            Case::Lower => s.to_lowercase(),
            Case::Upper => s.to_uppercase(),
            Case::Title => title::to_title_case(s),
            Case::Snake => snake::to_snake_case(s),
            Case::UpperSnake => snake::to_snake_case(s).to_uppercase(),
            Case::Constant => constant::to_constant_case(s).to_uppercase(),
            Case::Camel => cc::Casing::to_case(s, cc::Case::Camel),
            Case::Kebab => cc::Casing::to_case(s, cc::Case::Kebab),
            Case::Sentence => cc::Casing::to_case(s, cc::Case::Sentence),
            Case::UpperCamel => cc::Casing::to_case(s, cc::Case::UpperCamel),
            Case::UpperKebab => cc::Casing::to_case(s, cc::Case::Kebab).to_uppercase(),
        }
    }

    fn is_case(&self, case: Case) -> bool {
        &self.to_case(case) == self
    }
}
