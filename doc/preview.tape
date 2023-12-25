# Where should we write the GIF?
Output ./doc/preview.gif

# Set up a 1200x600 terminal with 46px font.
Set FontSize 16
Set Width 1200
Set Height 600
Set TypingSpeed 100ms

# Type a command in the terminal.
Type "./target/release/doggy"
# Type "./target/x86_64-unknown-linux-gnu/release/doggy"

Enter

# Pause for dramatic effect...
Sleep 500ms

# Show exec
Type "j"

Sleep 400ms
Ctrl+S
Sleep 1s
Type "/bin/sh"

Sleep 300ms
Enter

Sleep 300ms

Type "ls"
Enter

Sleep 300ms

Type "ps"
Enter

Sleep 300ms
Ctrl+d
Sleep 1s

# Run the command by pressing enter.
Type "a"
Sleep 400ms
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
#Ctrl+d
#Sleep 1s
#Enter
#Sleep 2s
Type "i"
Sleep 1s
PageDown
Sleep 1s
PageDown
Sleep 1s
PageDown
Sleep 2s
PageUp
Sleep 2s

Escape

Type ":"
Sleep 500ms
Type "im"

Sleep 1s

Enter

Type "j"
Sleep 1s
Type "j"
PageDown

# Admire the output for a bit.
Sleep 2s

Type ":"
Sleep 1s

Type "cont"
Sleep 1s

Type "q"
