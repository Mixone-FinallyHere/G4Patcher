use rfd::FileDialog;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{PathBuf};
use std::process::Command;

const GAME_DEPENDENT_OVERLAY_PLAT: &str = "0009";
const GAME_DEPENDENT_OVERLAY_HG: &str = "0000";
const PLATINUM_BYTES: [u8; 4] = [0x43, 0x50, 0x55, 0x45]; // "CPUE" in bytes
const HEARTGOLD_BYTES: [u8; 4] = [0x49, 0x50, 0x4B, 0x45]; // "IPKE" in bytes
const SOUSILVER_BYTES: [u8; 4] = [0x49, 0x50, 0x47, 0x45]; // "IPGE" in bytes
const PLATINUM: &str = "Platinum";
const HEARTGOLD: &str = "HeartGold";
const SOULSILVER: &str = "SoulSilver";

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

fn get_patch_path(exe_dir: &PathBuf) -> PathBuf {

    println!("Please select the patch file to apply");

    let patches_dir = exe_dir.clone().join("patches");

    // Use rfd to open a file dialog and select the project path
    if let Some(selected_patch) = FileDialog::new()
        .add_filter("Patch files", &["asm"])
        .set_title("Select Patch file")
        .set_directory(patches_dir)
        .pick_file()
    {
        println!("Selected patch: {:?}", selected_patch);
        selected_patch
    } else {
        println!("No patch selected, exiting.");
        std::process::exit(0);
    }
}

fn determine_game_version(project_path: &str) -> &str {
    let header_path = PathBuf::from(project_path).join("header.bin");

    if let Ok(mut file) = fs::File::open(&header_path) {
        let mut buf = [0u8; 4];
        file.seek(SeekFrom::Start(0xC))
            .expect("Failed to seek in header.bin");
        file.read_exact(&mut buf)
            .expect("Failed to read from header.bin");
        if buf == PLATINUM_BYTES {
            PLATINUM
        } else if buf == HEARTGOLD_BYTES {
            HEARTGOLD
        } else if buf == SOUSILVER_BYTES {
            SOULSILVER
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
    match determine_game_version(project_path) {
        PLATINUM if patch_path.contains("_PLAT") => true,
        HEARTGOLD if patch_path.contains("_HG") => true,
        SOULSILVER if patch_path.contains("_SS") => true,
        _ => false,
    }
}

fn is_arm9_expanded(project_path: &str, game_version: &str) -> bool {
    let arm9_path = PathBuf::from(project_path).join("arm9.bin");

    match game_version {
        HEARTGOLD | SOULSILVER => {
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
        PLATINUM => {
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

/// Determine the game (HG or Plat) based on the patch path.
///
/// If the patch path contains "_HG", it returns the overlay for HeartGold (0000).
/// If it contains "_PLAT", it returns the overlay for Platinum (0009).
///
/// Make sure to label patches accordingly!!!
fn determine_game_overlay(patch_path: &str) -> String {
    if patch_path.contains("_HG") {
        GAME_DEPENDENT_OVERLAY_HG.to_string()
    } else if patch_path.contains("_PLAT") {
        GAME_DEPENDENT_OVERLAY_PLAT.to_string()
    } else {
        panic!("Unknown game type in patch path: {}", patch_path);
    }
}

/// Find the first aligned offset after the last non-zero byte,
/// where there is at least `required_size` of 0x00 space.
/// The address must end in 0 (i.e., multiple of 0x10).
/// Find the first offset aligned to 0x10 (ending in 0),
/// with at least `required_size` continuous 0x00 bytes.
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


fn run_armips(asm_path: &str, rom_dir: &str, exe_dir: &PathBuf) -> std::io::Result<()> {
    // Get absolute path to .asm
    //let full_asm_path = fs::canonicalize(asm_path)?;

    println!("Running armips...");

    let armips_path = exe_dir.join("assets").join("armips.exe");
    println!("Using armips at: {:?}", armips_path);

    //let armips_path = Path::new("bin").join("armips.exe");

    let status = Command::new(armips_path)
        .arg(asm_path)
        .current_dir(rom_dir)
        .status()?;

    if !status.success() {
        println!("armips failed with exit code: {:?}", status.code().unwrap());
        std::process::exit(1);
    }

    Ok(())
}

fn enter_to_exit() -> io::Result<()> {
    println!("\nPress Enter to exit...");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut String::new());
    Ok(())
}

fn main() -> io::Result<()> {

    println!("Welcome to the Platinum/HeartGold code injection patcher!\nPlease select your unpacked ROM folder");

    // Get the project path from the user
    let project_path = get_project_path().display().to_string();
    println!("Game version: {}", determine_game_version(&project_path));

    // Check if the arm9 is expanded, if not, prompt the user to expand it
    if is_arm9_expanded(&project_path, determine_game_version(&project_path)) {
        println!("arm9 is expanded, proceeding");
    } else {
        println!("arm9 is not expanded, please expand it before running this tool.");
        return enter_to_exit();
    }

    // Get the directory of the executable for the patch file and armips locations
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|parent| parent.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    // Get the selected patch file from the user
    let patch_path = get_patch_path(&exe_dir).display().to_string();

    // Check if the patch is compatible with the selected ROM
    if !is_patch_compatible(&patch_path, &project_path) {
        println!("This patch is not compatible with this ROM, please select a compatible patch.");
        return enter_to_exit();
    }

    // Read and process the synthOverlay file
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

    run_armips(&patch_path, &project_path, &exe_dir).expect("Failed to run armips");
    println!("\narmips ran successfully, patch applied! You can now repack your ROM.\n");

    enter_to_exit()
}
