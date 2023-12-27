# Where should we write the GIF?
Output ./doc/preview.gif

# Set up a 1200x600 terminal with 46px font.
Set FontSize 14
Set Width 1200
Set Height 800
Set TypingSpeed 100ms

# Launch doggy
Type "doggy"

Enter

Sleep 1s

# -- Show help screen

Type "?"
Sleep 4s

Escape
Sleep 800ms

# -- Show exec
Type "j"
Sleep 1s

Type "S"
Sleep 1s
Type "/bin/sh"

Sleep 800ms
Enter

Sleep 1s

Type "ls"
Enter

Sleep 2s

Type "ps"
Enter

Sleep 3s
Ctrl+d
Sleep 1s

# -- Show display all containers
Type "a"
Sleep 400ms

Type "a"
Sleep 3s

Type "a"
Sleep 1s

# -- Show delete running container
Type "j"
Sleep 500ms
Type "j"
Sleep 500ms

Ctrl+d
Sleep 1s
Enter
Sleep 2s

Sleep 500ms
Type "j"

# -- Show inspect container
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

Sleep 800ms

# -- Show display logs

Type "j"
Sleep 800ms

Type "j"
Sleep 800ms

Type "l"
Sleep 8s

Escape

# -- Show change resource/autocomplete
Type ":"
Sleep 1s
Type "im"

Sleep 1s
Enter

Type "j"
Sleep 1s
Type "j"
Sleep 1s

# -- Show error notification

Ctrl+d
Sleep 1s

Enter
Sleep 3s

Escape
Sleep 800ms

# -- Show sorting

F3
Sleep 4s
F3
Sleep 2s

# -- Filter image names

Type "/"
Sleep 800ms

Type "ubun"
Sleep 800ms
Enter
Sleep 2s

Type "/"
Sleep 400ms
Enter
Sleep 1s

# -- Show go back to containers
Type ":"
Sleep 1s

Type "cont"
Sleep 1s

Type "q"
