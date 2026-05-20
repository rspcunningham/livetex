# LiveTex

Handy CLI tool for live latex editing

## Requirements

- TeX Live (`brew install texlive`)
- Skim (`brew install --cask skim`)

## Installation

```
git clone https://github.com/rspcunningham/livetex
cd livetex
./install.sh
# or install Homebrew dependencies too:
./install.sh --with-deps
```

## Usage

```
livetex start <path_to_latex_file>
# starts a live preview of the latex file

livetex logs <path_to_latex_file>
# shows logs for the most recent compile turn

livetex logs <path_to_latex_file> --turns <num_turns>
# shows logs for the past num_turns compiles

livetex stop [path_to_latex_file]
# stops the live preview of the latex file, or opens a selector when no path is given

livetex list
# lists all running live previews

livetex manage
# opens one workflow for list, export, logs, and stop

livetex export [path_to_latex_file]
# exports the latex file to pdf, or opens a selector when no path is given

livetex doctor
# checks prerequisites and Skim defaults

livetex setup
# sets Skim defaults

livetex --verbose <command>
# shows session IDs, process IDs, cache directories, PDF paths, and log paths
```

## Next steps
- [x] use arrow keys to navigate the list of running previews and select one to stop
- [x] last-turn-only log parsing
- [ ] right click on file to start a new preview session with it
- [ ] kill the compile when Skim window is closed
- [ ] use AppleScript to control Skim windows directly
- [x] installation flow with prerequisite fixing and Skim default setup
- [x] use the same interactive menu in 'export' as in 'stop'
- [x] homogenize the cli into one workflow supporting list, stop, and export -- and maybe start
