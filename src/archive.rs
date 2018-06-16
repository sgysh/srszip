use byteorder::{LittleEndian, WriteBytesExt};
use crc32;
use std::convert;
use std::io;
use std::io::prelude::*;

const LOCAL_FILE_HEADER_SIGNATURE: u32 = 0x04034b50;
const CENTRAL_FILE_HEADER_SIGNATURE: u32 = 0x02014b50;
const END_OF_CENTRAL_DIR_SIGNATURE: u32 = 0x06054b50;

pub type ZipResult<T> = Result<T, ZipError>;

#[derive(Debug)]
pub enum ZipError {
    Io(io::Error),
}

impl convert::From<io::Error> for ZipError {
    fn from(err: io::Error) -> ZipError {
        ZipError::Io(err)
    }
}

struct ZipFileData {
    system: u16,
    version_made_by: u8,
    crc32: u32,
    compressed_size: u64,
    uncompressed_size: u64,
    file_name: String,
    header_start: u64,
    external_attributes: u32,
}

pub struct ZipWriter<W: Write + io::Seek> {
    writee: W,
    files: Vec<ZipFileData>,
}

impl<W: Write + io::Seek> Drop for ZipWriter<W> {
    fn drop(&mut self) -> () {
        match self.write_end_of_central_directory_record() {
            Ok(_) => {}
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
}

impl<W: Write + io::Seek> ZipWriter<W> {
    pub fn new(writee: W) -> ZipWriter<W> {
        ZipWriter {
            writee: writee,
            files: Vec::new(),
        }
    }

    pub fn add_directory<S>(&mut self, name: S) -> ZipResult<()>
    where
        S: Into<String>,
    {
        let header_start = try!(self.writee.seek(io::SeekFrom::Current(0)));
        let permissions = 0o40775;

        let file = ZipFileData {
            // 3: Unix
            system: 3,
            version_made_by: 20,
            crc32: 0,
            compressed_size: 0,
            uncompressed_size: 0,
            file_name: name.into(),
            header_start: header_start,
            external_attributes: permissions << 16,
        };

        try!(self.write_local_file_header(&file));

        self.files.push(file);

        Ok(())
    }

    pub fn add_file<S>(&mut self, name: S, data: &[u8]) -> ZipResult<()>
    where
        S: Into<String>,
    {
        let header_start = try!(self.writee.seek(io::SeekFrom::Current(0)));
        let permissions = 0o100664;

        let mut file = ZipFileData {
            // 3: Unix
            system: 3,
            version_made_by: 20,
            crc32: 0,
            compressed_size: 0,
            uncompressed_size: 0,
            file_name: name.into(),
            header_start: header_start,
            external_attributes: permissions << 16,
        };

        try!(self.write_local_file_header(&file));

        let file_start = try!(self.writee.seek(io::SeekFrom::Current(0)));
        try!(self.writee.write_all(data));
        let file_end = try!(self.writee.seek(io::SeekFrom::Current(0)));

        file.crc32 = crc32::calc(&data);
        file.compressed_size = file_end - file_start;
        file.uncompressed_size = file.compressed_size;

        const CRC32_OFFSET: u64 = 14;
        try!(
            self.writee
                .seek(io::SeekFrom::Start(file.header_start + CRC32_OFFSET))
        );
        try!(self.writee.write_u32::<LittleEndian>(file.crc32));
        try!(
            self.writee
                .write_u32::<LittleEndian>(file.compressed_size as u32)
        );
        try!(
            self.writee
                .write_u32::<LittleEndian>(file.uncompressed_size as u32)
        );

        try!(self.writee.seek(io::SeekFrom::Start(file_end)));

        self.files.push(file);

        Ok(())
    }

    fn write_local_file_header(&mut self, file: &ZipFileData) -> ZipResult<()> {
        // local file header signature
        try!(
            self.writee
                .write_u32::<LittleEndian>(LOCAL_FILE_HEADER_SIGNATURE)
        );
        // version needed to extract
        let version_made_by = file.system << 8 | (file.version_made_by as u16);
        try!(self.writee.write_u16::<LittleEndian>(version_made_by));
        // general purpose bit flag
        let flag = if !file.file_name.is_ascii() {
            1u16 << 11
        } else {
            0
        };
        try!(self.writee.write_u16::<LittleEndian>(flag));
        // compression method
        // 0: stored
        try!(self.writee.write_u16::<LittleEndian>(0));
        // TODO
        // last mod file time and last mod file date
        try!(self.writee.write_u16::<LittleEndian>(0));
        try!(self.writee.write_u16::<LittleEndian>(0));
        // crc-32
        try!(self.writee.write_u32::<LittleEndian>(file.crc32));
        // compressed size
        try!(
            self.writee
                .write_u32::<LittleEndian>(file.compressed_size as u32)
        );
        // uncompressed size
        try!(
            self.writee
                .write_u32::<LittleEndian>(file.uncompressed_size as u32)
        );
        // file name length
        try!(
            self.writee
                .write_u16::<LittleEndian>(file.file_name.as_bytes().len() as u16)
        );
        // extra field length
        try!(self.writee.write_u16::<LittleEndian>(0));
        // file name
        try!(self.writee.write_all(file.file_name.as_bytes()));
        // extra field
        // <none>

        Ok(())
    }

    fn write_central_directory_headers(&mut self) -> ZipResult<()> {
        for file in self.files.iter() {
            // central file header signature
            try!(
                self.writee
                    .write_u32::<LittleEndian>(CENTRAL_FILE_HEADER_SIGNATURE)
            );
            // version made by
            let version_made_by = file.system << 8 | (file.version_made_by as u16);
            try!(self.writee.write_u16::<LittleEndian>(version_made_by));
            // version needed to extract
            try!(self.writee.write_u16::<LittleEndian>(20));
            // general puprose bit flag
            let flag = if !file.file_name.is_ascii() {
                1u16 << 11
            } else {
                0
            };
            try!(self.writee.write_u16::<LittleEndian>(flag));
            // compression method
            // 0: stored
            try!(self.writee.write_u16::<LittleEndian>(0));
            // TODO
            // last mod file time and last mod file date
            try!(self.writee.write_u16::<LittleEndian>(0));
            try!(self.writee.write_u16::<LittleEndian>(0));
            // crc-32
            try!(self.writee.write_u32::<LittleEndian>(file.crc32));
            // compressed size
            try!(
                self.writee
                    .write_u32::<LittleEndian>(file.compressed_size as u32)
            );
            // uncompressed size
            try!(
                self.writee
                    .write_u32::<LittleEndian>(file.uncompressed_size as u32)
            );
            // file name length
            try!(
                self.writee
                    .write_u16::<LittleEndian>(file.file_name.as_bytes().len() as u16)
            );
            // extra field length
            try!(self.writee.write_u16::<LittleEndian>(0));
            // file comment length
            try!(self.writee.write_u16::<LittleEndian>(0));
            // disk number start
            try!(self.writee.write_u16::<LittleEndian>(0));
            // internal file attribytes
            try!(self.writee.write_u16::<LittleEndian>(0));
            // external file attributes
            try!(
                self.writee
                    .write_u32::<LittleEndian>(file.external_attributes)
            );
            // relative offset of local header
            try!(
                self.writee
                    .write_u32::<LittleEndian>(file.header_start as u32)
            );
            // file name
            try!(self.writee.write_all(file.file_name.as_bytes()));
            // extra field
            // <none>
            // file comment
            // <none>
        }

        Ok(())
    }

    fn write_end_of_central_directory_record(&mut self) -> ZipResult<()> {
        let central_start = try!(self.writee.seek(io::SeekFrom::Current(0)));
        try!(self.write_central_directory_headers());
        let central_size = try!(self.writee.seek(io::SeekFrom::Current(0))) - central_start;

        let comment = b"srszip".to_vec();

        // end of central dir signature
        try!(
            self.writee
                .write_u32::<LittleEndian>(END_OF_CENTRAL_DIR_SIGNATURE)
        );
        // number of this disk
        try!(self.writee.write_u16::<LittleEndian>(0));
        // number of the disk with the start of the central directory
        try!(self.writee.write_u16::<LittleEndian>(0));
        // total number of entries in the central directory on this disk
        try!(
            self.writee
                .write_u16::<LittleEndian>(self.files.len() as u16)
        );
        // total number of entries in the central directory
        try!(
            self.writee
                .write_u16::<LittleEndian>(self.files.len() as u16)
        );
        // size of the central directory
        try!(self.writee.write_u32::<LittleEndian>(central_size as u32));
        // offset of start of central directory with respect to the starting disk number
        try!(self.writee.write_u32::<LittleEndian>(central_start as u32));
        // .ZIP file comment length
        try!(self.writee.write_u16::<LittleEndian>(comment.len() as u16));
        // .ZIP file comment
        try!(self.writee.write_all(&comment));

        Ok(())
    }
}

#[test]
fn add_dir() {
    let mut zip = ZipWriter::new(io::Cursor::new(Vec::new()));

    let r = zip.add_directory("test/");
    assert_eq!(r.is_ok(), true);

    assert_eq!(zip.writee.get_ref().len(), 35);
}

#[test]
fn add_file() {
    let mut zip = ZipWriter::new(io::Cursor::new(Vec::new()));

    let r = zip.add_file("test", &[b'f', b'o', b'o']);
    assert_eq!(r.is_ok(), true);

    assert_eq!(zip.writee.get_ref().len(), 37);
}
