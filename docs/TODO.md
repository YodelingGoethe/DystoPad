# REWRITE:
The project needs to be rewritten completely in Rust, not typescript and tauri. rename the new Project DystoPad globally, create its own github repo, I'll delete the old one. Don't fix the current repo, instead create a new one in which you'll implement the changes and bugfixes below. create a new Folder on the same level as DystoBlocks, DystoPoly and Dystonotepad. once everything is fully converted, move the relevant docs to the new folder (DystoPad) and delete the old folder (Dystonotepad). also recreate a new icon. instead of "N", the new icon should have ".Md" displayed on it, in the same format.

# Bugs
- when writing a bulleted list, if i press enter to start a new list entry and then tab to indent it, it temporarily becomes a heading in the preview. this resolves itself if i start to type
- the styling for code blocks is incomplete. you'll find the jpeg images code styling shouldbe and code styling current for references. make sure the github styling is replicated perfectly

# general changes:
- the tab names for unsaved files are currently "Untitled". if a file is not saved, I want the title to be the first few words of the actual text, like how it is in notepad.exe. see tabnames.jpeg for references
- the bar between preview and editor in split view should be movable to resize the individual windows. 
- the dot in the tab to signal unsaved changes should not be yellow, it should be blue like the rest. remember that this app should be minimalistic
- give it a gpl license