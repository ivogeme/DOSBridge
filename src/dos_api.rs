


use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};

use unicorn_engine::{RegisterX86, Unicorn};

const CF: u64 = 1;

pub struct DosApi {
    files: HashMap<u16, File>,
    next_handle: u16,
    halted: bool,
}

impl DosApi {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            next_handle: 5,
            halted: false,
        }
    }

    pub fn halted(&self) -> bool {
        self.halted
    }

    fn get_reg(emu: &Unicorn<'_, ()>, reg: RegisterX86) -> Result<u16, String> {
        emu.reg_read(reg)
            .map(|v| v as u16)
            .map_err(|e| format!("erro lendo registrador: {:?}", e))
    }

    fn set_reg(
        emu: &mut Unicorn<'_, ()>,
        reg: RegisterX86,
        value: u16,
    ) -> Result<(), String> {
        emu.reg_write(reg, value as u64)
            .map_err(|e| format!("erro escrevendo registrador: {:?}", e))
    }

    fn set_al(emu: &mut Unicorn<'_, ()>, value: u8) -> Result<(), String> {
        let ax = Self::get_reg(emu, RegisterX86::AX)?;
        let new_ax = (ax & 0xFF00) | value as u16;
        Self::set_reg(emu, RegisterX86::AX, new_ax)
    }

    fn set_cf(emu: &mut Unicorn<'_, ()>, value: bool) -> Result<(), String> {
        let mut flags = emu
            .reg_read(RegisterX86::EFLAGS)
            .map_err(|e| format!("erro lendo EFLAGS: {:?}", e))?;

        if value {
            flags |= CF;
        } else {
            flags &= !CF;
        }

        emu.reg_write(RegisterX86::EFLAGS, flags)
            .map_err(|e| format!("erro escrevendo EFLAGS: {:?}", e))
    }

    fn fail(emu: &mut Unicorn<'_, ()>, error: u16) -> Result<(), String> {
        Self::set_reg(emu, RegisterX86::AX, error)?;
        Self::set_cf(emu, true)
    }

    fn success(emu: &mut Unicorn<'_, ()>) -> Result<(), String> {
        Self::set_cf(emu, false)
    }

    fn read_memory(
        emu: &Unicorn<'_, ()>,
        address: usize,
        buffer: &mut [u8],
    ) -> Result<(), String> {
        emu.mem_read(address as u64, buffer)
            .map_err(|e| format!("erro lendo memória DOS: {:?}", e))
    }

    fn write_memory(
        emu: &mut Unicorn<'_, ()>,
        address: usize,
        buffer: &[u8],
    ) -> Result<(), String> {
        emu.mem_write(address as u64, buffer)
            .map_err(|e| format!("erro escrevendo memória DOS: {:?}", e))
    }

    fn read_dos_string(
        emu: &Unicorn<'_, ()>,
        segment: u16,
        offset: u16,
    ) -> Result<Vec<u8>, String> {
        let start = ((segment as usize) << 4) + offset as usize;
        let mut result = Vec::new();
        const MAX_STRING: usize = 64 * 1024;

        for i in 0..MAX_STRING {
            let address = start
                .checked_add(i)
                .ok_or_else(|| "overflow de endereço".to_string())?;

            let mut byte = [0u8; 1];
            Self::read_memory(emu, address, &mut byte)?;

            if byte[0] == b'$' {
                return Ok(result);
            }

            result.push(byte[0]);
        }

        Err("string DOS sem terminador '$'".to_string())
    }

    fn dos_path_to_host(path: &[u8]) -> String {
        let mut text = String::from_utf8_lossy(path).into_owned();
        text = text.replace('\\', "/");

        if text.len() >= 2 && text.as_bytes()[1] == b':' {
            text = text[2..].to_string();
        }

        while text.starts_with('/') {
            text.remove(0);
        }

        if text.is_empty() {
            ".".to_string()
        } else {
            text
        }
    }

    fn allocate_handle(&mut self) -> u16 {
        loop {
            let handle = self.next_handle;
            self.next_handle = self.next_handle.wrapping_add(1);

            if handle < 5 {
                continue;
            }

            if !self.files.contains_key(&handle) {
                return handle;
            }
        }
    }

    fn read_stdin_byte() -> io::Result<u8> {
        let mut buffer = [0u8; 1];
        io::stdin().read_exact(&mut buffer)?;
        Ok(buffer[0])
    }

    fn write_stdout(byte: u8) -> io::Result<()> {
        let mut stdout = io::stdout();
        stdout.write_all(&[byte])?;
        stdout.flush()?;
        Ok(())
    }

    fn handle_01h(&mut self, emu: &mut Unicorn<'_, ()>) -> Result<(), String> {
        match Self::read_stdin_byte() {
            Ok(byte) => {
                Self::set_al(emu, byte)?;
                Self::write_stdout(byte).map_err(|e| e.to_string())?;
                Self::success(emu)
            }
            Err(_) => Self::fail(emu, 0x0005),
        }
    }

    fn handle_02h(&mut self, emu: &mut Unicorn<'_, ()>) -> Result<(), String> {
        let dx = Self::get_reg(emu, RegisterX86::DX)?;
        let byte = (dx & 0x00FF) as u8;

        Self::write_stdout(byte).map_err(|e| e.to_string())?;
        Self::set_al(emu, byte)?;
        Self::success(emu)
    }

    fn handle_07h(&mut self, emu: &mut Unicorn<'_, ()>) -> Result<(), String> {
        match Self::read_stdin_byte() {
            Ok(byte) => {
                Self::set_al(emu, byte)?;
                Self::success(emu)
            }
            Err(_) => Self::fail(emu, 0x0005),
        }
    }

    fn handle_09h(&mut self, emu: &mut Unicorn<'_, ()>) -> Result<(), String> {
        let ds = Self::get_reg(emu, RegisterX86::DS)?;
        let dx = Self::get_reg(emu, RegisterX86::DX)?;
        let bytes = Self::read_dos_string(emu, ds, dx)?;

        let mut stdout = io::stdout();
        stdout.write_all(&bytes).map_err(|e| e.to_string())?;
        stdout.flush().map_err(|e| e.to_string())?;

        Self::set_al(emu, b'$')?;
        Self::success(emu)
    }

    fn handle_3ch(&mut self, emu: &mut Unicorn<'_, ()>) -> Result<(), String> {
        let ds = Self::get_reg(emu, RegisterX86::DS)?;
        let dx = Self::get_reg(emu, RegisterX86::DX)?;
        let name = Self::read_dos_string(emu, ds, dx)?;
        let path = Self::dos_path_to_host(&name);

        match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
        {
            Ok(file) => {
                let handle = self.allocate_handle();
                self.files.insert(handle, file);

                Self::set_reg(emu, RegisterX86::AX, handle)?;
                Self::success(emu)
            }
            Err(_) => Self::fail(emu, 0x0005),
        }
    }

    fn handle_3dh(&mut self, emu: &mut Unicorn<'_, ()>) -> Result<(), String> {
        let ds = Self::get_reg(emu, RegisterX86::DS)?;
        let dx = Self::get_reg(emu, RegisterX86::DX)?;
        let name = Self::read_dos_string(emu, ds, dx)?;
        let path = Self::dos_path_to_host(&name);

        match File::open(&path) {
            Ok(file) => {
                let handle = self.allocate_handle();
                self.files.insert(handle, file);

                Self::set_reg(emu, RegisterX86::AX, handle)?;
                Self::success(emu)
            }
            Err(_) => Self::fail(emu, 0x0002),
        }
    }

    fn handle_3eh(&mut self, emu: &mut Unicorn<'_, ()>) -> Result<(), String> {
        let bx = Self::get_reg(emu, RegisterX86::BX)?;

        if self.files.remove(&bx).is_some() {
            Self::success(emu)
        } else {
            Self::fail(emu, 0x0006)
        }
    }

    fn handle_3fh(&mut self, emu: &mut Unicorn<'_, ()>) -> Result<(), String> {
        let bx = Self::get_reg(emu, RegisterX86::BX)?;
        let cx = Self::get_reg(emu, RegisterX86::CX)?;
        let ds = Self::get_reg(emu, RegisterX86::DS)?;
        let dx = Self::get_reg(emu, RegisterX86::DX)?;
        let count = cx as usize;
        let mut buffer = vec![0u8; count];

        let result = if bx == 0 {
            io::stdin()
                .read(&mut buffer)
                .map_err(|e| e.to_string())
        } else if let Some(file) = self.files.get_mut(&bx) {
            file.read(&mut buffer).map_err(|e| e.to_string())
        } else {
            return Self::fail(emu, 0x0006);
        };

        match result {
            Ok(read_count) => {
                let address = ((ds as usize) << 4) + dx as usize;

                if address + read_count > crate::loader_mz::MEMORY_SIZE {
                    return Self::fail(emu, 0x0009);
                }

                Self::write_memory(emu, address, &buffer[..read_count])?;
                Self::set_reg(emu, RegisterX86::AX, read_count as u16)?;
                Self::success(emu)
            }
            Err(_) => Self::fail(emu, 0x0005),
        }
    }

    fn handle_40h(&mut self, emu: &mut Unicorn<'_, ()>) -> Result<(), String> {
        let bx = Self::get_reg(emu, RegisterX86::BX)?;
        let cx = Self::get_reg(emu, RegisterX86::CX)?;
        let ds = Self::get_reg(emu, RegisterX86::DS)?;
        let dx = Self::get_reg(emu, RegisterX86::DX)?;
        let count = cx as usize;
        let address = ((ds as usize) << 4) + dx as usize;

        if address + count > crate::loader_mz::MEMORY_SIZE {
            return Self::fail(emu, 0x0009);
        }

        let mut buffer = vec![0u8; count];
        Self::read_memory(emu, address, &mut buffer)?;

        let result = if bx == 1 {
            io::stdout()
                .write(&buffer)
                .map_err(|e| e.to_string())
        } else if bx == 2 {
            io::stderr()
                .write(&buffer)
                .map_err(|e| e.to_string())
        } else if let Some(file) = self.files.get_mut(&bx) {
            file.write(&buffer).map_err(|e| e.to_string())
        } else {
            return Self::fail(emu, 0x0006);
        };

        match result {
            Ok(written) => {
                Self::set_reg(emu, RegisterX86::AX, written as u16)?;
                Self::success(emu)
            }
            Err(_) => Self::fail(emu, 0x0005),
        }
    }

    fn handle_4ch(&mut self, emu: &mut Unicorn<'_, ()>) -> Result<(), String> {
        let ax = Self::get_reg(emu, RegisterX86::AX)?;
        let code = (ax & 0x00FF) as u8;

        println!(
            "\n[DOSBridge] programa terminou com código {}",
            code
        );

        self.halted = true;
        Self::success(emu)?;

        emu.emu_stop()
            .map_err(|e| format!("erro parando Unicorn: {:?}", e))
    }

    pub fn handle_interrupt(
        &mut self,
        emu: &mut Unicorn<'_, ()>,
        interrupt: u32,
    ) -> Result<(), String> {
        if interrupt != 0x21 {
            return Err(format!(
                "interrupção não suportada: INT {:02X}h",
                interrupt
            ));
        }

        let ax = Self::get_reg(emu, RegisterX86::AX)?;
        let ah = (ax >> 8) as u8;

        match ah {
            0x01 => self.handle_01h(emu),
            0x02 => self.handle_02h(emu),
            0x07 => self.handle_07h(emu),
            0x09 => self.handle_09h(emu),
            0x3C => self.handle_3ch(emu),
            0x3D => self.handle_3dh(emu),
            0x3E => self.handle_3eh(emu),
            0x3F => self.handle_3fh(emu),
            0x40 => self.handle_40h(emu),
            0x4C => self.handle_4ch(emu),
            _ => {
                eprintln!(
                    "[DOSBridge] INT 21h AH={:02X}h não implementada",
                    ah
                );
                Self::fail(emu, 0x0001)
            }
        }
    }
}
