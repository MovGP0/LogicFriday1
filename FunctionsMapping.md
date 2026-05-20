# LogicFriday Decompiled Function Mapping

This is a first-pass map of `Decompiled/logicfriday_decompiled.c`. The names are inferred from call sites, literals, Win32 APIs, and data flow; they are not original symbols. The mapping starts at the lowest system-call-facing functions and works upward toward command handlers and application behavior.

## Confidence Legend

- High: behavior is directly evidenced by API calls, strings, file formats, or command IDs.
- Medium: behavior is clear, but exact user-facing name or object type is inferred from surrounding flow.
- Low: only partial behavior is known.

## System Call and External Boundary Layer

| Function | Inferred responsibility | Evidence | Confidence |
| --- | --- | --- | --- |
| `FUN_0043e408` | Locate the installed HTML Help ActiveX control path from the registry. | Calls `RegOpenKeyExA(HKEY_CLASSES_ROOT, CLSID\\{ADB880A6-D8FF-11CF-9377-00AA003B7A11}\\InprocServer32)`, reads default value with `RegQueryValueExA`, closes key. | High |
| `FUN_0043e36f` | Lazy-load and dispatch to `HtmlHelpA`/HTML Help control. | Loads path returned by `FUN_0043e408`, falls back to `LoadLibraryA("hhctrl.ocx")`, gets ordinal `0xe`, invokes cached function pointer. | High |
| `FUN_004013e5` | Open a hyperlink or email address with the shell and mark it visited. | Uses `FUN_004010ba(..., 1)` to get `mailto:` or web URL, calls `ShellExecuteA(..., "open", ...)`, invalidates the link control. | High |
| `FUN_0040c196` | Run Espresso or MISII asynchronously with redirected output and cancellation support. | Builds `%s\\espresso\\espresso.exe` or `%s\\misii\\misii.exe`, creates `%s\\minout.dat`, starts process with inherited stdout/stderr, waits on both cancel event and process, posts completion message `0x8004`/`0x8006`. | High |
| `FUN_0040c55b` | Run Espresso or MISII synchronously, redirecting output to `minout.dat`. | Same executable selection as `FUN_0040c196`, calls `CreateProcessA`, waits up to 3,600,000 ms, sends `WM_COMMAND 0x8004`/`0x8006`. | High |
| `FUN_0040c7c0` | Run MISII synchronously for `checkout.dat` generation. | Uses `%s\\misii\\misii.exe`, redirects stdout/stderr to `%s\\checkout.dat`, sends `WM_COMMAND 0x8021`. | High |
| `FUN_0040e929` | Initialize paths, temp/work files, file dialogs, and verify bundled tools. | Uses `GetModuleFileNameA`, `SHGetSpecialFolderPathA`, `CreateDirectoryA`, sets `lf.ini`, `minin.dat`, `minout.dat`, `checkin.dat`, `checkout.dat`, `user.genlib`, checks `espresso.exe`, `misii.exe`, and `script.txt` with `SearchPathA`. | High |
| `FUN_00415364` | Load persisted window placement, folder paths, and warning flags from `lf.ini`. | Uses `GetPrivateProfileStructA`, `GetPrivateProfileStringA`, and `GetPrivateProfileIntA`. | High |
| `FUN_00415663` | Save window placement, folder paths, and warning flags to `lf.ini`. | Uses `WritePrivateProfileStructA` and `WritePrivateProfileStringA`. | High |
| `FUN_004158aa` | Relax security on generated temp files when using the temporary working directory. | Creates security descriptor and calls `SetFileSecurityA` for `minin.dat`, `minout.dat`, and `user.genlib`. | Medium |
| `FUN_0041509f` | Copy current gate diagram to the clipboard as enhanced metafile and bitmap. | Calls `CopyEnhMetaFileA`, `OpenClipboard`, `EmptyClipboard`, `SetClipboardData(CF_ENHMETAFILE)`, then renders bitmap with `PlayEnhMetaFile`. | High |
| `FUN_004151da` | Copy another generated metafile view to the clipboard as enhanced metafile and optional bitmap. | Creates EMF via `FUN_00436925`, renders bitmap from `GetEnhMetaFileHeader`, writes both clipboard formats. | Medium |
| `FUN_00410780` | Export gate diagram image to `.emf` or `.bmp`. | For `.emf`, calls `CopyEnhMetaFileA`; otherwise renders the EMF to a 1-bit DIB and writes BMP headers/data via `_fwrite`. | High |

## Win32 UI Shell

| Function | Inferred responsibility | Evidence | Confidence |
| --- | --- | --- | --- |
| `FUN_0040145f` | Application entry/main message loop. | Registers main window plus `DiagOutWindow` and `DiagInWindow`, loads RichEdit DLL, creates main window, loads accelerators, runs `GetMessageA`/`TranslateMessage`/`DispatchMessageA`. | High |
| `FUN_004016d2` | Main window procedure and central command dispatcher. | Registered as main class wndproc; handles many `WM_COMMAND` IDs, opens dialogs, launches map/minimize/import/export/save flows, calls `DefWindowProcA` fallback. | High |
| `FUN_00401133` | Subclass procedure for hyperlink-like static controls. | Handles `WM_SETCURSOR` with hand cursor, otherwise forwards to previous wndproc from `FUN_00401078`. | High |
| `FUN_0040117c` | Initialize about-dialog hyperlink controls. | Gets dialog items from link table, creates underlined font, sizes controls, subclasses each with `FUN_00401133`. | High |
| `FUN_0040133c` | Owner-draw the hyperlink controls. | Selects visited/unvisited colors, draws URL/mail text with `ExtTextOutA`. | High |
| `FUN_00401445` | Release hyperlink font resource. | Deletes cached font object. | High |
| `FUN_0040c9df` | Check or uncheck a menu item. | Thin wrapper over `CheckMenuItem`. | High |
| `FUN_0040ca01` | Enable or disable a menu item. | Thin wrapper over `EnableMenuItem`. | High |
| `FUN_0040cd0c` | Create and populate the toolbar. | Creates `ToolbarWindow32`, assigns image lists, adds buttons via toolbar messages. | High |
| `FUN_0040b004` | `DiagOutWindow` window procedure. | Registered under `"DiagOutWindow"` by `FUN_0040145f`; receives application-defined command messages from the main dispatcher. | Medium |
| `FUN_0040afc3` | `DiagInWindow` window procedure. | Registered under `"DiagInWindow"` by `FUN_0040145f`; used for diagram-entry interaction. | Medium |

## File Dialog and Path Helpers

| Function | Inferred responsibility | Evidence | Confidence |
| --- | --- | --- | --- |
| `FUN_004102f4` | Show Open dialog for Logic function files. | Filter string `"Logic function (*.lfcn)"`, uses `GetOpenFileNameA`, stores selected open directory. | High |
| `FUN_004103b5` | Show Save dialog for Logic function files. | Filter string `"Logic function (*.lfcn)"`, uses `GetSaveFileNameA`, stores selected save directory. | High |
| `FUN_00410476` | Show Save dialog for C source export. | Filter string `"C Source File (*.c)"`, uses `GetSaveFileNameA`. | High |
| `FUN_00410538` | Show Save dialog for enhanced metafile export. | Filter string `"Enhanced Metafile (*.emf)"`, uses `GetSaveFileNameA`. | High |
| `FUN_004105fb` | Show Open dialog for arbitrary files. | Filter string `"All Files (*.*)"`, uses `GetOpenFileNameA`. | High |
| `FUN_004106be` | Show Save dialog for CSV truth-table export. | Filter string `"Comma Separated Variables (*.csv)"`, uses `GetSaveFileNameA`. | High |
| `FUN_00415864` | Extract application directory from module path. | Finds final backslash; copies directory prefix or defaults to empty string. | High |
| `FUN_0040e68a` | Prepare or validate a file operation caption/path context. | Called before `"File Open"` and other file workflows; exact UI side effect not fully mapped. | Low |

## Project Load, Save, and Import/Export

| Function | Inferred responsibility | Evidence | Confidence |
| --- | --- | --- | --- |
| `FUN_00412219` | Save a Logic Friday `.lfcn` function file. | Opens output as `wb`, writes `"LTK1"`-style header/version data, fixed 0x2700-byte function state, truth-table arrays, text fields, mapping/gate data, optional EMF bits, then marks function clean. | High |
| `FUN_00412923` | Load a Logic Friday `.lfcn` function file. | Opens input as `rb`, verifies `"LTK1"`, checks stored version against `DAT_004519e4`, reads 0x2700-byte base state, allocates truth-table and mapping buffers, restores optional EMF via `SetEnhMetaFileBits`. | High |
| `FUN_00410a47` | Export truth table to CSV/text. | Opens output as `wt`, writes input/output names and rows, emits `0`, `1`, and `X` values, supports both full truth table and reduced/lookup table representation. | High |
| `FUN_004110a2` | Import CSV/text truth table. | Opens input as `rt`, skips comments/blank lines, parses names and rows, validates input/output counts, allocates output arrays, fills truth table. | High |
| `FUN_004116b2` | Parse one truth-table data row into input and output token strings. | Splits on comma/tab, accepts only `0`, `1`, `X`/`x`, normalizes lowercase `x`. | High |
| `FUN_00411871` | Parse and validate CSV variable-name header row. | Splits input/output names, enforces length limits, validates characters through `FUN_0040daf0`, rejects duplicate input/output names. | High |
| `FUN_00411caa` | Apply one parsed truth-table row to the in-memory table, expanding input don't-cares. | Converts input bits to row index; if input contains `X`, enumerates all matching rows; delegates cell assignment to `FUN_00411dfc`. | High |
| `FUN_00411dfc` | Assign output values for one truth-table row and detect conflicts. | Writes `1`, `0`, or `2` (`X`) into output arrays; increments true/don't-care counts; returns conflict code `10` if row already has different value. | High |
| `FUN_00411f69` | Show import error dialogs. | Maps import error codes 1-10 to `"Import Error"` message boxes. | High |
| `FUN_0041338b` | Destructor/deallocator for a vector of 0xfc-byte objects. | Calls `_eh_vector_destructor_iterator_` and `_free` depending on flags. | Medium |
| `FUN_0041c785` | Construct/initialize a function model object. | Allocates buffers for expressions, truth-table arrays, diagram data, labels such as `"Entered:"`, `"Minimized:"`, default capacities. | High |
| `FUN_0041cbd4` | Clone a function model object. | Allocates new buffers and copies arrays, strings, truth table, and diagram/metafile-related members from another function. | High |
| `FUN_0041d5e1` / `FUN_0041d8f2` / `FUN_0041d91b` / `FUN_0041d944` | Function model cleanup/destruction and reset/update helpers. | Nearby code frees truth-table arrays, expression strings, diagram object vectors, and refreshes UI via messages. | Medium |

## External Minimization and Mapping Workflow

| Function | Inferred responsibility | Evidence | Confidence |
| --- | --- | --- | --- |
| `FUN_0040c130` | Complete or refresh output after external minimization/mapping finishes. | Sets busy flag, updates output view via `FUN_0043983d`, changes status text, calls cleanup/status helpers. | Medium |
| `FUN_00414ab1` | Write external-tool input files (`minin.dat`/`checkin.dat`) before running Espresso/MISII. | Called immediately before setting command lines and launching `FUN_0040c196`; surrounding object path fields include `minin.dat`, `checkin.dat`. | Medium |
| `FUN_0041e0c1` | Generate equation/PLA data from a function model for minimization or testing. | Called in random truth-table/minimize flows before external tool launch; returns app error codes. | Medium |
| `FUN_00421aa2` / `FUN_00421c38` / `FUN_00421d2a` | Parse external output back into function/gate data. | Called after reading generated files; nearby code scans text lines until `.e`, typical PLA terminator. | Medium |
| Main command `0x8008` in `FUN_004016d2` | Start minimization. | Builds Espresso command variants such as `espresso\\espresso -Dso %s\\minin.dat`, starts worker thread `FUN_0040c196`, sets “working” state. | High |
| Main command `0xa5` in `FUN_004016d2` | Start gate mapping. | Builds MISII command with `read_library`, `print_gate`, `print_level`, launches `FUN_0040c196`. | High |
| Main command paths around `0x8021` | Complete MISII checkout/output parsing. | `FUN_0040c7c0` posts `0x8021`; dispatcher then parses/imports generated output and refreshes function view. | Medium |

## Help and About

| Function | Inferred responsibility | Evidence | Confidence |
| --- | --- | --- | --- |
| `FUN_00401000` | Test whether a dialog control ID is one of the known hyperlink controls. | Scans a two-entry table at `DAT_00451060`. | High |
| `FUN_00401036` | Return visited/opened state for a known hyperlink control. | Same table; returns state at `DAT_00451064`. | High |
| `FUN_00401078` | Return original wndproc for a subclassed hyperlink control. | Same table; returns stored previous wndproc from `DAT_00451068`. | High |
| `FUN_004010ba` | Return hyperlink display target or click target. | Returns `logic_friday@sontrak.com`/URL text for display; on click marks visited and returns `mailto:` or web target. | High |
| `FUN_0040bfb7` | About dialog procedure. | Dialog resource `"ABOUTDLG"` uses this proc; handles hyperlink clicks through `FUN_004013e5`. | Medium |
| Help command branches in `FUN_004016d2` | Open specific CHM pages. | Builds paths like `%s\\lf.chm::/features.htm`, `%s\\lf.chm::/Entering_TT.htm`, then calls `FUN_0043e36f`. | High |

## Application-Level Collections and Selection

| Function | Inferred responsibility | Evidence | Confidence |
| --- | --- | --- | --- |
| `FUN_0040f186` | Get current selected function from a list/controller. | Many command handlers call it and branch on zero, one, or multiple selected functions. | High |
| `FUN_0040f234` | Get two selected functions. | Used for two-function operations; returns count and output pointers. | Medium |
| `FUN_0040f300` | Select/focus a function by index in the list/controller. | Called before save/close and after creating/opening functions. | Medium |
| `FUN_0040fa6b` | Remove a function from the list/controller. | Used during close/delete flows before freeing the backing function. | Medium |
| `FUN_0040fbca` | Return function count. | Used after closing functions to decide if any remain. | Medium |
| `FUN_0040eb81` | Add/register a function item in the list/controller. | Called after open/import/create; argument points to per-function slot data. | Medium |
| `FUN_0040b4ed` | Allocate or find a free function slot. | Called before creating/opening new functions; negative return is treated as allocation failure. | Medium |
| `FUN_0040b784` | Free/release a function slot. | Called after open/load failure and close/delete flows. | Medium |
| `FUN_0040b8f0` | Resolve function object pointer to slot/index. | Used before saving, close prompts, and filename lookup in `DAT_004528a4`. | Medium |

## Display, Diagram, and Rich Edit Wrappers

| Function | Inferred responsibility | Evidence | Confidence |
| --- | --- | --- | --- |
| `FUN_00439623` | Create a RichEdit-backed text/output control. | Uses `"RichEdit"` or `"RichEdit20A"`, sets event mask/format/limits with RichEdit messages. | High |
| `FUN_004397b2` | Set RichEdit text and formatting. | Sends selection/text replacement and formatting messages. | Medium |
| `FUN_0043983d` | Replace output text in the RichEdit control. | Used when updating output pane/status after operations; sends RichEdit text messages. | Medium |
| `FUN_00439974` | Set expression/source text in an edit control. | Called when selecting/editing a function expression. | Medium |
| `FUN_00439aab` / `FUN_00439af4` | Clear/reset RichEdit or expression edit state. | Called during new/open/close and command reset paths. | Medium |
| `FUN_00439c3b` / `FUN_00439c5c` | Scroll or navigate the RichEdit output. | Send RichEdit messages `0x4dd`/`0x4de`. | Medium |
| `FUN_004373f1` / `FUN_00436925` | Build or prepare an enhanced metafile representation of the gate diagram. | Called before copying/exporting diagrams; outputs `HENHMETAFILE`. | Medium |

## CRT and Compiler Runtime Functions

Most functions from roughly `FUN_0043e470` onward are Visual C++ runtime, exception, heap, file I/O, locale, startup, or low-level OS wrapper code, not Logic Friday business logic. Examples:

| Function | Inferred responsibility | Evidence | Confidence |
| --- | --- | --- | --- |
| `_malloc`, `_free`, `_fread`, `_fwrite`, `_memset`, `_strlen`, `_strcmp`, etc. | Standard CRT library functions recovered by Ghidra. | Ghidra comments identify library matches; wrappers feed project load/save/import/export code. | High |
| `FUN_0044410a` | Low-level CRT write wrapper. | Uses `WriteFile` on OS file handles. | High |
| `FUN_0044448f` | Low-level CRT read wrapper. | Uses `ReadFile` on OS file handles. | High |
| `__sopen` and related functions near `004469xx` | CRT file-open implementation. | Ultimately wraps `CreateFileA` and handle bookkeeping. | High |

## Important Data and File Formats

- `.lfcn` save files use magic/header `"LTK1"` and a fixed 0x2700-byte base state block, followed by dynamically sized truth-table arrays, strings, gate/mapping objects, optional enhanced-metafile bits, and trailing global settings.
- External minimization uses `minin.dat` as input and `minout.dat` as output.
- MISII equation/PLA conversion uses `checkin.dat` and `checkout.dat`.
- `lf.ini` stores window placement, last-used folders, and warning flags.
- The tool bundle is expected beside the application under `espresso\\espresso.exe`, `misii\\misii.exe`, and `misii\\lib\\script.txt`.

## High-Value Next Targets

These functions appear central but need deeper pass-by-pass annotation:

| Function | Why it matters |
| --- | --- |
| `FUN_004016d2` | Central command dispatcher; mapping individual command IDs would explain most UI workflows. |
| `FUN_00414ab1` | Likely serializes function data into Espresso/MISII input formats. |
| `FUN_00421aa2`, `FUN_00421c38`, `FUN_00421d2a` | Likely parse minimizer/mapper output back into model structures. |
| `FUN_0041d944`, `FUN_0041df4d`, `FUN_0041e0c1` | Core function-model mutation and text/equation generation. |
| `FUN_004373f1`, `FUN_00436925`, `FUN_0043a*` drawing helpers | Gate-diagram generation/rendering pipeline. |
