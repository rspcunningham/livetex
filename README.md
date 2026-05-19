# LiveTex

Handy CLI tool for live latex editing

## Requirements

- latexmk (`brew install latexmk`)
- Skim (`brew install skim`)

## Installation

```
git clone https://github.com/rspcunningham/livetex
cd livetex
cargo install --path .

defaults write net.sourceforge.skim-app.skim SKAutoCheckFileUpdate -bool true
defaults write net.sourceforge.skim-app.skim SKAutoReloadFileUpdate -bool true
```

## Usage

```
livetex start <path_to_latex_file>
# starts a live preview of the latex file

livetex logs <path_to_latex_file> --lines <num_lines>
# shows the logs for the live preview of the latex file

livetex stop <path_to_latex_file>
# stops the live preview of the latex file

livetex list
# lists all running live previews

livetex export <path_to_latex_file>
# exports the latex file to pdf in the same directory as the .tex
```

## Next steps
- [ ] flow to select a running preview to stop without specifying the path
- [ ] last-turn-only log parsing
- [ ] right click on file to open
- [ ] kill the compile when Skim window is closed
- [ ] use AppleScript to control Skim windows directly
- [ ] installation flow with prerequisites checking and Skim default setup
