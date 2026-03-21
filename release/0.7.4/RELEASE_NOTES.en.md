# ZSClip 0.7.4

This release focuses on a group-menu crash fix and list layout/scaling corrections under high DPI.

## Highlights

### 1. Fixed group menu crash
- Fixed the issue where right-clicking on the `Phrases` tab after creating a group could close the app
- The tab group filter menu no longer switches the active tab before a command is actually chosen
- The selected tab/filter is only applied after the user picks a filter item
- Removed the side effect that forced focus back to search after closing the group menu

### 2. Fixed scaled list icon/layout issues
- Unified left-side row icons, pin icons, quick action buttons, and text padding under the main layout calculation
- Removed several fixed size caps that did not scale correctly with DPI
- Improved icon size, position, and text spacing consistency under different scaling ratios

### 3. Local release packaging
- Version updated to `0.7.4`
- Chinese and English README release links updated to `0.7.4`
- Local `release` directory refreshed for the `0.7.4` build
