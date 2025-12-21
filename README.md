# rsync_tree

A command-line tool for visualizing rsync included and excluded files in a tree-like structure. This tool helps you understand exactly which files and directories will be synchronized by rsync before running the actual sync operation.

Please read the [Current Limitations & Known Issues](#current-limitations--known-issues) section before using this tool.

## Overview

`rsync_tree` takes an rsync command as input, automatically adds the `--dry-run` and `--itemize-changes` flags, executes it, parses the output, compares it to files on the filesystem, and presents the results as a hierarchical tree showing what will be saved (included) and what will be excluded. The tool also calculates and displays file sizes and directory sizes with percentages to help you understand the data distribution.

### Use Cases

This tool is particularly useful for:

- **Backup Analysis**: Understanding what your backup includes and excludes before committing to potentially large sync operations
- **Include/Exclude Rule Validation**: Visualizing the effect of complex `--include` and `--exclude` patterns to ensure they work as intended
- **rsync-based Backup Tools**: Can be used with tools like [Back In Time](https://github.com/bit-team/backintime), [rsnapshot](https://rsnapshot.org/), or any other rsync-based backup solution
- **Debugging rsync Behavior**: When rsync's behavior seems unexpected, the tree view helps identify what's actually being transferred

### Design Philosophy

Rather than reimplementing rsync's complex pattern matching and decision logic, `rsync_tree` directly parses rsync's own `--itemize-changes` output. This design ensures the tool is as faithful as possible to rsync's actual behavior, including:

- Complex interactions between multiple `--include` and `--exclude` rules
- Order-dependent pattern matching
- Anchor patterns, wildcards, and directory-specific rules
- Edge cases in rsync's filter rule processing

By letting rsync itself determine what will be transferred, the tool guarantees accuracy even with the most intricate rule sets.

## Features

- **Automatic Dry Run**: Automatically adds `--dry-run` flag to prevent actual file transfers
- **Tree Visualization**: Displays rsync operations as an intuitive tree structure
- **Size Analysis**: Shows file and directory sizes with percentage breakdowns
- **Color-Coded Output**: Green for included files/directories, red for excluded ones
- **Collapsible View**: Option to collapse directories that are entirely included or excluded

## Installation

### From Source

```bash
cargo build --release
```

The binary will be available at `target/release/rsync_tree`. You will need rust and rsync installed on your system.

## Usage

### Basic Usage

```bash
rsync_tree "rsync -av /source/ /destination/"
```

### Command-Line Options

```
Options:
  -b, --base-path <BASE_PATH>
          Base path for tree construction (defaults to current directory)

  -c, --color
          Enable colored tree output [default: true]

  -C, --collapse
          Collapse directories that are entirely included or excluded [default: true]

  -D, --debug
          Show debug information in tree output

  -S, --show-sizes
          Collect and display file/folder sizes [default: true]

  --log-level <LOG_LEVEL>
          Set logging level (error, warn, info, debug, trace) [default: info]

  -s, --save-output <SAVE_OUTPUT>
          Save rsync output to specified file

  -h, --help
          Print help

  -V, --version
          Print version
```

## Sample Output

```
/ (85.3 GB)
├── bin
├── boot
├── dev
├── etc
├── home (85.3 GB 100.0%)
│   ├── user (85.3 GB 100.0%)
│   │   ├── .bash_history (1.2 KB 0.0%)
│   │   ├── .bashrc (4.1 KB 0.0%)
│   │   ├── .cache (0 B 0.0%)
│   │   ├── .config (42.5 MB 0.0%)
│   │   │   ├── Code (38.2 MB 89.9%)
│   │   │   ├── gtk-3.0 (1.1 KB 0.0%)
│   │   │   └── systemd (4.3 MB 10.1%)
│   │   ├── .local (2.3 GB 2.7%)
│   │   │   ├── bin (1.8 GB 78.3%)
│   │   │   ├── lib (456.7 MB 19.4%)
│   │   │   └── share (52.1 MB 2.3%)
│   │   ├── Documents (52.4 GB 61.4%)
│   │   │   ├── projects (48.2 GB 92.0%)
│   │   │   ├── notes (3.8 GB 7.3%)
│   │   │   └── archives (412.5 MB 0.7%)
│   │   ├── Downloads (8.9 GB 10.4%)
│   │   ├── Pictures (15.2 GB 17.8%)
│   │   ├── Videos (6.5 GB 7.6%)
│   │   ├── node_modules (0 B 0.0%)
│   │   └── tmp (0 B 0.0%)
│   └── shared (12.3 MB 0.0%)
├── lib
├── media
├── opt
├── proc
├── sys
└── var
```

With colors enabled, the output looks like this:

- <span style="color: #00ff00">**Green items**</span> (e.g., `.bashrc`, `Documents`) are **included** in the sync
- <span style="color: #ff0000">**Red items**</span> (e.g., `.cache`, `node_modules`, `tmp`) are **excluded** from the sync
- **White/bold items** (e.g., `user`, `.local`) are directories with **mixed** included/excluded content
- **Sizes in parentheses** show the total size and percentage of parent directory
- **Percentages** are color-coded from dark gray (0-5%) through white, yellow, to bright red (80-100%)

## How It Works

1. **Command Parsing**: The tool parses your rsync command and extracts the arguments
2. **Flag Injection**: Automatically adds `--dry-run` and `--itemize-changes` flags
   - Removes conflicting flags like `--out-format` and `-v`
3. **Execution**: Runs the modified rsync command and captures the output
4. **Parsing**: Parses rsync's itemize-changes output format (11-character codes + paths)
5. **Tree Construction**:
   - Scans the filesystem starting from the base path
   - Marks each file/directory as included or excluded based on rsync output
   - Computes directory statuses (included, excluded, mixed, or standalone)
6. **Size Calculation**: Recursively computes sizes for included files and directories
7. **Rendering**: Displays the tree with ASCII art, colors, sizes, and percentages

## Current Limitations & Known Issues

⚠️ **This is a work in progress!** Always make sure you understand what rsync command will be executed before using this tool. Incorrect usage or bugs may lead to data loss or unexpected behavior. I make no guarantees about the correctness or safety of this tool.

 Known limitations:

- Limited testing with edge cases and complex rsync scenarios
- Size calculation requires filesystem access (may fail for restricted directories)
- Some rsync flags and output formats may not be fully supported
- Path computation should be improved to chose in which directory to run rsync
- Error handling could be more comprehensive
- The tool assumes local filesystem paths (remote paths may not work correctly)

## Contributing

Contributions are very welcome! Areas that need improvement:

- [ ] More comprehensive testing
- [ ] Better error handling and user feedback
- [ ] Support for remote rsync paths
- [ ] Performance optimization for large trees
- [ ] Additional output formats (JSON, HTML)
- [ ] Progress indicators for long operations
- [ ] Better handling of symlinks and hard links
- [ ] ... and more!

## License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

This tool is based on [rsync](https://github.com/RsyncProject/rsync).
