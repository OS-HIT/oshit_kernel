use alloc::string::String;
use alloc::vec::Vec;

const MAX_FILE_NAME_LENGTH: usize = 255;

pub enum STATE {
        Start,
        FName,
        DirCur,
        DirParent,
}

#[derive(Clone, Copy)]
pub enum PathFormatError {
        NotAbs,
        NotRel,
        EmptyFileName,
        FileNameTooLong,
        InvalidCharInFileName,
        InvalidCharInFileExt,
        EmptyPath,
        ReferingRootParent,
        Unknown,
}

pub fn to_string(error: PathFormatError) -> &'static str {
        match error {
                PathFormatError::NotAbs => "Path should start with '/'",
                PathFormatError::NotRel => "Processing non-relative path with relative parser",
                PathFormatError::EmptyFileName => "File name is empty",
                PathFormatError::FileNameTooLong => "File name longer than 255 bytes is not allowed",
                PathFormatError::InvalidCharInFileName => "Invalid char is found in file name",
                PathFormatError::InvalidCharInFileExt => "Invalid char is found in file extension",
                PathFormatError::EmptyPath => "Path is empty",
                PathFormatError::ReferingRootParent => "Path invalid because is refering parent of root",
                PathFormatError::Unknown => "unknown error",
                // _ => "unknown error",
        }
}

#[derive(Clone)]
pub struct Path {
        pub path: Vec::<String>,
        pub must_dir: bool,
        pub is_abs: bool,
}

impl Path {
        fn new() -> Path {
                return Path {
                        path: Vec::<String>::new(),
                        must_dir: false,
                        is_abs: true,
                };
        }

        pub fn purge(&mut self) -> Result<(), PathFormatError> {
                let mut idx = 0;
                while idx < self.path.len() {
                        if self.path[idx].eq("..") {
                                if idx == 0 && self.is_abs {
                                        return Err(PathFormatError::ReferingRootParent);
                                } else {
                                        self.path.remove(idx);
                                        self.path.remove(idx -1);
                                        idx -= 1;
                                }
                        } else {
                                idx += 1;
                        }
                }
                return Ok(());
        }

        pub fn to_string(&self) -> String {
                let mut res = String::new();
                if !self.is_abs {
                        res.push('.');
                }
                for part in self.path.iter() {
                        res.push('/');
                        res.push_str(part.as_str());
                }
                res
        }
}

struct PathParser {
        state: STATE,
        buf: String,
        path: Path,
        result: Option<Result<Path, PathFormatError>>,
}

fn valid_fname_char(_c: char) -> bool {
        return true;
}

impl PathParser {
        fn new() -> PathParser {
                return PathParser {
                        state: STATE::Start,
                        buf: String::with_capacity(MAX_FILE_NAME_LENGTH),
                        path: Path::new(),
                        result: None,
                };
        }

        fn read(&mut self, c: char) -> Option<Result<Path, PathFormatError>> {
                if let Some(result) = self.result.as_ref() {
                        return Some(result.clone());
                }
                match self.state {
                        STATE::Start => {
                                if c == '/' {
                                        self.state = STATE::FName;
                                        return None;
                                } else {
                                        self.path.is_abs = false;
                                        if c == '.' {
                                                self.buf.push('.');
                                                self.state = STATE::DirCur;
                                        } else {
                                                if valid_fname_char(c) {
                                                        if self.buf.len() < 255 {
                                                                self.buf.push(c);
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
                                        return None;
                                }
                        },
                        STATE::FName => {
                                if c == '/' {
                                        if self.buf.len() > 0 {
                                                self.path.path.push(self.buf.clone());
                                                self.buf = String::with_capacity(MAX_FILE_NAME_LENGTH);
                                                return None;
                                        } else {
                                                self.result = Some(Err(PathFormatError::EmptyFileName));
                                                return Some(Err(PathFormatError::EmptyFileName));
                                        }
                                } else if c == '.' && self.buf.len() == 0 {
                                        self.state = STATE::DirCur;
                                        return None;
                                } else {
                                        if valid_fname_char(c) {
                                                if self.buf.len() < 255 {
                                                        self.buf.push(c);
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
                        STATE::DirCur => {
                                if c == '/' {
                                        self.state = STATE::FName;
                                        return None;
                                } else if c == '.' {
                                        self.state = STATE::DirParent;
                                        return None;
                                } else if valid_fname_char(c) {
                                        self.buf.push(c);
                                        self.state = STATE::FName;
                                        return None;
                                } else {
                                        self.result = Some(Err(PathFormatError::InvalidCharInFileName));
                                        return Some(Err(PathFormatError::InvalidCharInFileName));
                                }
                        },
                        STATE::DirParent => {
                                if c == '/' {
                                        self.state = STATE::FName;
                                        self.path.path.push(String::from(".."));
                                        self.buf.pop();
                                        return None;
                                } else if valid_fname_char(c) {
                                        self.buf.push(c);
                                        self.state = STATE::FName;
                                        return None;
                                } else {
                                        self.result = Some(Err(PathFormatError::InvalidCharInFileName));
                                        return Some(Err(PathFormatError::InvalidCharInFileName));
                                }
                        }
                }
        }

        fn finish(mut self) -> Result<Path, PathFormatError> {
                if let Some(error) = self.result {
                        return error;
                }
                match self.state {
                        STATE::Start => {
                                return Err(PathFormatError::EmptyPath);
                        },
                        STATE::FName => {
                                if self.buf.len() == 0 {
                                        self.path.must_dir = true;
                                        return Ok(self.path);
                                } else {
                                        self.path.path.push(self.buf);
                                        return Ok(self.path);
                                }
                        },
                        STATE::DirCur => {
                                self.path.must_dir = true;
                                return Ok(self.path);
                        },
                        STATE::DirParent => {
                                self.path.path.push(String::from(".."));
                                self.path.must_dir = true;
                                return Ok(self.path);
                        }
                }
        }
}

pub fn parse_path(path: &str) -> Result<Path, PathFormatError> {
        let mut parser = PathParser::new();
        let chars = path.chars();
        for c in chars {
                if let Some(error) = parser.read(c) {
                        return error;
                }
        }
        return parser.finish();
}


