use grass::{Options, OutputStyle};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Result, Watcher};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;

fn main() -> Result<()> {
    // Input and output directories
    let input_dir = "./sass";
    let output_dir = "./css";

    // Ensure the output directory exists
    std::fs::create_dir_all(output_dir).expect("Failed to create output directory");

    // Start watching for changes in the input directory
    watch_sass_files(input_dir, output_dir)?;

    Ok(())
}

fn watch_sass_files(input_dir: &str, output_dir: &str) -> Result<()> {
    // Create a channel to receive file events
    let (tx, rx) = channel();

    // Create a watcher with default configuration
    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event>| {
            if let Ok(event) = res {
                // Send the event to the channel
                tx.send(event).unwrap();
            }
        },
        Config::default(),
    )?;

    let path = Path::new(input_dir);

    // Add the input directory to the watcher
    watcher.watch(path, RecursiveMode::Recursive)?;

    println!("Watching for changes in: {}", input_dir);

    // Event loop to process changes
    for event in rx {
        if let EventKind::Modify(_) | EventKind::Create(_) = event.kind {
            if let Some(path) = event.paths.get(0) {
                // Check if the file has the .scss extension and is not a partial
                if path.extension().and_then(|ext| ext.to_str()) == Some("scss") {
                    // Exclude partials (files starting with "_")
                    if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                        if file_name.starts_with('_') {
                            println!("Skipping partial file: {:?}", path);
                            continue; // Skip processing this file
                        }
                    }

                    // Handle the SCSS change if it's not a partial
                    handle_sass_change(path, input_dir, output_dir);
                }
            }
        }
    }

    Ok(())
}

fn handle_sass_change(path: &Path, input_dir: &str, output_dir: &str) {
    // Normalize input directory to an absolute path
    let input_dir = match fs::canonicalize(input_dir) {
        Ok(abs_path) => abs_path,
        Err(e) => {
            eprintln!("Failed to resolve input directory {:?}: {}", input_dir, e);
            return;
        }
    };

    // Normalize the path being processed
    let path = match fs::canonicalize(path) {
        Ok(abs_path) => abs_path,
        Err(e) => {
            eprintln!("Failed to resolve file path {:?}: {}", path, e);
            return;
        }
    };

    // Get the relative path of the changed file
    let relative_path = match path.strip_prefix(&input_dir) {
        Ok(rel_path) => rel_path,
        Err(_) => {
            eprintln!(
                "File {:?} is not under input directory {:?}",
                path, input_dir
            );
            return;
        }
    };

    // Determine the output CSS file path
    let mut output_path = PathBuf::from(output_dir);
    output_path.push(relative_path); // Append the relative path
    output_path.set_extension("css"); // Change the extension to .css

    println!("Compiling {:?} to {:?}", &path, output_path);

    let options = Options::default();
    // Compile the SCSS file to CSS
    match grass::from_path(&path, &options.style(OutputStyle::Compressed)) {
        Ok(css) => {
            // Ensure the parent directory of the output path exists
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)
                    .expect("Failed to create output directory structure");
            }

            // Write the compiled CSS to the output file
            std::fs::write(&output_path, css).expect("Failed to write CSS file");

            println!("Successfully compiled: {:?}", output_path);
        }
        Err(e) => eprintln!("Failed to compile {:?}: {}", path, e),
    }
}
