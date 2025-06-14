use rfd::FileDialog;
use std::fs;
use std::io::{self, Write, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command;

const GAME_DEPENDENT_OVERLAY_PLAT: &str = "0009";
const GAME_DEPENDENT_OVERLAY_HG: &str = "0000";
const PLATINUM_BYTES: [u8; 4] = [0x43, 0x50, 0x55, 0x45]; // "CPUE" in bytes
const HEARTGOLD_BYTES: [u8; 4] = [0x49, 0x50, 0x4B, 0x45]; // "IPKE" in bytes

fn get_project_path() -> PathBuf {
    // Use rfd to open a file dialog and select the project path
    if let Some(selected_folder) = FileDialog::new()
        .set_title("Select unpacked ROM folder")
        .pick_folder()
    {
        println!("Selected folder: {:?}", selected_folder);
        selected_folder
    } else {
        println!("No folder selected, exiting.");
        std::process::exit(0);
    }
}

fn get_patch_path() -> PathBuf {
    // Use rfd to open a file dialog and select the project path
    if let Some(selected_folder) = FileDialog::new()
        .set_title("Select Patch file")
        .pick_file()
    {
        println!("Selected patch: {:?}", selected_folder);
        selected_folder
    } else {
        println!("No patch selected, exiting.");
        std::process::exit(0);
    }
}

fn determine_game_version(project_path: &str) -> String {
    let header_path = PathBuf::from(project_path).join("header.bin");

    if let Ok(mut file) = fs::File::open(&header_path) {
        let mut buf = [0u8; 4];
        file.seek(SeekFrom::Start(0xC))
            .expect("Failed to seek in header.bin");
        file.read_exact(&mut buf)
            .expect("Failed to read from header.bin");
        if buf == PLATINUM_BYTES {
            "Platinum".to_string()
        } else if buf == HEARTGOLD_BYTES {
            "HeartGold".to_string()
        } else {
            panic!("Unknown game version in header.bin at path: {}\nBytes found:{:02X} {:02X} {:02X} {:02X}",
                   header_path.display(), buf[0], buf[1], buf[2], buf[3]);
        }
    } else {
        panic!("header.bin not found at path: {}", header_path.display());
    }
}

fn is_patch_compatible(patch_path: &str, project_path: &str) -> bool {
    // Check if the patch path contains the project path
    if patch_path.contains("_HG") && determine_game_version(project_path) == "HeartGold" {
        true
    } else if patch_path.contains("_PLAT") && determine_game_version(project_path) == "Platinum" {
        true
    } else {
        false
    }
}

fn is_arm9_expanded(project_path: &str, game_version_overlay: &str) -> bool {
    let arm9_path = PathBuf::from(project_path).join("arm9.bin");

    match game_version_overlay {
        GAME_DEPENDENT_OVERLAY_HG => {
            if let Ok(mut file) = fs::File::open(&arm9_path) {
                let mut buf = [0u8; 4];
                if file.seek(SeekFrom::Start(0xCD0)).is_ok() && file.read_exact(&mut buf).is_ok() {
                    //println!("Read bytes at 0xCD0: {:02X} {:02X} {:02X} {:02X}",
                    //         buf[0], buf[1], buf[2], buf[3]);
                    buf == [0x0F, 0xF1, 0x30, 0xFB]
                } else {
                    false
                }
            } else {
                panic!("arm9.bin not found at path: {}", arm9_path.display());
            }
        }
        GAME_DEPENDENT_OVERLAY_PLAT => {
            if let Ok(mut file) = fs::File::open(&arm9_path) {
                let mut buf = [0u8; 4];
                if file.seek(SeekFrom::Start(0xCB4)).is_ok() && file.read_exact(&mut buf).is_ok() {
                    //println!("Read bytes at 0xCB4: {:02X} {:02X} {:02X} {:02X}",
                    //         buf[0], buf[1], buf[2], buf[3]);
                    buf == [0x00, 0xF1, 0xB4, 0xF8]
                } else {
                    false
                }
            } else {
                panic!("arm9.bin not found at path: {}", arm9_path.display());
            }
        }
        _ => panic!("Unknown game version"),
    }
}

/// Find the first aligned offset after the last non-zero byte,
/// where there is at least `required_size` of 0x00 space.
/// The address must end in 0 (i.e., multiple of 0x10).
/// Find the first offset aligned to 0x10 (ending in 0),
/// with at least `required_size` contiguous 0x00 bytes.
pub fn find_injection_offset(data: &[u8], required_size: usize) -> Option<usize> {
    let mut i = 0;

    while i + required_size <= data.len() {
        if i % 0x10 == 0 {
            let window = &data[i..i + required_size];
            if window.iter().all(|&b| b == 0) {
                return Some(i);
            }
        }
        i += 1;
    }

    None
}

fn insert_corrected_offset(asm_path: &str, new_addr: u32) -> std::io::Result<PathBuf> {
    let input = BufReader::new(fs::File::open(asm_path)?);
    let mut lines: Vec<String> = Vec::new();

    for line in input.lines() {
        let mut line = line?;
        if line.contains("INJECT_ADDR equ") {
            line = format!("INJECT_ADDR equ 0x{:08X}", new_addr);
        }
        lines.push(line);
    }

    let out_path = PathBuf::from(asm_path);
    fs::write(&out_path, lines.join("\n"))?;
    Ok(out_path)
}

/// Determine the game (HG or Plat) based on the patch path.
///
/// If the patch path contains "_HG", it returns the overlay for HeartGold (0000).
/// If it contains "_PLAT", it returns the overlay for Platinum (0009).
///
/// make sure to label patches accordingly!!!
fn determine_game_overlay(patch_path: &str) -> String {
    if patch_path.contains("_HG") {
        GAME_DEPENDENT_OVERLAY_HG.to_string()
    } else if patch_path.contains("_PLAT") {
        GAME_DEPENDENT_OVERLAY_PLAT.to_string()
    } else {
        panic!("Unknown game type in patch path: {}", patch_path);
    }
}

fn run_armips(asm_path: &str, rom_dir: &str) -> std::io::Result<()> {
    // Get absolute path to .asm
    //let full_asm_path = fs::canonicalize(asm_path)?;

    println!("Running armips...");

    let armips_path = Path::new("bin").join("armips.exe");

    let status = Command::new(armips_path)
        .arg(asm_path)
        .current_dir(rom_dir)
        .status()?;

    if !status.success() {
        println!("armips failed with exit code: {:?}", status.code());
        std::process::exit(1);
    }

    Ok(())
}

fn main() -> std::io::Result<()> {
    // Get the project path from the user
    let project_path = get_project_path().display().to_string();

    println!("Game version: {}", determine_game_version(&project_path));

    // placeholder for patch path logic
    //let patch_path = "./patches/EVIV_HG.asm".to_string();
    let patch_path = get_patch_path().display().to_string();

    if !is_patch_compatible(&patch_path, &project_path) {
        println!("This patch is not compatible with this ROM, exiting.");
        return Ok(());
    }

    if is_arm9_expanded(&project_path, &determine_game_overlay(&patch_path)) {
        println!("arm9 is expanded, proceeding");
    } else {
        println!("arm9 is not expanded, please expand it before running this tool.");
        return Ok(());
    }

    let synth_overlay_path = format!(
        "{}/unpacked/synthOverlay/{}",
        project_path,
        determine_game_overlay(&patch_path)
    );
    let synth_overlay = fs::read(&synth_overlay_path)?;
    println!(
        "Read synthOverlay file successfully. Located at: {:?}",
        &synth_overlay_path
    );

    println!("Searching for injection offset");
    let offset =
        find_injection_offset(&synth_overlay, 0x1000).expect("Failed to find injection offset");
    println!(
        "Found injection offset at {:#X} in synthOverlay {}",
        offset,
        determine_game_overlay(&patch_path)
    );

    let corrected_offset = 0x23c8000 + offset as u32;
    println!("Corrected offset: {:#X}", corrected_offset);
    insert_corrected_offset(&patch_path, corrected_offset)
        .expect("Failed to correct offset in asm file");

    run_armips(&patch_path, &project_path).expect("Failed to run armips");
    println!("\narmips ran successfully, patch applied!\n");

    println!("\nPress Enter to exit...");
    let _ = io::stdout().flush(); // Make sure the message is printed before pause
    let _ = io::stdin().read_line(&mut String::new());

    Ok(())
}
