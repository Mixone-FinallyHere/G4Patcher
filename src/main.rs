#![warn(clippy::nursery, clippy::pedantic)]

mod constants;
mod synthoverlay_utils;
mod usage_checks;

use constants::{PATCH_DIRECTIVE, PREASSEMBLE_DIRECTIVE};
use eframe::egui;
use rfd::FileDialog;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use synthoverlay_utils::handle_synthoverlay;
use usage_checks::{determine_game_version, is_patch_compatible, needs_synthoverlay};
use clap::Parser;
use log::{debug, error, info, warn};

#[derive(Parser)]
#[clap(about = "Pokemon Gen 4 Code Injection Patcher")]
struct Args {
    #[clap(long)]
    gui: bool,
    #[clap(long, short)]
    rom_folder: Option<String>,
    #[clap(long, short)]
    patch_file: Option<String>,
}

fn get_project_path() -> PathBuf {
    info!("Opening ROM folder dialog...");
    let result = FileDialog::new()
        .set_title("Select unpacked ROM folder")
        .pick_folder();
    match result {
        Some(selected_folder) => {
            info!("Selected folder: {}", selected_folder.display());
            selected_folder
        }
        None => {
            error!("No folder selected, exiting.");
            std::process::exit(0);
        }
    }
}

fn get_patch_path(exe_dir: &Path, game_version: &str) -> PathBuf {
    info!("Opening patch file dialog...");
    let patches_dir = exe_dir.join("patches");
    let directory = if patches_dir.exists() {
        patches_dir.join(match game_version {
            "Platinum" => "PLAT",
            "HeartGold" | "SoulSilver" => "HG_SS",
            _ => "",
        })
    } else {
        warn!("Patches directory {} does not exist, using current directory", patches_dir.display());
        PathBuf::from(".")
    };
    let result = FileDialog::new()
        .add_filter("Patch files", &["asm"])
        .set_title("Select Patch file")
        .set_directory(&directory)
        .pick_file();
    match result {
        Some(selected_patch) => {
            info!("Selected patch: {}", selected_patch.display());
            selected_patch
        }
        None => {
            error!("No patch selected, exiting.");
            std::process::exit(0);
        }
    }
}

fn run_armips(asm_path: &str, rom_dir: &str, exe_dir: &Path, armips_directive: &str) -> io::Result<()> {
    let armips_path = exe_dir.join("assets").join("armips.exe");
    if !armips_path.exists() {
        error!("armips executable not found at {}", armips_path.display());
        return Err(io::Error::new(io::ErrorKind::NotFound, "armips.exe not found"));
    }
    if armips_directive == PREASSEMBLE_DIRECTIVE {
        info!("Calculating patch size...");
        Command::new(&armips_path)
            .args([asm_path, "-definelabel", PREASSEMBLE_DIRECTIVE, "1"])
            .current_dir(rom_dir)
            .status()?;
    } else {
        info!("Patching ROM with armips...");
        Command::new(&armips_path)
            .args([asm_path, "-definelabel", PATCH_DIRECTIVE, "1"])
            .current_dir(rom_dir)
            .status()?;
    }
    Ok(())
}

fn enter_to_exit() -> io::Result<()> {
    info!("\nPress Enter to exit...");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut String::new());
    Ok(())
}

struct G4PatcherApp {
    rom_folder: Option<PathBuf>,
    patch_file: Option<PathBuf>,
    status_message: String,
    exe_dir: PathBuf,
    game_version: Option<String>,
}

impl G4PatcherApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| {
                warn!("Could not determine executable directory, using current directory");
                PathBuf::from(".")
            });
        Self {
            rom_folder: None,
            patch_file: None,
            status_message: String::from("Select an unpacked ROM folder and patch to begin."),
            exe_dir,
            game_version: None,
        }
    }

    fn apply_patch(&mut self) {
        if self.rom_folder.is_none() {
            self.status_message = "Error: No ROM folder selected.".to_string();
            error!("Apply patch failed: No ROM folder selected");
            return;
        }
        if self.patch_file.is_none() {
            self.status_message = "Error: No patch file selected.".to_string();
            error!("Apply patch failed: No patch file selected");
            return;
        }

        let project_path = self.rom_folder.as_ref().unwrap().display().to_string();
        let patch_path = self.patch_file.as_ref().unwrap().display().to_string();
        info!("Applying patch {} to ROM {}", patch_path, project_path);

        let game_version = match determine_game_version(&project_path) {
            Ok(version) => version.to_string(),
            Err(e) => {
                self.status_message = format!("Error determining game version: {}\nPlease ensure you are selecting the ROM folder, not the 'unpacked' folder within it.", e);
                error!("Error determining game version: {}", e);
                return;
            }
        };
        self.status_message = format!("Game version: {}", game_version);
        self.game_version = Some(game_version.clone());
        info!("Detected game version: {}", game_version);

        if !is_patch_compatible(&patch_path, &project_path) {
            self.status_message = "This patch is not compatible with this ROM, please select a compatible patch.".to_string();
            error!("Patch {} is not compatible with ROM {}", patch_path, project_path);
            return;
        }

        if needs_synthoverlay(&patch_path) {
            if !matches!(run_armips(&patch_path, &project_path, &self.exe_dir, PREASSEMBLE_DIRECTIVE), Ok(())) {
                self.status_message = "Error calculating patch size.".to_string();
                error!("Error calculating patch size for {}", patch_path);
                return;
            }

            let patch_size = match fs::metadata(format!("{}/temp.bin", project_path)) {
                Ok(metadata) => metadata.len() as usize,
                Err(e) => {
                    self.status_message = format!("Failed to read temp.bin: {}", e);
                    error!("Failed to read temp.bin: {}", e);
                    return;
                }
            };
            self.status_message = format!("Patch size: {} bytes", patch_size);
            info!("Patch size: {} bytes", patch_size);

            if let Err(e) = fs::remove_file(format!("{}/temp.bin", project_path)) {
                self.status_message = format!("Failed to delete temp.bin: {}", e);
                error!("Failed to delete temp.bin: {}", e);
                return;
            }

            if let Err(e) = handle_synthoverlay(&patch_path, &project_path, &game_version, patch_size) {
                self.status_message = format!("Error handling synthOverlay: {}", e);
                error!("Error handling synthOverlay: {}", e);
                return;
            }
        }

        if matches!(run_armips(&patch_path, &project_path, &self.exe_dir, PATCH_DIRECTIVE), Ok(())) {
            self.status_message = format!("\narmips ran successfully, patch {} applied to {}!\nYou can now repack your ROM.", 
                self.patch_file.as_ref().unwrap().file_name().unwrap().to_string_lossy(), 
                project_path);
            info!("Patch {} applied successfully to {}", patch_path, project_path);
        } else {
            self.status_message = "Error applying patch with armips.".to_string();
            error!("Error applying patch {} with armips", patch_path);
        }
    }
}

impl eframe::App for G4PatcherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Pokemon Gen 4 Code Injection Patcher");

            ui.horizontal(|ui| {
                if ui.button("Select ROM Folder").clicked() {
                    debug!("ROM folder button clicked");
                    if let Some(path) = FileDialog::new()
                        .set_title("Select unpacked ROM folder")
                        .pick_folder() {
                        self.rom_folder = Some(path);
                        self.status_message = format!("Selected ROM folder: {}", self.rom_folder.as_ref().unwrap().display());
                        info!("Selected ROM folder: {}", self.rom_folder.as_ref().unwrap().display());
                    } else {
                        self.status_message = "No ROM folder selected.".to_string();
                        warn!("No ROM folder selected");
                    }
                }
                ui.label(self.rom_folder
                    .as_ref()
                    .map_or("No folder selected".to_string(), |p| p.display().to_string()));
            });

            ui.horizontal(|ui| {
                if ui.button("Select Patch File").clicked() {
                    debug!("Patch file button clicked");
                    let patches_dir = self.exe_dir.join("patches");
                    let directory = if let Some(ref game_version) = self.game_version {
                        if patches_dir.exists() {
                            match game_version.as_str() {
                                "Platinum" => patches_dir.join("PLAT"),
                                "HeartGold" | "SoulSilver" => patches_dir.join("HG_SS"),
                                _ => patches_dir.clone(),
                            }
                        } else {
                            warn!("Patches directory {} does not exist, using current directory", patches_dir.display());
                            PathBuf::from(".")
                        }
                    } else {
                        if patches_dir.exists() {
                            patches_dir.clone()
                        } else {
                            warn!("Patches directory {} does not exist, using current directory", patches_dir.display());
                            PathBuf::from(".")
                        }
                    };
                    if let Some(path) = FileDialog::new()
                        .add_filter("Patch files", &["asm"])
                        .set_title("Select Patch file")
                        .set_directory(&directory)
                        .pick_file() {
                        self.patch_file = Some(path);
                        self.status_message = format!("Selected patch: {}", 
                            self.patch_file.as_ref().unwrap().file_name().unwrap().to_string_lossy().to_string());
                        info!("Selected patch: {}", self.patch_file.as_ref().unwrap().display());
                    } else {
                        self.status_message = "No patch file selected.".to_string();
                        warn!("No patch file selected");
                    }
                }
                ui.label(self.patch_file
                    .as_ref()
                    .map_or("No patch selected".to_string(), |p| p.file_name().unwrap().to_string_lossy().to_string()));
            });

            if ui.button("Apply Patch").clicked() {
                debug!("Apply Patch button clicked");
                self.apply_patch();
            }

            ui.add(
                egui::TextEdit::multiline(&mut self.status_message.clone())
                    .desired_rows(10)
                    .desired_width(f32::INFINITY),
            );

            ui.collapsing("Limitations", |ui| {
                ui.label("- Does not check if patch is already applied (may duplicate).");
                ui.label("- Does not verify overlay compression (ensure hook overlay is uncompressed).");
            });

            ui.label("Make sure to read the documentation for the patch you are applying!");
        });
    }
}

fn run_cli(rom_folder: Option<String>, patch_file: Option<String>) -> io::Result<()> {
    info!("Starting CLI mode");
    println!("Welcome to the Platinum/HGSS code injection patcher!\n\nMake sure to read the documentation for the patch you are trying to apply!\n\nPlease select your unpacked ROM folder");

    let project_path = rom_folder.map(PathBuf::from).unwrap_or_else(get_project_path).display().to_string();
    let game_version = match determine_game_version(&project_path) {
        Ok(version) => version,
        Err(e) => {
            error!("Error determining game version: {}\nPlease ensure you are selecting the ROM folder, and not the \"unpacked\" folder within it.", e);
            enter_to_exit()?;
            return Ok(());
        }
    };
    info!("Game version: {}", game_version);
    println!("Game version: {}", game_version);

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));

    let patch_path = patch_file.map(PathBuf::from).unwrap_or_else(|| get_patch_path(&exe_dir, game_version)).display().to_string();

    if !is_patch_compatible(&patch_path, &project_path) {
        error!("Patch {} is not compatible with ROM {}", patch_path, project_path);
        println!("This patch is not compatible with this ROM, please select a compatible patch.");
        return enter_to_exit();
    }

    if needs_synthoverlay(&patch_path) {
        if !matches!(run_armips(&patch_path, &project_path, &exe_dir, PREASSEMBLE_DIRECTIVE), Ok(())) {
            error!("Failed to calculate patch size for {}", patch_path);
            return enter_to_exit();
        }

        let patch_size = fs::metadata(format!("{}/temp.bin", project_path))
            .map_err(|e| io::Error::new(io::ErrorKind::NotFound, format!("Failed to read temp.bin: {}", e)))?
            .len() as usize;
        info!("Patch size: {} bytes", patch_size);
        println!("Patch size: {} bytes", patch_size);

        fs::remove_file(format!("{}/temp.bin", project_path))
            .map_err(|e| io::Error::new(io::ErrorKind::NotFound, format!("Failed to delete temp.bin: {}", e)))?;

        handle_synthoverlay(&patch_path, &project_path, game_version, patch_size)?;
    }

    if matches!(run_armips(&patch_path, &project_path, &exe_dir, PATCH_DIRECTIVE), Ok(())) {
        info!("Patch {} applied successfully to {}", patch_path, project_path);
        println!("\narmips ran successfully, patch applied! You can now repack your ROM.\n");
    } else {
        error!("Failed to apply patch {} with armips", patch_path);
    }

    enter_to_exit()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init(); // Initialize logger
    let args = Args::parse();

    if args.gui {
        info!("Starting GUI mode");
        let options = eframe::NativeOptions {
            initial_window_size: Some(egui::vec2(600.0, 400.0)),
            ..Default::default()
        };
        eframe::run_native(
            "G4Patcher",
            options,
            Box::new(|cc| Box::new(G4PatcherApp::new(cc))),
        )?;
    } else {
        run_cli(args.rom_folder, args.patch_file)?;
    }
    Ok(())
}