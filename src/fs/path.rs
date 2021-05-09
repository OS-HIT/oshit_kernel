use alloc::string::String;
use alloc::vec::Vec;

pub enum STATE {
        Start,
        FNameInRoot,
        FName,
        FExt,
        DirCur,
        DirParent,
}

#[derive(Clone, Copy)]
pub enum PathFormatError {
        NotAbs,
        EmptyFileName,
        FileNameTooLong,
        InvalidCharInFileName,
        EmptyFileExt,
        InvalidCharInFileExt,
        EmptyPath,
        ReferingRootParent,
}

pub fn to_string(error: PathFormatError) -> &'static str {
        match error {
                PathFormatError::NotAbs => "Path should start with '/'",
                PathFormatError::EmptyFileName => "File name is empty",
                PathFormatError::FileNameTooLong => "File name longer than 8 bytes is not allowed",
                PathFormatError::InvalidCharInFileName => "Invalid char is found in file name",
                PathFormatError::EmptyFileExt => "File name with no extend should not have '.'",
                PathFormatError::InvalidCharInFileExt => "Invalid char is found in file extension",
                PathFormatError::EmptyPath => "Path is empty",
                PathFormatError::ReferingRootParent => "Path invalid because is refering parent of root",
                // _ => "unknown error",
        }
}

pub type Path = Vec<(String, String)>;

pub fn cat_name(name: &(String, String)) -> String {
        let mut result = String::new();
        result += &name.0;
        if name.1.len() != 0 {
                result += ".";
                result += &name.1;
        }
        return result;
}

pub fn get_name(name: &(String, String)) -> String {
        let mut result = String::new();
        result += &name.0;
        while result.len() < 8 {
                result.push(' ');
        }
        return result;
}

pub fn get_ext(name: &(String, String)) -> String {
        let mut result = String::new();
        result += &name.1;
        while result.len() < 3 {
                result.push(' ');
        }
        return result;
}

struct AbsPathCheck {
        state: STATE,
        name_buf: String,
        ext_buf: String,
        path: Path,
        result: Option<Result<(Path, bool), PathFormatError>>,
}

impl AbsPathCheck {
        fn new() -> AbsPathCheck {
                return AbsPathCheck {
                        state: STATE::Start,
                        name_buf: String::with_capacity(8),
                        ext_buf: String::with_capacity(3),
                        path: Vec::<(String, String)>::new(),
                        result: None,
                };
        }

        fn read(&mut self, c: char) -> Option<Result<(Path, bool), PathFormatError>> {
                if let Some(result) = self.result.as_ref() {
                        return Some(result.clone());
                }
                match self.state {
                        STATE::Start => if c != '/' {
                                return Some(Err(PathFormatError::NotAbs));
                        } else {
                                self.state = STATE::FNameInRoot;
                                return None;
                        },
                        STATE::FNameInRoot => {
                                if c == '/' {
                                        if self.name_buf.len() > 0 {
                                                self.path.push((self.name_buf.to_ascii_uppercase(), self.ext_buf.to_ascii_uppercase()));
                                                self.name_buf = String::with_capacity(8);
                                                self.ext_buf = String::with_capacity(3);
                                                return None;
                                        } else {
                                                self.result = Some(Err(PathFormatError::EmptyFileName));
                                                return Some(Err(PathFormatError::EmptyFileName));
                                        }
                                } else if c == '.' {
                                        if self.name_buf.len() > 0 {
                                                self.state = STATE::FExt;
                                                return None;
                                        } else {
                                                self.result = Some(Err(PathFormatError::InvalidCharInFileName));
                                                return Some(Err(PathFormatError::InvalidCharInFileName));
                                        }
                                } else {
                                        if c.is_alphanumeric() || c == '_' {
                                                if self.name_buf.len() < 8 {
                                                        self.name_buf.push(c);
                                                        return None;
                                                } else {
                                                        self.result = Some(Err(PathFormatError::FileNameTooLong));
                                                        return Some(Err(PathFormatError::FileNameTooLong));
                                                }
                                        } else {
                                                self.result = Some(Err(PathFormatError::InvalidCharInFileName));
                                                return Some(Err(PathFormatError::InvalidCharInFileName));
                                        }
                                }
                        },
                        STATE::FName => {
                                if c == '/' {
                                        if self.name_buf.len() > 0 {
                                                self.path.push((self.name_buf.to_ascii_uppercase(), self.ext_buf.to_ascii_uppercase()));
                                                self.name_buf = String::with_capacity(8);
                                                self.ext_buf = String::with_capacity(3);
                                                return None;
                                        } else {
                                                self.result = Some(Err(PathFormatError::EmptyFileName));
                                                return Some(Err(PathFormatError::EmptyFileName));
                                        }
                                } else if c == '.' {
                                        if self.name_buf.len() > 0 {
                                                self.state = STATE::FExt;
                                                return None;
                                        } else {
                                                self.state = STATE::DirCur;
                                                return None;
                                        }
                                } else {
                                        if c.is_alphanumeric() || c == '_' {
                                                if self.name_buf.len() < 8 {
                                                        self.name_buf.push(c);
                                                        return None;
                                                } else {
                                                        self.result = Some(Err(PathFormatError::FileNameTooLong));
                                                        return Some(Err(PathFormatError::FileNameTooLong));
                                                }
                                        } else {
                                                self.result = Some(Err(PathFormatError::InvalidCharInFileName));
                                                return Some(Err(PathFormatError::InvalidCharInFileName));
                                        }

                                }
                        },
                        STATE::FExt => {
                                if c == '/' {
                                        if self.ext_buf.len() > 0 {
                                                self.state = STATE::FName;
                                                self.path.push((self.name_buf.to_ascii_uppercase(), self.ext_buf.to_ascii_uppercase()));
                                                self.name_buf = String::with_capacity(8);
                                                self.ext_buf = String::with_capacity(3);
                                                return None;
                                        } else {
                                                self.result = Some(Err(PathFormatError::EmptyFileExt));
                                                return Some(Err(PathFormatError::EmptyFileExt));
                                        }
                                } else if c == '.' {
                                        self.result = Some(Err(PathFormatError::InvalidCharInFileExt));
                                        return Some(Err(PathFormatError::InvalidCharInFileExt));
                                } else {
                                        if c.is_alphanumeric() || c == '_' {
                                                if self.ext_buf.len() < 8 {
                                                        self.ext_buf.push(c);
                                                        return None;
                                                } else {
                                                        self.result = Some(Err(PathFormatError::FileNameTooLong));
                                                        return Some(Err(PathFormatError::FileNameTooLong));
                                                }
                                        } else {
                                                self.result = Some(Err(PathFormatError::InvalidCharInFileExt));
                                                return Some(Err(PathFormatError::InvalidCharInFileExt));
                                        }
                                }
                        },
                        STATE::DirCur => {
                                if c == '/' {
                                        self.state = STATE::FName;
                                        return None;
                                } else if c == '.' {
                                        self.state = STATE::DirParent;
                                        return None;
                                } else {
                                        self.result = Some(Err(PathFormatError::InvalidCharInFileName));
                                        return Some(Err(PathFormatError::InvalidCharInFileName));
                                }
                        },
                        STATE::DirParent => {
                                if c == '/' {
                                        self.state = STATE::FName;
                                        if self.path.len() > 0 {
                                                self.path.pop().unwrap();
                                                return None;
                                        } else {
                                                self.result = Some(Err(PathFormatError::ReferingRootParent));
                                                return Some(Err(PathFormatError::ReferingRootParent));
                                        }
                                } else {
                                        self.result = Some(Err(PathFormatError::InvalidCharInFileName));
                                        return Some(Err(PathFormatError::InvalidCharInFileName));
                                }
                        }
                }
        }

        fn finish(mut self) -> Result<(Path, bool), PathFormatError> {
                if let Some(error) = self.result {
                        return error;
                }
                match self.state {
                        STATE::Start => {
                                return Err(PathFormatError::EmptyPath);
                        },
                        STATE::FName => {
                                if self.name_buf.len() == 0 {
                                        return Ok((self.path, true));
                                } else {
                                        self.path.push((self.name_buf.to_ascii_uppercase(), self.ext_buf.to_ascii_uppercase()));
                                        return Ok((self.path, false));
                                }
                        },
                        STATE::FNameInRoot => {
                                if self.name_buf.len() == 0 {
                                        return Ok((self.path, true));
                                } else {
                                        self.path.push((self.name_buf.to_ascii_uppercase(), self.ext_buf.to_ascii_uppercase()));
                                        return Ok((self.path, false));
                                }
                        }
                        STATE::FExt => {
                                if self.ext_buf.len() == 0 {
                                        return Err(PathFormatError::InvalidCharInFileName);
                                } else {
                                        self.path.push((self.name_buf.to_ascii_uppercase(), self.ext_buf.to_ascii_uppercase()));
                                        return Ok((self.path, false));
                                }
                        }
                        STATE::DirCur => {
                                return Ok((self.path, true));
                        },
                        STATE::DirParent => {
                                if self.path.len() > 0 {
                                        self.path.pop().unwrap();
                                        return Ok((self.path, true));
                                } else {
                                        return Err(PathFormatError::ReferingRootParent);
                                }
                        }
                }
        }
}

pub fn parse_path(path: &str) -> Result<(Path, bool), PathFormatError> {
        let mut parser = AbsPathCheck::new();
        let chars = path.chars();
        for c in chars {
                if let Some(error) = parser.read(c) {
                        return error;
                }
        }
        return parser.finish();
}

