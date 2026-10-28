use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

pub const MEMORY_SIZE: usize = 1024 * 1024;
pub const LOAD_SEGMENT: u16 = 0x1000;

#[derive(Debug, Clone, Copy)]
pub struct MzHeader {
    pub bytes_last_page: u16,
    pub pages_in_file: u16,
    pub relocation_count: u16,
    pub header_paragraphs: u16,
    pub min_extra_paragraphs: u16,
    pub max_extra_paragraphs: u16,
    pub initial_ss: u16,
    pub initial_sp: u16,
    pub checksum: u16,
    pub initial_ip: u16,
    pub initial_cs: u16,
    pub relocation_table_offset: u16,
    pub overlay_number: u16,
}

impl MzHeader {
    pub fn parse(header: &[u8]) -> io::Result<Self> {
        if header.len() < 28 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "MZ header incompleto",
            ));
        }

        let signature = &header[0..2];

        if signature != b"MZ" && signature != b"ZM" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "assinatura MZ inválida",
            ));
        }

        Ok(Self {
            bytes_last_page: u16::from_le_bytes([header[2], header[3]]),
            pages_in_file: u16::from_le_bytes([header[4], header[5]]),
            relocation_count: u16::from_le_bytes([header[6], header[7]]),
            header_paragraphs: u16::from_le_bytes([header[8], header[9]]),
            min_extra_paragraphs: u16::from_le_bytes([header[10], header[11]]),
            max_extra_paragraphs: u16::from_le_bytes([header[12], header[13]]),
            initial_ss: u16::from_le_bytes([header[14], header[15]]),
            initial_sp: u16::from_le_bytes([header[16], header[17]]),
            checksum: u16::from_le_bytes([header[18], header[19]]),
            initial_ip: u16::from_le_bytes([header[20], header[21]]),
            initial_cs: u16::from_le_bytes([header[22], header[23]]),
            relocation_table_offset: u16::from_le_bytes([header[24], header[25]]),
            overlay_number: u16::from_le_bytes([header[26], header[27]]),
        })
    }

    pub fn image_size(&self) -> usize {
        if self.pages_in_file == 0 {
            return 0;
        }

        let total = self.pages_in_file as usize * 512;

        let used_last_page = if self.bytes_last_page == 0 {
            512
        } else {
            self.bytes_last_page as usize
        };

        total - (512 - used_last_page)
    }

    pub fn header_size(&self) -> usize {
        self.header_paragraphs as usize * 16
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ExecutableFormat {
    Com,
    Mz,
}

#[derive(Debug)]
pub struct LoadedProgram {
    pub format: ExecutableFormat,
    pub memory: Vec<u8>,
    pub cs: u16,
    pub ip: u16,
    pub ds: u16,
    pub es: u16,
    pub ss: u16,
    pub sp: u16,
}

impl LoadedProgram {
    pub fn load(path: &str) -> io::Result<Self> {
        let mut file = File::open(path)?;

        let file_size = file.metadata()?.len() as usize;

        if file_size < 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "arquivo pequeno demais",
            ));
        }

        let mut magic = [0u8; 2];
        file.read_exact(&mut magic)?;

        file.seek(SeekFrom::Start(0))?;

        if magic == *b"MZ" || magic == *b"ZM" {
            Self::load_mz(file, file_size)
        } else {
            Self::load_com(file, file_size)
        }
    }

    fn load_com(mut file: File, file_size: usize) -> io::Result<Self> {
        const COM_OFFSET: usize = 0x100;

        if COM_OFFSET + file_size > 0x10000 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "programa COM maior que o espaço disponível",
            ));
        }

        let mut memory = vec![0u8; MEMORY_SIZE];

        let base = (LOAD_SEGMENT as usize) << 4;
        let start = base + COM_OFFSET;

        if start + file_size > MEMORY_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "programa COM excede a memória",
            ));
        }

        file.read_exact(&mut memory[start..start + file_size])?;

        /*
         * Ambiente mínimo:
         *
         * LOAD_SEGMENT: PSP simplificado
         * LOAD_SEGMENT:0100: programa COM
         */
        let cs = LOAD_SEGMENT;
        let ds = LOAD_SEGMENT;
        let es = LOAD_SEGMENT;
        let ss = LOAD_SEGMENT;
        let sp = 0xFFFE;

        Ok(Self {
            format: ExecutableFormat::Com,
            memory,
            cs,
            ip: COM_OFFSET as u16,
            ds,
            es,
            ss,
            sp,
        })
    }

    fn load_mz(mut file: File, file_size: usize) -> io::Result<Self> {
        let mut raw_header = [0u8; 28];
        file.read_exact(&mut raw_header)?;

        let header = MzHeader::parse(&raw_header)?;

        let image_size = header.image_size();
        let header_size = header.header_size();

        if image_size < header_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "tamanho da imagem MZ inválido",
            ));
        }

        if image_size > file_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "imagem MZ maior que o arquivo",
            ));
        }

        if header.relocation_table_offset as usize
            + header.relocation_count as usize * 4
            > header_size
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "tabela de relocação inválida",
            ));
        }

        let image_bytes = image_size - header_size;
        let load_address = (LOAD_SEGMENT as usize) << 4;

        if load_address + image_bytes > MEMORY_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "imagem MZ excede a memória disponível",
            ));
        }

        let mut memory = vec![0u8; MEMORY_SIZE];

        file.seek(SeekFrom::Start(header_size as u64))?;

        file.read_exact(
            &mut memory[load_address..load_address + image_bytes],
        )?;

        /*
         * Relocações MZ.
         *
         * Cada entrada contém:
         * offset:u16
         * segment:u16
         */
        file.seek(SeekFrom::Start(
            header.relocation_table_offset as u64,
        ))?;

        for _ in 0..header.relocation_count {
            let mut entry = [0u8; 4];
            file.read_exact(&mut entry)?;

            let offset = u16::from_le_bytes([entry[0], entry[1]]);
            let segment = u16::from_le_bytes([entry[2], entry[3]]);

            let physical =
                ((LOAD_SEGMENT as usize + segment as usize) << 4)
                    + offset as usize;

            if physical + 2 > MEMORY_SIZE {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "relocação fora da memória",
                ));
            }

            let old_value =
                u16::from_le_bytes([memory[physical], memory[physical + 1]]);

            let new_value = old_value.wrapping_add(LOAD_SEGMENT);
            let bytes = new_value.to_le_bytes();

            memory[physical] = bytes[0];
            memory[physical + 1] = bytes[1];
        }

        let cs = LOAD_SEGMENT.wrapping_add(header.initial_cs);
        let ss = LOAD_SEGMENT.wrapping_add(header.initial_ss);

        /*
         * DS/ES apontam para o segmento de carregamento.
         * Um DOS real também inicializaria o PSP.
         */
        let ds = LOAD_SEGMENT;
        let es = LOAD_SEGMENT;

        Ok(Self {
            format: ExecutableFormat::Mz,
            memory,
            cs,
            ip: header.initial_ip,
            ds,
            es,
            ss,
            sp: header.initial_sp,
        })
    }

    pub fn linear_address(segment: u16, offset: u16) -> usize {
        ((segment as usize) << 4) + offset as usize
    }
}
