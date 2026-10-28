mod dos_api;
mod loader_mz;

use std::env;
use std::process;

use dos_api::DosApi;
use loader_mz::LoadedProgram;

use unicorn_engine::{
    unicorn_const::{Arch, Mode, Prot},
    RegisterX86,
    Unicorn,
};

fn run(program_path: &str) -> Result<(), String> {
    println!("DOSBridge 0.5.0");
    println!("----------------");
    println!("Programa: {}", program_path);

    let program = LoadedProgram::load(program_path)
        .map_err(|e| format!("erro carregando programa: {}", e))?;

    println!("Formato: {:?}", program.format);
    println!("Entrada: {:04X}:{:04X}", program.cs, program.ip);
    println!("Stack:   {:04X}:{:04X}", program.ss, program.sp);

    let mut emu = Unicorn::new(Arch::X86, Mode::MODE_16)
        .map_err(|e| format!("não foi possível iniciar Unicorn: {:?}", e))?;

    emu.mem_map(
        0,
        loader_mz::MEMORY_SIZE as u64,
        Prot::ALL,
    )
    .map_err(|e| format!("erro mapeando memória: {:?}", e))?;

    emu.mem_write(0, &program.memory)
        .map_err(|e| format!("erro copiando memória: {:?}", e))?;

    emu.reg_write(RegisterX86::CS, program.cs as u64)
        .map_err(|e| format!("CS: {:?}", e))?;
    emu.reg_write(RegisterX86::IP, program.ip as u64)
        .map_err(|e| format!("IP: {:?}", e))?;
    emu.reg_write(RegisterX86::DS, program.ds as u64)
        .map_err(|e| format!("DS: {:?}", e))?;
    emu.reg_write(RegisterX86::ES, program.es as u64)
        .map_err(|e| format!("ES: {:?}", e))?;
    emu.reg_write(RegisterX86::SS, program.ss as u64)
        .map_err(|e| format!("SS: {:?}", e))?;
    emu.reg_write(RegisterX86::SP, program.sp as u64)
        .map_err(|e| format!("SP: {:?}", e))?;

    emu.reg_write(RegisterX86::AX, 0)
        .map_err(|e| format!("AX: {:?}", e))?;
    emu.reg_write(RegisterX86::BX, 0)
        .map_err(|e| format!("BX: {:?}", e))?;
    emu.reg_write(RegisterX86::CX, 0)
        .map_err(|e| format!("CX: {:?}", e))?;
    emu.reg_write(RegisterX86::DX, 0)
        .map_err(|e| format!("DX: {:?}", e))?;

    let mut dos = DosApi::new();

    emu.add_intr_hook(move |emu, interrupt| {
        if let Err(error) = dos.handle_interrupt(emu, interrupt) {
            eprintln!("[DOSBridge] erro: {}", error);
            let _ = emu.emu_stop();
        }
    })
    .map_err(|e| format!("erro instalando hook de interrupção: {:?}", e))?;

    let entry = ((program.cs as u64) << 4) + program.ip as u64;

    println!("Executando em endereço físico {:05X}h...", entry);

    emu.emu_start(entry, 0, 0, 0)
        .map_err(|e| format!("erro durante execução: {:?}", e))?;

    println!("DOSBridge: execução encerrada.");
    Ok(())
}

fn main() {
    let mut args = env::args();
    let _program_name = args.next();

    let Some(path) = args.next() else {
        eprintln!("Uso:");
        eprintln!("  dosbridge programa.com");
        eprintln!("  dosbridge programa.exe");
        process::exit(1);
    };

    if let Err(error) = run(&path) {
        eprintln!("DOSBridge: {}", error);
        process::exit(1);
    }
}
