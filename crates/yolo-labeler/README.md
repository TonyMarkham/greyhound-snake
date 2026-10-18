# yolo-labeler

A basic YOLO-compatible image labeling tool built with
[GPUI](https://gpui.rs) (Zed's GPU-accelerated UI framework, crates.io
release `0.2.2`).

Open a folder of images, cycle through them, view the YOLO labels that
already exist, and draw new boxes. Two folder layouts are recognized:

- **Ultralytics dataset run** — the folder has an `images/` child:
  frames come from `images/`, labels from the sibling `labels/`
  (paired by stem, Ultralytics' own path substitution), and class
  names come from the tree's `data.yaml` one directory up.
- **Flat folder** — images sit beside their `<image>.txt` labels
  (`class cx cy w h`, normalized to `0..1`), with one shared
  `classes.txt` (one class name per line).

## Build and run

```sh
cargo run -p yolo-labeler -- <dataset run folder or image folder>
```

The folder argument is optional; without it the app opens a native folder
picker at startup (Ctrl-O reopens it later).

## Controls

| Input                  | Action                                                    |
| ---------------------- | --------------------------------------------------------- |
| Left drag on image     | Draw a box with the active class                          |
| Click a box            | Select it                                                 |
| Drag a selected box    | Move it                                                   |
| Drag a selected box's edge / corner | Resize it (white corner handles mark the zones) |
| Scroll wheel           | Zoom around the cursor (1×–8×)                            |
| Middle drag            | Pan when zoomed                                           |
| `1`–`9`                | Set active class (re-classes the selected box too)        |
| `Tab`                  | Cycle the active class (classes beyond 9 included)        |
| `Delete` / `Backspace` | Delete the selected box                                   |
| `Esc`                  | Deselect / cancel the current drag                        |
| `Right` / `Left`       | Next / previous image (wraps around)                      |
| `Ctrl-S`               | Save the current image's labels                           |
| `Ctrl-O`               | Open another folder                                       |
| `Ctrl-0`               | Reset zoom and pan                                        |

Toolbar buttons mirror the actions: Open Folder, Prev, Next, Save,
Reset View.

## Classes

The status bar shows one chip per class; the white-outlined chip is the
active class. `+ class` opens an inline input: type the name, `Enter`
adds it (persisted immediately and selected as the active class), `Esc`
cancels.

Class-name source: the dataset tree's `data.yaml` (one directory above
the target folder, wherever that is), `names:` key — list or mapping
style. **While that yaml exists, `classes.txt` is never written** —
renames via the right-click menu write back to the yaml's `names:` as
a list (other keys preserved; comment formatting is not), and `+ class`
is session-only. If the yaml has no usable `names:` yet, the chip list
fills with `class N` placeholders from the label scan, in memory only.

Flat folders with no `data.yaml` one directory up keep the
`classes.txt` convention: renames and additions persist there, the
file is created with a single `object` class if missing or empty, and
a scan that finds higher class ids extends the persisted list.

When a folder is opened, the label files are scanned for the highest
class id in use; if the annotations reference ids beyond the class list
(e.g. labels written by an external tool), the list is extended with
`class N` placeholders and shown — persisted to `classes.txt` only when
no `data.yaml` sits one directory up.

Class chips also have a right-click context menu with `Rename`: the
chip turns into an inline editor prefilled with the current name —
`Enter` commits (persisted immediately), `Esc` cancels. Click anywhere
else or `Esc` closes the menu.

## Files and conventions

- **Flat layout:** `<image>.txt` beside the image. **Ultralytics
  layout:** `labels/<stem>.txt` beside the `images/` folder. YOLO
  detection format, six decimals.
- **Flat layout:** `classes.txt` beside the images. **Ultralytics
  layout:** the tree's `data.yaml` one directory up. Class ids
  beyond the palette length cycle through the 10 display colors.
- Scanned extensions: `bmp gif jpg jpeg png tif tiff webp`.
- Empty label files are legitimate (a background frame); per the
  dataset format's never-wipe rule, the editor refuses to save a
  wholesale emptying of a label file that has lines — the status line
  says so and the file is left untouched.

Labels are written when you navigate away from a changed image or press
`Ctrl-S`/the Save button. Plain viewing never touches the folder — no
label file is created for an image you did not edit. The status line
shows the live box count and an `unsaved` marker whenever the current
image has changes in memory.

## Known limits (v2)

- The rename/add-class inputs are append/backspace only (no caret
  repositioning or text selection).
- Dragging past the image edge pauses the drag until the cursor returns.
- Class deletion and re-mapping existing label ids is not supported.

