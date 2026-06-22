use crate::{DptError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DptId {
    main: u16,
    sub: u16,
}

impl DptId {
    pub(crate) fn parse(input: &str) -> Result<Self> {
        let Some((main, sub)) = input.split_once('.') else {
            return Err(unsupported(input));
        };
        if main.is_empty() || sub.is_empty() {
            return Err(unsupported(input));
        }
        if !main.bytes().all(|byte| byte.is_ascii_digit())
            || !sub.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(unsupported(input));
        }

        let main = main.parse().map_err(|_| unsupported(input))?;
        let sub = sub.parse().map_err(|_| unsupported(input))?;

        Ok(Self { main, sub })
    }

    pub(crate) const fn main(self) -> u16 {
        self.main
    }

    pub(crate) const fn sub(self) -> u16 {
        self.sub
    }
}

fn unsupported(input: &str) -> DptError {
    DptError::UnsupportedDpt(input.to_owned())
}
