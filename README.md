# rsync_tree

Visualize exactly which files rsync will include or exclude — before running the actual sync.

`rsync_tree` reads rsync's `--itemize-changes` output from **stdin** (piped by the user), parses it, and presents a color-coded hierarchical tree of what will be transferred. It can also **compare multiple saved snapshots** to highlight differences between backups.

Please read the [Current Limitations & Known Issues](#current-limitations--known-issues) before using.

## Features

- **Pipe-friendly** — reads rsync `--itemize-changes` directly from stdin
- **Tree visualization** — green (included), red (excluded), gray (missing), bold white (mixed)
- **Size analysis** — file/directory sizes with percentage of parent
- **Collapsible view** — compress fully-included or fully-excluded subtrees
- **Compare mode** — diff multiple saved trees side-by-side to see what changed between backups
- **Save / Load** — serialize trees to JSON for later inspection or comparison
- **Progress feedback** — line count and parse warnings printed to stderr during processing

## Design Philosophy

Rather than reimplementing rsync's pattern matching, `rsync_tree` parses rsync's own `--itemize-changes` output. This guarantees accuracy even with complex nested `--include`/`--exclude` rules and order-dependent matching.

## Installation

```bash
cargo build --release
# binary at target/release/rsync_tree
```

Requires Rust.

## Usage

`rsync_tree` reads rsync's `--itemize-changes` output from **stdin**. Pipe the output of your rsync dry-run into the tool.

**Important:** The `--base-path` / `-b` option is **required** for the `build` subcommand. It tells `rsync_tree` where the source directory lives on your filesystem so it can build the tree correctly. This is necessary because rsync's output only shows relative paths.

### Build mode

```bash
rsync -av --dry-run --itemize-changes /source/ /destination/ | rsync_tree build --base-path /source/
```

To save the raw rsync output while also piping to rsync_tree, use `tee`:

```bash
rsync -av --dry-run --itemize-changes /source/ /destination/ | tee output.txt | rsync_tree build --base-path /source/
```

### Save a tree snapshot for later comparison

```bash
rsync -av --dry-run --itemize-changes /source/ /destination/ | rsync_tree build --base-path /source/ --save-tree backup-week1.json
```

### Load a previously saved tree (skip running rsync)

```bash
rsync_tree build --load-tree backup-week1.json
```

### Compare two or more saved trees

```bash
rsync_tree compare backup-week1.json backup-week2.json
```

Only nodes that differ between trees are shown. When trees disagree, labels (the JSON filenames) appear on the left colored by that tree's status for the node. Gray labels indicate the file/directory was **missing** from that snapshot.

### All options

#### `rsync_tree build`

```
Options:
  -b, --base-path <BASE_PATH>
          Base path for tree construction (REQUIRED)

  -c, --color
          Enable colored output [default: true]

  -C, --collapse
          Collapse directories entirely included or excluded [default: true]

  -D, --debug
          Show node status debug info (FI/FE/DI/DE/DM)

  -S, --show-sizes
          Collect and display file/folder sizes [default: true]

  --log-level <LOG_LEVEL>
          Logging level (error, warn, info, debug, trace) [default: info]

  --save-tree <PATH>
          Serialize the constructed tree to a JSON file

  --load-tree <PATH>
          Load a tree from a JSON file instead of running rsync

  -h, --help
  -V, --version
```

#### `rsync_tree compare`

```
Usage: rsync_tree compare [OPTIONS] <PATHS>...

Arguments:
  <PATHS>...  Paths to saved tree JSON files (at least 2 required)

Options:
  -c, --color     Enable colored output [default: true]
  -D, --debug     Show debug information in tree output
  --log-level <LOG_LEVEL>
          Logging level (error, warn, info, debug, trace) [default: info]
  --ignore-size   Ignore file sizes when comparing trees
  -h, --help
  -V, --version
```

## Sample Output

<pre>
<strong>/</strong> (85.3 GB)
├── <strong style="color:red">bin</strong>
├── <strong style="color:red">boot</strong>
├── <strong style="color:red">dev</strong>
├── <strong style="color:red">etc</strong>
├── <strong>home</strong> (85.3 GB <span style="color:red">100.0%</span>)
│   ├── <strong>user</strong> (85.3 GB <span style="color:red">100.0%</span>)
│   │   ├── <span style="color:green">.bash_history</span> (1.2 KB <span style="color:gray">0.0%</span>)
│   │   ├── <span style="color:green">.bashrc</span> (4.1 KB <span style="color:gray">0.0%</span>)
│   │   ├── <strong style="color:red">.cache</strong>
│   │   ├── <strong style="color:green">.config</strong> (42.5 MB <span style="color:gray">0.0%</span>)
│   │   │   ├── <strong style="color:green">Code</strong> (38.2 MB <span style="color:red">89.9%</span>)
│   │   │   ├── <span style="color:green">gtk-3.0</span> (1.1 KB <span style="color:gray">0.0%</span>)
│   │   │   └── <strong style="color:green">systemd</strong> (4.3 MB 10.1%)
│   │   ├── <strong style="color:green">Documents</strong> (52.4 GB <span style="color:red">61.4%</span>)
│   │   └── <span style="color:gray">(2 items)</span>
│   └── <strong style="color:green">shared</strong> (12.3 MB <span style="color:gray">0.0%</span>)
└── <strong style="color:red">var</strong>
</pre>

- <span style="color:green">Green</span> — included
- <span style="color:red">Red</span> — excluded
- White — mixed (some children included, some excluded)
- **Bold** — directory

**Note**: colors rendering depends on your markdown viewer.

## Current Limitations & Known Issues

⚠️ **This is a work in progress!** Always verify what rsync will do before relying on this tool.

- Limited testing with edge cases and complex rsync scenarios
- Size calculation requires filesystem access (may fail for restricted directories)
- Remote paths may not work correctly
- Error handling could be more comprehensive

## Contributing

Contributions welcome! Areas needing improvement:

- [ ] More comprehensive testing
- [ ] Better error handling
- [ ] Remote rsync path support
- [ ] Performance optimization for large trees
- [ ] Additional output formats (JSON, HTML)
- [ ] Progress indicators
- [ ] Symlink and hard link handling

## License

GNU General Public License v3.0 — see [LICENSE](LICENSE).

## Acknowledgments

Based on [rsync](https://github.com/RsyncProject/rsync).
