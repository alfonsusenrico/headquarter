# Design: an empty folder field means the home folder

## D1. Empty is home

The field starts empty with the placeholder "Home folder, or type a path or a folder name". The
folder browser treats empty text as `~/` (the same list and "Open here" row), and the chosen folder
for an empty field is the home folder. Tab completion from empty completes to `~/` plus the
highlighted subfolder, as today from `~/`.

Rejected: keeping `~/` selected so typing replaces it. It still shows a value the owner did not
type, and a click into the field deselects it.
