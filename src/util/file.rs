use std::os::unix::fs::FileTypeExt;

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub enum UnixFileType {
    File = 0,
    Fifo = 1,
    Dir = 2,
    CharDevice = 3,
    BlockDevice = 4,
    Symlink = 5,
    Socket = 6,
}

impl UnixFileType {
    pub fn from_i64(x: i64) -> Self {
        match x {
            0 => { Self::File },
            1 => { Self::Fifo },
            2 => { Self::Dir },
            3 => { Self::CharDevice },
            4 => { Self::BlockDevice },
            5 => { Self::Symlink },
            6 => { Self::Socket },
            // TODO: improve error handling
            _ => panic!("bug: invalid enum variant"),
        }
    }
    pub fn to_i64(self) -> i64 {
        self as i64
    }
    pub fn from_std(ft: &std::fs::FileType) -> UnixFileType {
        if      ft.is_file()         { Self::File }
        else if ft.is_dir()          { Self::Dir }
        else if ft.is_symlink()      { Self::Symlink }
        else if ft.is_socket()       { Self::Socket }
        else if ft.is_fifo()         { Self::Fifo }
        else if ft.is_block_device() { Self::BlockDevice }
        else if ft.is_char_device()  { Self::CharDevice }
        // TODO: improve error handling
        else                         { panic!("bug: unsupported file type") }
    }
}

pub fn get_file_type(path: &str) -> UnixFileType {
    use std::fs::File;
    UnixFileType::from_std(
        &File::open(path)
            .expect("FIXME: improve error handling")
            .metadata()
            .expect("FIXME: improve error handling")
            .file_type()
    )
}
