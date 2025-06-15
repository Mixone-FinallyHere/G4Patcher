#![warn(clippy::nursery, clippy::pedantic)]

mod constants;
mod synthoverlay_utils;
mod usage_checks;

use rfd::FileDialog;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use synthoverlay_utils::{determine_game_overlay, find_injection_offset, insert_corrected_offset};
use usage_checks::{determine_game_version, is_arm9_expanded, is_patch_compatible};

fn get_project_path() -> PathBuf {
    // Use rfd to open a file dialog and select the project path
    FileDialog::new()
        .set_title("Select unpacked ROM folder")
        .pick_folder()
        .map_or_else(
            || {
                println!("No folder selected, exiting.");
                std::process::exit(0);
            },
            |selected_folder| {
                println!("Selected folder: {}", selected_folder.display());
                selected_folder
            },
        )
}

fn get_patch_path(exe_dir: &Path) -> PathBuf {
    println!("Please select the patch file to apply");

    let patches_dir = exe_dir.join("patches");

    // Use rfd to open a file dialog and select the project path
    FileDialog::new()
        .add_filter("Patch files", &["asm"])
        .set_title("Select Patch file")
        .set_directory(patches_dir)
        .pick_file()
        .map_or_else(
            || {
                println!("No patch selected, exiting.");
                std::process::exit(0);
            },
            |selected_patch| {
                println!("Selected patch: {}", selected_patch.display());
                selected_patch
            },
        )
}

fn run_armips(asm_path: &str, rom_dir: &str, exe_dir: &Path) -> io::Result<()> {
    println!("Running armips...");

    let armips_path = exe_dir.join("assets").join("armips.exe");
    println!("Using armips at: {}", armips_path.display());

    let status = Command::new(armips_path)
        .arg(asm_path)
        .current_dir(rom_dir)
        .status()?;

    if !status.success() {
        println!("armips failed with exit code: {}", status.code().unwrap());
        return Err(io::Error::other("armips failed to run"));
    }

    Ok(())
}

fn enter_to_exit() -> Result<(), io::Error> {
    println!("\nPress Enter to exit...");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut String::new());
    Ok(())
}

fn main() -> io::Result<()> {
    println!("Welcome to the Platinum/HGSS code injection patcher!\nPlease select your unpacked ROM folder");

    // Get the project path from the user
    let project_path = get_project_path().display().to_string();
    println!("Game version: {}", determine_game_version(&project_path)?);

    // Check if the arm9 is expanded, if not, prompt the user to expand it
    if is_arm9_expanded(
        &project_path,
        determine_game_version(&project_path).unwrap(),
    )? {
        println!("arm9 is expanded, proceeding");
    } else {
        println!("arm9 is not expanded, please expand it before running this tool.");
        return enter_to_exit();
    }

    // Get the directory of the executable for the patch file and armips locations
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(std::path::Path::to_path_buf))
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
    println!("Corrected offset: {corrected_offset:#X}");
    insert_corrected_offset(&patch_path, corrected_offset)
        .expect("Failed to correct offset in asm file");

    if matches!(run_armips(&patch_path, &project_path, &exe_dir), Ok(())) {
        println!("\narmips ran successfully, patch applied! You can now repack your ROM.\n");
    } else {
        return enter_to_exit();
    }

    enter_to_exit()
}
