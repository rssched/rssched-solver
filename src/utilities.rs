use std::fmt;

/// a string of constant size that implements the Copy-trait.
/// it is used as Station (N = 4) (given by the abbreviation code)
/// and for the UnitId (N = 10)
#[derive(Hash,Eq,PartialEq,Copy,Clone)]
pub(crate) struct CopyStr<const N: usize> {
    code: [u8;N],
    len: usize
}

impl<const N: usize> CopyStr<N>{
    pub(crate) fn from(string: &str) -> Self {
        let raw = string.as_bytes();
        let len = raw.len();
        if len > N {
            panic!("station code string is too long");
        }

        let mut writable: [u8; N] = [0; N];
        let (writearea, _) = writable.split_at_mut(len);
        writearea.copy_from_slice(&raw);

        CopyStr{code: writable, len}
    }
}

impl<const N: usize> fmt::Display for CopyStr<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (s, _) = self.code.split_at(self.len);
        let as_str = std::str::from_utf8(s).expect("Invalid UTF8.");
        write!(f, "{}", as_str)
    }
}