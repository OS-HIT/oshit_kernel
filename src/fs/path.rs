//! Path parsing module using Finite State Machine
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

const MAX_FILE_NAME_LENGTH: usize = 255;

pub enum STATE {
        Start,
        FName,
        DirCur,
        DirParent,
}

#[derive(Clone, Copy, Debug)]
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

/// Convert errors to string for printing
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

/// Struct representing a path, no more worries for syntax errors in path
#[derive(Clone, PartialEq, Eq, Debug)]
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

        /// Create path representing root directory
        pub fn root() -> Path {
                return Path {
                        path: Vec::<String>::new(),
                        must_dir: true, 
                        is_abs: true,
                }
        }

        /// Get ride of ".." in path
        /// # Note 
        /// "." is already removed in parse_path()
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
                if !self.is_abs && self.path.len() == 0 {
                        res.push('.');
                        return res;
                } else if self.is_abs {
                        res.push('/')
                }
                for part in self.path.iter() {
                        res.push_str(part.as_str());
                        res.push('/');
                }
                if !self.must_dir {
                        res.pop();
                }
                res
        }

        /// pop the trailing file / diretory name
        pub fn pop(&mut self) -> Option<Path> {
                if self.path.len() != 0 {
                        let vt = vec![self.path.pop().unwrap()];
                        let p = Path {
                                path: vt,
                                must_dir: self.must_dir,
                                is_abs: false,
                        };
                        self.must_dir = true;
                        return Some(p);
                } else {
                        return None;
                }
        }

        /// add a file / dretory name at the end of the path
        pub fn push(&mut self, name: String, must_dir: bool) -> Result<(),()> {
                self.path.push(name);
                self.must_dir = must_dir;
                return Ok(());
        }

        /// merge the path "self" with another ("rel_path")
        /// # Note
        /// "rel_path" must be a relative path
        pub fn merge(&mut self, rel_path: Path) -> Result<(), &'static str> {
                if rel_path.is_abs == true {
                        return Err("Cannot merge a abs path");
                }
                let Path {path: mut path, must_dir: must_dir, ..} = rel_path;
                self.path.append(&mut path);
                self.must_dir = must_dir;
                return Ok(());
        }
}

/// FSM of parsing path
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
                                                return None;
                                        } else {
                                                if valid_fname_char(c) {
                                                        if self.buf.len() < 255 {
                                                                self.buf.push(c);
                                                                self.state = STATE::FName;
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

/// Construct a struct Path from string
pub fn parse_path(path: &str) -> Result<Path, PathFormatError> {
        // debug!("parse_path: path {}", path);
        let mut parser = PathParser::new();
        let chars = path.chars();
        for c in chars {
                if c == 0 as char {
                        break;
                }
                if let Some(error) = parser.read(c) {
                        return error;
                }
        }
        return parser.finish();
}


