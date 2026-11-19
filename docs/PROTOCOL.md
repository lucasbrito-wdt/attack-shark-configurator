# ATTACK SHARK - Device Protocol and Feature Spec

Reverse-engineered from the vendor Windows configurator (`X3.exe`, x86 PE, DuiLib UI,
sha256 `db7e366f06aa6b06e73cd3a76c7c4d02cf8927e0d9b80d6a838a179d4c52ab41`) plus its bundled
`hiddriver_1.dll`, `hiddriver_2.dll`, `hidapi.dll`, `Hook.dll` and `res/*.xml`.
Purpose: let a Rust developer rebuild the configurator on macOS without the binary.

## 0. Conventions

| Tag | Meaning |
|-----|---------|
| **O** | Observed: read directly from decompiled code, XML or binary strings. |
| **I** | Inferred: deduced from context; verify with a USB/BLE capture before relying on it. |
| `sub_XXXXXX` | Function address inside `X3.exe` that builds or handles the item. |
| `P+0xNNN` | Offset inside the 0x80E0-byte profile struct (section 6.2). |
| `[n]` | Byte index inside a packet, 0-based. All multi-byte packet fields are explicit. |

Static analysis only. Nothing was executed. Pseudo-C is register-level and approximate, so
every byte layout below was cross-checked against stack offsets.

---

## 1. Device identification

### 1.1 IDs

| Link | VID | PID | Opened by | Mode flag | Tag |
|------|-----|-----|-----------|-----------|-----|
| USB cable (wired) | `0x1D57` | `0xFA61` | `hiddriver_2.dll` | `this+0x3DC` | O (IDs), I (wired label) |
| 2.4 GHz dongle | `0x1D57` | `0xFA60` | `hiddriver_1.dll` | `this+0x3D8` | O (IDs), I (dongle label) |
| Bluetooth LE | `0x045E` | `0x0040` | Windows BLE GATT + hidapi probe | `this+0x3E0` | O |

- Constants are set in the constructor `sub_407e60`: `+0x60=0x1D57, +0x64=0xFA61` (passed to
  `hiddriver_2.Set_VIDPID`) and `+0x68=0x1D57, +0x6C=0xFA60` (to `hiddriver_1.Set_VIDPID`). O
- Wired versus dongle labeling (I): the `0x3D8` path arms a 6000 ms battery/heartbeat watchdog
  (timer `0x64`). When it expires, the app shows "Device Sleep!", except when the `0x3DC` link or
  BLE is active. A cable needs no heartbeat, so `0x3DC` = wired and `0x3D8` = dongle.
- BLE uses Microsoft's VID `0x045E` (VID source `02` = USB-IF). The app matches the device
  interface path against the substrings `VID&02045e` and `PID&0040`. O

### 1.2 Interface, usage page, usage

| Item | Value | Tag |
|------|-------|-----|
| API used for config | `HidD_SetFeature` (feature reports) through `hiddriver_x.dll!SetFeature` | O (import present) |
| API used for events | `ReadFile` on the HID handle inside `hiddriver_x.dll` (input reports), forwarded as window messages | O (imports), I (flow) |
| Interface selection | `HidP_GetCaps` + `HidD_GetAttributes` inside the DLL | O (imports) |
| Config interface (SetFeature/GetFeature) | `HIDP_CAPS.UsagePage == 0x000B` and `Usage == 0x0000` | O (hiddriver.dll, Ghidra) |
| Event interface (`Hid_Read`, device -> host events) | `HIDP_CAPS.UsagePage == 0x000A` and `Usage == 0x0000` | O (hiddriver.dll, Ghidra) |
| Device match | VID/PID first, then the HIDP_CAPS usage page/usage above | O (hiddriver.dll, Ghidra) |

macOS/Linux guidance: with `hidapi`, enumerate `0x1D57:0xFA61` and `0x1D57:0xFA60` and select
interfaces with `DeviceInfo::usage_page()/usage()` using the values above. Linux hidraw may
report usage 0/0 for every interface (I): then try each interface of the VID/PID and pin the
one where `send_feature_report` succeeds. Command ID = report ID = `buf[0]`; pass the packet
as-is to `send_feature_report`.

### 1.3 Connection-mode detection (`sub_40e480`, startup)

| Step | Action | Tag |
|------|--------|-----|
| 1 | Load `hiddriver_2.dll`. Call `Set_VIDPID(0x1D57,0xFA61)`, `Open_DevMonitor(hwnd,0x52F,0x530)` and `Open_FeatureDevice()`. Success sets `0x3DC=1` and `0x3E8=0` (awake), hides the sleep overlay and starts UI timer `0x35` (30 ms). | O |
| 2 | Load `hiddriver_1.dll`. Call `Set_VIDPID(0x1D57,0xFA60)`, `Open_DevMonitor(hwnd,0x52D,0x52E)` and `Open_FeatureDevice()`. Success sets `0x3D8=1` and `SetTimer(0x64, 6000 ms)`. | O |
| 3 | Call `hid_init()` and `hid_enumerate(0x045E,0x0040)`, then set `0x3E0` (BLE present). The pseudo-C sets it whenever `hid_init` succeeds, so the null check was probably lost. | O / I |
| 4 | `0x3E4 = 0x3DC OR 0x3D8 OR 0x3E0` ("any device"). | O |
| 5 | When BLE is present, `sub_40e270` → `sub_4060d0` subscribes to notifications (section 4.3). | O |

Multiple PIDs: wired and dongle are opened independently and may both be open at once. The
send path has a fixed priority: **wired > dongle > BLE** (section 2.2). On the dongle, writes
are only sent after the mouse has proven it is alive: `0x3EC=1` is set by the first `0x4010`
status event and cleared when the 6 s watchdog expires. O

### 1.4 Hot-plug (window messages from `hiddriver_x.dll`)

Handlers: `sub_413110` (wired, driver 2) and `sub_412fe0` (dongle, driver 1).

| lParam HIWORD | LOWORD | Meaning | Tag |
|---------------|--------|---------|-----|
| `0x7F00` | `1` | Arrival: `Sleep(1000)`, then `Open_FeatureDevice`, then set the mode flag | O |
| `0x7F00` | `2` | Removal: close the device and clear the mode flag | O |
| `0x1000..0x9000` | value | Device event, forwarded to `sub_4132d0(type=HIWORD, value=LOWORD)` (section 4) | O (range), I (arg order) |

macOS: use IOHIDManager matching callbacks (or polling `hid_enumerate` every ~1 s).

---

## 2. Transport

### 2.1 Packet framing (all commands)

| Byte | Content | Tag |
|------|---------|-----|
| `[0]` | Command ID. On Windows HID, `buf[0]` of `HidD_SetFeature` is the report ID, so **command ID = feature report ID**. | O (value), I (report-ID equivalence) |
| `[1]` | Total packet length in bytes, including `[0]` and the trailing padding | O |
| `[2]` | `0x01` for most commands. Button index for macros (cmd `0x09`). | O |
| `[3..k]` | Payload | O |
| `[k+1]`, `[k+2]` | Checksum, **big-endian** (high byte first), when the command has one | O |
| rest | `0x00` padding up to `[1]` | O |

**Checksum**: `sum = (Σ buf[i] for i in 3..=k) & 0xFFFF`, stored as `buf[k+1] = sum >> 8` and
`buf[k+2] = sum & 0xFF`. It is a plain unsigned 16-bit byte sum. There is no XOR, no seed and
no complement. Commands `0x06` and `0x0C` carry no checksum (they use an inverted-byte check
instead). O

Worked example: cmd `0x05` with factory defaults (section 3.2). Mode 0, deep sleep 10,
speed 6, light colour bytes `00 00 FF`, sleep 1, debounce 4:

```
idx: 0  1  2  3  4  5  6  7  8  9  10 11 12 13 14
     05 0F 01 00 03 A8 00 00 FF 01 04 01 AF 00 00
sum(bytes 3..10) = 00+03+A8+00+00+FF+01+04 = 0x01AF -> [11]=0x01 [12]=0xAF
```

Second check: cmd `0x04` with the defaults in section 3.1 gives `sum(3..49) = 0x0E7D`, so
`[50]=0x0E` and `[51]=0x7D`.

### 2.2 Central send routine `sub_413570(buf, len)`

| Priority | Condition | Behaviour | Tag |
|----------|-----------|-----------|-----|
| 1 | `0x3DC==1` (wired) and `buf[0]!=0x09` | `SetFeature(buf,len)` once | O |
| 1 | `0x3DC==1` and `buf[0]==0x09` (macro) | Split into 3 x 64-byte chunks (section 2.3). After each successful chunk, `Sleep(200)`. If a chunk fails, fall through to priority 2. | O |
| 2 | `0x3D8==1` (dongle) and `0x3EC!=0` (link alive), non-macro | Clear ack flag `0x3F8`. Loop up to **10** times: `SetFeature(buf,len)`, `Sleep(500)`, and stop once `0x3F8` is set by event `0x5010`. | O |
| 2 | Dongle, macro | Clear `0x3F8`. Up to **10** attempts. Each attempt sends 3 chunks with `Sleep(1000)` after each. On chunk index 2, `[1]` is set to `0x0C`. Stop when `0x3F8` is set. | O |
| 3 | `0x3E0!=0` (BLE) and `buf[0]!=0x06` | One GATT reliable write of the **whole** packet to characteristic `0xFEE3` (`sub_4058e0`). Macros are not chunked. Polling-rate packets are **never** sent over BLE. | O |
| - | After any path that returned non-zero | `Sleep(500)` | O |

Debug string `"Wireless Send = %d"` logs each dongle attempt. O

### 2.3 Macro chunking (wired and dongle)

The source is the full 131-byte cmd `0x09` packet. 128 bytes (`src = pkt[3..130]`, which
includes the checksum) are split into 60-byte slices.

| Chunk byte | Value | Tag |
|------------|-------|-----|
| `[0]` | `0x09` | O |
| `[1]` | `0x40` (64). Dongle chunk 2 uses `0x0C`. | O |
| `[2]` | `pkt[2]` (button index) | O |
| `[3]` | chunk index `0,1,2` | O |
| `[4..63]` | `src[60*idx .. 60*idx+59]`, zero-filled past 128 bytes | O |

Chunk 2 therefore carries 8 real bytes: `pkt[123..130]`. Each chunk is written with
`len=0x40`. O

### 2.4 Timing summary

| Item | Value | Tag |
|------|-------|-----|
| Gap after every successful write | 500 ms | O |
| Wired macro inter-chunk | 200 ms | O |
| Dongle retry interval / max | 500 ms / 10 | O |
| Dongle macro inter-chunk / max attempts | 1000 ms / 10 | O |
| Dongle heartbeat watchdog | 6000 ms (timer `0x64`), re-armed by each `0x4010` | O |
| UI debounce before apply (combo boxes) | 200 ms (timer `0x2B`), then message `0x531` | O |
| Arrival settle delay | 1000 ms | O |

### 2.5 Bluetooth LE (GATT)

| Item | Value | Tag |
|------|-------|-----|
| Device lookup | `SetupDiGetClassDevs(GUID @0x41B6A0)`, path containing `VID&02045e` and `PID&0040` | O |
| Write characteristic | 16-bit UUID **`0xFEE3`**, property `IsWritable` | O |
| Write mode | `BluetoothGATTBeginReliableWrite` → `SetCharacteristicValue{DataSize=len, Data=packet}` → `EndReliableWrite` | O |
| Notify characteristic | 16-bit UUID **`0xFEE4`**, property `IsNotifiable`. The CCCD is enabled via `SetDescriptorValue`. | O |
| Primary service UUID | **Unknown**: the code walks every service and picks characteristics by short UUID | - |
| Notification callback | `sub_414e10` → `sub_4132d0` | O |

macOS (I): CoreBluetooth can reach vendor services of an already-connected BLE HID mouse.
Use `retrieveConnectedPeripherals(withServices:)` with the HID service `0x1812`, then
`discoverServices(nil)` and look for `FEE3`/`FEE4`. Use `.withResponse` writes (the closest
match to reliable write). Keep each packet at or under the negotiated MTU. Cmd `0x09` is 131
bytes, so it needs MTU ≥ 134 or long-write support.

---

## 3. Commands (host → device)

Every write is built from the in-memory profile and then dispatched by `sub_413ca0` (apply).
The apply step computes dirty flags against a snapshot (`sub_413920`) and sends **only the
changed groups**, in this order:

| Order | Condition | Packet | Builder |
|-------|-----------|--------|---------|
| 1 | Full resync (`this+0x50CDC==1`) | `0x0C` | inline in `sub_413ca0` |
| 2 | Sensor/DPI group dirty (flag `+0xF1`) | `0x04` | `sub_413fe0` |
| 3 | Light/power/debounce group dirty (`+0xFB`) | `0x05` | inline in `sub_413ca0` |
| 4 | Polling dirty (`+0xF6`) | `0x06` | inline in `sub_413ca0` |
| 5 | Any button dirty (`+0x100`) | `0x08` | `sub_4145f0` |
| 6 | For each button i (1..18) with action `0x11` and macro-dirty `+0x109+i` | `0x09` | `sub_414340` |

Apply is skipped while the device sleeps (`0x3E8==1`). A full resync marks every group dirty
and happens on startup load, profile switch, profile reset and profile import. O

### 3.1 `0x04` Sensor + DPI (len 56 = `0x38`), `sub_413fe0`

| Byte | Field | Source | Range / encoding | Tag |
|------|-------|--------|------------------|-----|
| 0 | `0x04` | const | | O |
| 1 | `0x38` | const | | O |
| 2 | `0x01` | const | | O |
| 3 | Lift-off distance | `P+0x8B4` | `0`=1 mm, `1`=2 mm | O |
| 4 | Ripple control | `P+0x940` | `0`=Off, `1`=On | O |
| 5 | DPI stage enable mask | `P+0x904..0x920` | bit i = stage i enabled | O |
| 6 | Angle snap | `P+0x944` | `0`=Off, `1`=On | O |
| 7 | Motion sync | `P+0x948` | `0`=Off, `1`=On | O |
| 8..15 | DPI stage 1..8, low byte | `P+0x8C4+4i` | `enc = units-1`. DPI = `units*50`, units 1..520 (50..26000 DPI). If `units-1 < 0`, both bytes stay `0`. | O |
| 16..23 | DPI stage 1..8, high byte | same | `enc >> 8` | O |
| 24 | Active stage | `P+0x934` | stored 0..7, sent **+1** (1..8) | O |
| 25..48 | Stage colours 1..8 | `P+0x960+4i` bytes 0,1,2 | 8 x (R,G,B) | O |
| 49 | `0x01` | const | | O |
| 50,51 | Checksum hi, lo | Σ `[3..49]` | | O |
| 52..55 | `0x00` | padding | | O |

Default packet (bytes 3..49):
`00 00 3F 00 00 | 0F 1F 2F 3F 63 07 00 00 | 00 00 00 00 00 02 00 00 | 02 | FF 00 00 00 FF 00 00 00 FF FF FF 00 00 FF FF FF 00 FF FF 40 00 FF FF FF | 01`.

### 3.2 `0x05` Lighting + power + debounce (len 15 = `0x0F`), inline in `sub_413ca0`

| Byte | Field | Source | Encoding | Tag |
|------|-------|--------|----------|-----|
| 0..2 | `05 0F 01` | const | | O |
| 3 | Light mode | `P+0x94C` | `mode << 4` (low nibble 0) | O |
| 4 | Deep-sleep hi nibble + speed | `P+0x92C`, `P+0x928` | `(deepSleep & 0xF0) | (9 - speed)` | O (formula), I (meaning) |
| 5 | Deep-sleep lo nibble + brightness | `P+0x92C`, `P+0x924` | `((deepSleep << 4) & 0xFF) | b`, where `b = brightness` if mode is 1 or 9, else `8` | O (formula), I (meaning) |
| 6,7,8 | Light colour R,G,B | `P+0x95C` bytes 0,1,2 | | O |
| 9 | Sleep time (minutes) | `P+0x930` | 1..60 | O |
| 10 | Key response time (debounce) | `P+0x958` | slider value 2..25 | O |
| 11,12 | Checksum hi, lo | Σ `[3..10]` | | O |
| 13,14 | `00 00` | padding | | O |

Note: deep-sleep minutes (1..60) are split across two nibbles: the high nibble goes to `[4]`
bits 7..4 and the low nibble to `[5]` bits 7..4. This is what the code does. The device-side
meaning is I.

### 3.3 `0x06` Polling rate (len 9), inline in `sub_413ca0`

| Byte | Value | Tag |
|------|-------|-----|
| 0..2 | `06 09 01` | O |
| 3 | `v` = report interval in ms: idx `0`→`8` (125 Hz), `1`→`4` (250 Hz), `2`→`2` (500 Hz), `3`→`1` (1000 Hz) | O |
| 4 | `~v & 0xFF` (check byte) | O |
| 5..8 | `00` | O |

Source `P+0x938` (0..3, default 3). Never sent over BLE. O

### 3.4 `0x08` Button map (len 59 = `0x3B`), `sub_4145f0`

| Byte | Content | Tag |
|------|---------|-----|
| 0..2 | `08 3B 01` | O |
| 3..56 | 18 buttons x 3 bytes `(t, a, b)`; button i at `[3+3i]` | O |
| 57,58 | Checksum hi, lo = Σ `[3..56]` | O |

Per-button encoding, where `id = P+0x9A1+0x4F8*i` (internal action id, section 3.7):

| Action id | `t` | `a` | `b` | Tag |
|-----------|-----|-----|-----|-----|
| `0x10` Shortcut | table `b0` | modifier mask (`button+7`) | HID key usage (`button+3`) | O (layout), I (usage table) |
| `0x11` Macro | table `b0` | `0x00` | button index + 1 (1..18) | O |
| any other | table `b0` | table `b1` | table `b2` | O |
| not in table | `00` | `00` | `00` | O |

Lookup table `@0x41C4A8`: up to 63 entries of `{u32 id, u32 b0, u32 b1, u32 b2, u32 0}`
(stride 0x14, ends at 0x41C998). Only partially decoded (section 7). Known `b2` values match
HID keyboard usages, so for keyboard shortcuts `(b0, b1, b2) = (type, modifier, key)`:

| id | Action | b1 (mod) | b2 (key) | Tag |
|----|--------|----------|----------|-----|
| `0x25` | Find | Ctrl (I) | `0x09` (F) | O (b2) |
| `0x2A` | Close Window | Alt (I) | `0x3D` (F4) | O (b2) |
| `0x2B` | Swap Windows | Alt (I) | `0x2B` (Tab) | O (b2) |
| `0x30` | (unlabelled) | ? | `0x2E` (=) | O (b2) |
| `0x31` | (unlabelled) | ? | `0x2D` (-) | O (b2) |
| `0x32` | Screen Capture | `0x00` | ? (PrintScreen `0x46`, I) | O (b1) |

Modifier mask (shortcut dialog `shortcut.xml`): bit0 Ctrl, bit1 Shift, bit2 Alt, bit3 Win (I:
standard HID modifier order). macOS should map Cmd to Win. O (bits as stored), I (wire meaning)

### 3.5 `0x09` Macro (len 131 = `0x83`), `sub_414340(buttonIndex, button)`

| Byte | Content | Tag |
|------|---------|-----|
| 0,1 | `09 83` | O |
| 2 | Button index (1..18) | O |
| 3 | Play mode, from `button+0x0C` (I: 1=loop N times, 2=until any key, 3=while held; see `select_macro_4..6_text`) | O (source), I (values) |
| 4,5,6 | Loop count from `button+0x43C`, written 3 times | O |
| 7 | `0x01` | O |
| 8..27 | `0x00` | O |
| 28 | Derived count/terminator byte (decompiler unclear) | I |
| 29.. | Event pairs `(timing|flags, key)`, 2 bytes each, up to `[128]` (≤ 50 pairs) | O (position), I (encoding) |
| 129,130 | Checksum hi, lo = Σ `[3..128]` | O |

Event source: `u16 count @button+0x44`, then `count` records of 4 bytes at `button+0x440`
(`u8 key`, `u8 ?`, `u16 delay_ms`). Encoding:

- A delay ≤ 1270 ms (`0x4F6`) becomes `byte = trunc(max(delay,1) / F1 + D2) | flag`. `F1`
  (float @`0x4207B8`) and `D2` (double @`0x4207B0`) were not decoded. `flag` is `0x81` or
  `0x01`, depending on the event's up/down state. O (structure), unknown (constants)
- A delay > 1270 ms is split: `delay / 200` filler events, at most 10, each with key byte `0x03`,
  followed by the remainder. O
- Recommendation: capture one macro with a known 1-key, 50 ms delay on USB before
  implementing.

Macros are always delivered via the chunking in section 2.3 (wired/dongle).

### 3.6 `0x0C` Full-resync / restore (len 10), inline in `sub_413ca0`

`0C 0A 01 FE 01 FE 00 00 00 00`. O (bytes)

Sent before a full rewrite of all groups: at startup load (`sub_40f0c0`), profile switch
(`sub_411300`), "Reset Profile" (`sub_410070`, after reloading defaults) and profile import
(`sub_414820`). I: it puts the device into its default state or begins a bulk update. It is
not a stand-alone "factory reset" button.

### 3.7 Internal action ids (`sub_412540`, menu `ms_1_menu.xml`) - stored at `button+1`

| id | Action (UI string) | id | Action | id | Action |
|----|--------------------|----|--------|----|--------|
| `0x00` | Button Off | `0x0D` | Scroll to Left | `0x47` | Mute |
| `0x01` | Left Click (also the default for no match) | `0x0E` | Scroll to Right | `0x50` | Browser Home |
| `0x02` | Right Click | `0x10` | Assign A Shortcut | `0x51` | Browser Favorites |
| `0x03` | Middle Button | `0x11` | Select A Macro | `0x52` | Browser Forward |
| `0x04` | Forward | `0x16` | Easy Aim | `0x53` | Browser Backward |
| `0x05` | Backward | `0x17` | LED Loop | `0x54` | Browser Stop |
| `0x06` | DPI Cycle | `0x40` | Media Player | `0x55` | Browser Refresh |
| `0x07` | DPI + | `0x41` | Play / Pause | `0x56` | Browser Search |
| `0x08` | DPI - | `0x42` | Stop | `0x57` | Email |
| `0x09` | Double Click | `0x43` | Previous Track | `0x58` | Calculator |
| `0x0A` | Fire Button | `0x44` | Next Track | `0x59` | My Computer |
| `0x0B` | Scroll Up | `0x45` | Volume + | | |
| `0x0C` | Scroll Down | `0x46` | Volume - | | |

Shortcut presets: `0x20` Cut, `0x21` Copy, `0x22` Paste, `0x23` Open, `0x24` Save, `0x25` Find,
`0x26` Redo, `0x28` Select All, `0x29` Print, `0x2A` Close Window, `0x2B` Swap Windows,
`0x2C` Show Desktop, `0x2D` Run Command, `0x2E` Lock PC, `0x32` Screen Capture. The table also
holds `0x27`, `0x2F`, `0x30` and `0x31` (I: Undo and others). O

The strings Profile Cycle/+/-, Power, Sleep and Wakeup exist in `lan.xml`, but the X3 menu
handler assigns them no id. O (absence in handler)

Rule enforced by the UI: at least one button must stay "Left Click" (`one_click_text`). If a
change would violate it, the UI shows a warning and does not assign the action. O (string), I (exact check)

### 3.8 Feature → command cross-reference

| Feature | Command / field | Tag |
|---------|-----------------|-----|
| DPI (8 stages, enable, colour, active stage) | `0x04` | O |
| Polling rate | `0x06` | O |
| Lift-off distance | `0x04[3]` | O |
| Debounce (Key Response Time) | `0x05[10]` | O |
| Angle snap | `0x04[6]` | O |
| Ripple control | `0x04[4]` | O |
| Motion sync | `0x04[7]` | O |
| Sleep / deep sleep | `0x05[9]`, `0x05[4..5]` nibbles | O / I |
| Button remap | `0x08` | O |
| Macros | `0x09` (+ chunking) | O |
| Lighting (UI section hidden on X3) | `0x05[3..8]` | O |
| Profiles | Host-side only: no on-device profile select command exists. Switching resends everything. | O |
| Battery | Event `0x4010` / `0x6000` (read path) | O |
| Firmware version | **No command found.** The "Version" string is the app version (`V1.0.0.0`). | O (absence) |
| Factory reset | Closest is `0x0C` (I) | I |
| Save to flash | **No explicit command.** The device is assumed to persist each write. | I |

---

## 4. Read path (device → host)

### 4.1 No polling reads

`GetFeature` is resolved from both DLLs (`+0x44`, `+0x24`), but **no call site** exists in the
decompiled `X3.exe`. The app never reads configuration back. Its source of truth is the local
`pro.data` (section 6). A rewrite should do the same, or treat the device as write-only. O

### 4.2 Event format

Events arrive as `(type: u16, value: u16)`. They come through window messages from the DLLs
(USB input reports) or through BLE notifications. Both feed `sub_4132d0`.

| Type | Value | Effect in app | Tag |
|------|-------|---------------|-----|
| `0x1000` | 1..8 | Device changed DPI stage: `P+0x934 = v-1`. Shows the on-screen DPI popup (msg `0x469`, value = units*50) and saves. | O |
| `0x2000` | 1..8 | `P+0x938 = v` (polling). Conflicts with the 0..3 index used by cmd `0x06` (section 7). | O |
| `0x4010` | see 4.4 | Heartbeat + battery + awake (section 4.4) | O |
| `0x5010` | status | Write ack: `0x3F8 = (value == expected)`. The exact expected value was lost in the decompile. | O / I |
| `0x6000` | 1..10 | Stored at `this+0x174` and nothing else. `this+0x174` is **never read**, so this event is never displayed. | O |
| `0x7000` | 0..7 | Device changed light mode: `P+0x94C = v`, then save | O |
| `0x8000` | 1..5 | Save only (I: onboard key event, e.g. LED loop) | O / I |
| `0x1010` | 1, 2 | Not in the spec. Observed on hardware over BLE (FEE4 notifications `10 10 02 00` and `10 10 01 00`). Meaning unknown, probably the DPI stage. Parsed as `Event::Unknown { kind, value }`, logged under `SHARK_LOG_EVENTS=1` and never acted on. | O (bytes), unknown (meaning) |

Note: `0x1010` falls inside the accept filter below but has no handler in `sub_4132d0`, so the original app would also ignore it.

Accept filter (`sub_412fe0` / `sub_413110`): an event is forwarded only if
`(type - 0x1000) as u32 <= 0x8000`, i.e. type `0x1000..=0x9000`. Any other type is dropped
silently. Inside `sub_4132d0`, unknown types in that range are no-ops. O

### 4.3 Raw framing

#### 4.3.1 USB (wired `FA61` and dongle `FA60`): X3.exe side, fully resolved

`Open_DevMonitor(hwnd, msgDevice, msgData)` registers two window messages per DLL
(`sub_40e480`). The main window procedure `sub_4086c0` decodes them:

| Msg | DLL / link | Decoding in `sub_4086c0` | Forwarded to | Tag |
|-----|-----------|--------------------------|--------------|-----|
| `0x52F` | driver_2 / wired | `arg = 0x7F000000 + (wParam != 0 ? 1 : 2)` (1 = arrival, 2 = removal) | `sub_413110(arg)` | O |
| `0x530` | driver_2 / wired | `arg = (wParam << 16) ^ lParam` | `sub_413110(arg)` | O |
| `0x52D` | driver_1 / dongle | same as `0x52F` | `sub_412fe0(arg)` | O |
| `0x52E` | driver_1 / dongle | same as `0x530` | `sub_412fe0(arg)` | O |

Both handlers then split `arg` as `type = arg >> 16` and `value = arg & 0xFFFF`. If
`type == 0x7F00`, it is a hot-plug notification. Otherwise the 4.2 filter applies and they
call `sub_4132d0(type, value)`. So for data messages, **type = LOWORD(wParam)** (XOR with
HIWORD(lParam), which is 0 for 16-bit values) and **value = LOWORD(lParam)**. O

#### 4.3.2 USB: inside `hiddriver_x.dll` (resolved, Ghidra via Craft Anything)

`Open_DevMonitor` -> `FUN_10001bc0(0)` opens the UsagePage `0x000A` / Usage `0` interface,
`FUN_100019e0` allocates the input buffer of `HIDP_CAPS.InputReportByteLength`, and
`FUN_10001340` starts reader thread `FUN_10001000` (loop: `Sleep(10)`, then `FUN_10001a70`).

`FUN_10001a70` (one overlapped `ReadFile` of `InputReportByteLength` bytes):

| Step | Code | Tag |
|------|------|-----|
| Filter | Only reports with `b[0] == 0x03` (input report ID 3) are used; anything else is dropped silently | O |
| wParam | `*(u16*)(b+1)` = `b[1] | b[2] << 8` (u16 LE) = **type** | O |
| lParam | `*(u16*)(b+3)` = `b[3] | b[4] << 8` (u16 LE) = **value** | O |
| Post | `PostMessageA(hwnd, msgData, wParam, lParam)`; debug string logs `3, b1, b2, b3, b4` | O |
| Bytes `b[5..]` | ignored | O |

So a raw USB input report is `03 TT TT VV VV ...` with type and value little-endian.
hidapi on macOS/Linux returns the report ID as byte 0 for numbered reports, so the same
offsets apply to `hid_read` output.

#### 4.3.3 BLE notification `0xFEE4`: `sub_414e10` (registered in `sub_4060d0`)

The callback receives `BLUETOOTH_GATT_VALUE_CHANGED_EVENT*` (x86: `+8` =
`PBTH_LE_GATT_CHARACTERISTIC_VALUE`). If `DataSize == 0`, it logs and returns. Otherwise it reads
`Data[0..3]` (structure offsets `+4..+7`) as two little-endian u16 and calls `sub_4132d0`. O

| Data bytes | Meaning | Tag |
|------------|---------|-----|
| `Data[0]` lo, `Data[1]` hi | type (u16 LE) | O (reads), I (arg order: MSVC push order; the decompiler omits the pushes) |
| `Data[2]` lo, `Data[3]` hi | value (u16 LE) | same |
| `Data[4..]` | ignored | O |

The order risk is harmless if the parser self-checks. In the swapped order, a heartbeat's
"type" would be e.g. `0x0801`, which fails the `0x1000..=0x9000` filter. Rule: try
`(type=Data[0..1], value=Data[2..3])`, and if the type fails the filter, try the swap. Log it.

### 4.4 Battery: exact semantics

`0x4010` handler (`sub_4132d0`, `value` = u16 as in 4.3):

| Step | Code behaviour | Tag |
|------|----------------|-----|
| 1 | `KillTimer`/`SetTimer(0x64, 6000 ms)`: re-arm the heartbeat watchdog | O |
| 2 | If link was down (`0x3EC==0`): set `0x3EC=1`, show the `ms_1_wireless` icon, refresh | O |
| 3 | `awake = (value & 0xFF) != 0`. Awake sets `0x3E8=0`, else `0x3E8=1` (the "Device Sleep!" overlay `ms_1_sleep_label` follows it, and apply is blocked while asleep) | O |
| 4 | `level = (value >> 8) & 0xFF`. Only `1 <= level <= 10` updates the UI. Other values (0, >10) keep the previous display. | O |
| 5 | `ms_1_progress_power.SetValue(level * 10)` (range 0..100) | O |
| 6 | `ms_1_label_power.SetResourceText(false)`, then `SetText(format("%d%%", level*10))` (format string at `0x420238`) | O |

| Question | Answer | Tag |
|----------|--------|-----|
| Scale | 10 steps: level 1..10 maps to 10 %..100 %. No finer resolution exists. | O |
| `0x6000` | Value 1..10 is stored, never shown (4.2) | O |
| Dongle vs BLE | Same handler, same scale. BLE events also go through `sub_4132d0`. | O |
| Wired (`FA61`) arrival (`sub_413110`) | Shows the `ms_1_wired` icon and sets `0x3E8=0`. `ms_1_label_power` switches to resource text `charging_text` = **"Charging..."**. Timer `0x35` (30 ms, `sub_411a10`) animates `ms_1_progress_power` 0→100 in steps of 2, looping. | O |
| Wired removal | Kills timer `0x35` and sets the label text to `""` | O |
| Charging flag in protocol | **None.** "Charging" is purely "the USB cable PID is present". No event carries a charge state. | O |
| Battery while wired | Not handled specially. If a wired device sent `0x4010`, the label would switch to `%` while the animation keeps running. Whether wired sends `0x4010` is unknown. | O (code), unknown (device) |
| Low-battery warning | **Dead code.** Timers `0x32`/`0x33` (`sub_411a10`) slide control `ms_1_low_power_label` (`low_power_text` = "It needs to be charged"), but nothing calls `SetTimer` with `0x32`/`0x33`, and that control does not exist in any XML. No threshold exists. | O |
| Before the first heartbeat | XML defaults: progress `value="50"`, label `"50%"`, so the app shows a fake 50 %. A rewrite should show "unknown" instead. | O |
| No heartbeat for 6 s (dongle) | Timer `0x64` fires: `0x3EC=0`, hide the wireless icon, show "Device Sleep!" (unless wired or BLE is up). The battery display keeps its last value. | O |
| Skin images | `power_1.png` (frame) and `power_2.png` (fill, clipped by progress). There are no per-level images. | O |

**Diagnostic only (not in the original app).** Over BLE the X3 also exposes the standard Battery Service
`0x180F` / Battery Level `0x2A19`. Its value is unreliable on this device (always 100 %, observed), so it is
only logged (`bas_battery=<n>` under `SHARK_LOG_EVENTS=1`) and never used for the display. The `0x4010` path above
is the only source.

### 4.5 Event test vectors

`(type, value)` → effect: these are observed and safe for unit tests.

| type | value | Expected result |
|------|-------|-----------------|
| `0x4010` | `0x0801` | awake, battery 80 % |
| `0x4010` | `0x0A01` | awake, battery 100 % |
| `0x4010` | `0x0100` | asleep, battery 10 % |
| `0x4010` | `0x0001` | awake, battery unchanged (level 0 ignored) |
| `0x4010` | `0x0B01` | awake, battery unchanged (level 11 ignored) |
| `0x1000` | `0x0003` | DPI stage index 2 active |
| `0x6000` | `0x0005` | stored only, no display change |
| `0x0801` | any | dropped (fails `0x1000..=0x9000` filter) |

Raw bytes (BLE `Data[0..3]`, order per 4.3.3; the swap fallback makes either order pass):

| Bytes | Event |
|-------|-------|
| `10 40 01 08` | `0x4010`, awake, 80 % |
| `10 40 00 01` | `0x4010`, asleep, 10 % |
| `00 10 03 00` | `0x1000`, stage 3 |

USB raw bytes (O, section 4.3.2):

| Bytes | Event |
|-------|-------|
| `03 10 40 01 08` | `0x4010`, awake, 80 % |
| `03 10 40 00 0A` | `0x4010`, asleep, 100 % |
| `03 00 10 03 00` | `0x1000`, stage 3 |
| `02 10 40 01 08` | dropped (report ID is not 3) |

---

## 5. UI inventory (`res/MS/MS_1/ms_1_page.xml`, `lan.xml`)

### 5.1 Main page controls

| Control (name) | English label | Range / options | Profile field | Command | Tag |
|----------------|---------------|-----------------|---------------|---------|-----|
| `ms_1_btn_key_1..18` | Key rows (1,2,3,7,8 visible; others hidden) | menu `ms_1_menu.xml` | `P+0x9A0+0x4F8*i` | `0x08` / `0x09` | O |
| DPI vertical sliders x8 | "DPI Settings" | 1..520, shown as x50 (stages 7-8 hidden) | `P+0x8C4+4i` | `0x04` | O |
| DPI edit boxes x8 | | 50..26000 | same | `0x04` | O |
| Stage radio group | (active stage) | 1..8 | `P+0x934` | `0x04[24]` | O |
| Stage colour buttons (`DpiColorDialog.xml`) | | RGB | `P+0x960+4i` | `0x04[25..48]` | O |
| Stage enable check | | on/off | `P+0x904+4i` | `0x04[5]` | O |
| `ms_1_combo_light` (hidden) | "Light Settings" / "Mode:" | 12 modes (8-12 hidden) | `P+0x94C` | `0x05[3]` | O |
| `ms_1_combo_light_ble` (hidden) | "Mode:" | 6 modes | `P+0x950` | none found | O |
| `ms_1_slider_brightness` | "Brightness:" | 1..8 | `P+0x924` | `0x05[5]` | O |
| `ms_1_slider_breathspeed` | "Speed:" | 4..8 | `P+0x928` | `0x05[4]` (`9-v`) | O |
| `ms_1_btn_light_color` | "Color:" | RGB | `P+0x95C` | `0x05[6..8]` | O |
| `ms_1_btn_prate_1..4` | "Polling Rate Settings": Power Saving / Office / Gaming / E-sports | 125/250/500/1000 Hz | `P+0x938` = 0..3 | `0x06` | O |
| `ms_1_btn_mouse_attribute` | "Mouse Attribute" | opens Windows mouse control panel | `P+0x898..0x8A4` (I) | none | I |
| `ms_1_slider_light_off` + label | "Power Management" / "Sleep Time: N Min" | 1..60 | `P+0x930` | `0x05[9]` | O (field), I (label) |
| `ms_1_slider_sleep_timer` + label | "Deep Sleep Time: N Min" | 1..60 | `P+0x92C` | `0x05[4..5]` | O (field), I (label) |
| `ms_1_check_movemode` (hidden) | "Move Wake" | on/off | unknown | none found | O |
| `ms_1_option_lift_of_distance_1/2` | "Lift Of Distance": 1 MM / 2 MM | 0/1 | `P+0x8B4` | `0x04[3]` | O |
| `ms_1_slider_key_debounce` + label | "Key Response Time" | 2..25. Label shows `2*v` (I: ms) | `P+0x958` | `0x05[10]` | O |
| `ms_1_option_ripple_control_1/2` | "Ripple Control": Off / On | 0/1 | `P+0x940` | `0x04[4]` | O |
| `ms_1_option_angle_snap_1/2` | "Angle Snap": Off / On | 0/1 | `P+0x944` | `0x04[6]` | O |
| `ms_1_option_motion_sync_1/2` | "Motion Sync": Off / On | 0/1 | `P+0x948` | `0x04[7]` | O |
| Battery progress + label | `"%d%%"` | 10..100 | event `0x4010` | read | O |
| Sleep overlay | "Device Sleep!" | | `this+0x3E8` | read | O |
| Apply button | "Apply" | | | runs apply (section 3) | O |

Light modes (`ms_1_lightmode1..12`): LED Off, Static, Breathing, Neon, Color Breathing,
Color Static, Mixed Breathing, Rainbow Wave, Lightning, Static Mixed Color, Marquee,
Marquee 2. The value sent is the 0-based index. O

### 5.2 Profile list and macro manager

| Control | Label | Effect | Tag |
|---------|-------|--------|-----|
| Profile list | "Profile" | Select: set `this+0x3FC` = index, full resync (`0x0C` + all) | O |
| Add / Delete / Rename | "Add Profile", "delete Profile", "Rename Profile" | Host-side only. "Keep at least one configuration file!" | O |
| Import / Export | "Import Profile" / "Export Profile" | `.pro` file = raw 0x80E0 profile struct, filter `"Profile Flie" *.pro;*.PRO` | O (import), I (export symmetric) |
| Reset | "Reset Profile" | Confirm "Are you sure you want to restore...". Reloads defaults (`sub_4155e0`), full resync. | O |
| Macro manager (`ms_1_macropage.xml`) | "Macro", Recording/Stop, Insert delay, Move Up/Down, Delete, Clear All, Import/Export | Edits the macro library (`macro.data`). Recording uses global hooks from `Hook.dll`. | O (UI), I (hook use) |
| Select macro (`selectmacro.xml`) | "Select A Macro", "Loop Times:", play-mode radios | Copies the macro into the button struct, action `0x11` | O |
| Shortcut dialog (`shortcut.xml`) | "Assign A Shortcut" | Ctrl/Shift/Alt/Win + key → action `0x10` | O |

---

## 6. Local persistence

### 6.1 Files

| File | Content | Writer / reader | Tag |
|------|---------|-----------------|-----|
| `<base>\pro.data` (strings `\pro.data`, `ms_1\pro.data`) | `0x28464`-byte global block (`this+0x3FC`; first u32 = selected profile index) followed by N x `0x80E0` profile structs | `sub_4158e0` / `sub_415b50` | O (layout), I (path) |
| `<base>\macro.data` (`\macro.data`, `ms_1\macro.data`) | Macro library, `0xF0`-byte entries: `wchar name[24]`, u32 @0x30, u32 @0x34, 46 x 4-byte events @0x38 | `sub_4160d0` / `sub_416320` | O (entry), I (header) |
| `*.pro` | One raw profile struct (0x80E0 bytes) | `sub_414820` | O |

`<base>` is built by `sub_4015c0()` plus wide string `@0x41D39C`, and the directory is created
if missing. Nearby strings `configuration` and `ms_1` suggest `...\configuration\ms_1\` (I).
For the rewrite, use `~/Library/Application Support/<app>/`. Byte-compatibility with
`pro.data` is optional.

### 6.2 Profile struct (0x80E0 bytes, little-endian u32 unless noted)

| Offset | Field | Default (`sub_4155e0`) | Tag |
|--------|-------|------------------------|-----|
| `0x000` | `wchar name[100]` | | O |
| `0x0C8` | `wchar[500]` (unknown) | "" | O |
| `0x4B0` | `wchar[500]` (unknown) | "" | O |
| `0x898` | u32 (I: Windows double-click time) | 420 | O / I |
| `0x89C` | u32 (I: wheel lines) | 3 | O / I |
| `0x8A0` | u8 (I: enhance precision) | 0 | O / I |
| `0x8A4` | u32 (I: pointer speed) | 6 | O / I |
| `0x8B0` | u32 unknown (cmd-4 dirty set, not sent) | 0 | O |
| `0x8B4` | LOD 0/1 | 0 | O |
| `0x8B8` | u32 unknown (cmd-4 dirty set, not sent) | 0 | O |
| `0x8C4` | DPI units[8] | 16, 32, 48, 64, 100, 520, 0, 0 (800..26000) | O |
| `0x8E4` | DPI units mirror[8] (I: Y axis or backup, not sent) | same | O |
| `0x904` | stage enabled[8] | 1,1,1,1,1,1,0,0 | O |
| `0x924` | brightness | 8 | O |
| `0x928` | light speed | 6 | O |
| `0x92C` | deep sleep minutes | 10 | O |
| `0x930` | sleep minutes | 1 | O |
| `0x934` | active stage (0-based) | 1 | O |
| `0x938` | polling index 0..3 | 3 (1000 Hz) | O |
| `0x940` | ripple | 0 | O |
| `0x944` | angle snap | 0 | O |
| `0x948` | motion sync | 0 | O |
| `0x94C` | light mode | 0 | O |
| `0x950` | BLE light mode | 3 | O |
| `0x954` | u8 unknown | 1 | O |
| `0x958` | debounce | 4 | O |
| `0x95C` | light colour (bytes R,G,B,0) | `00 00 FF` | O |
| `0x960` | stage colour[8] (bytes R,G,B,0) | red, green, blue, yellow, cyan, magenta, `FF 40 00`, white | O |
| `0x980` | palette[8] for the colour dialog | | O |
| `0x9A0` | button[18], stride `0x4F8` (below) | from tables `@0x41C9E0`/`@0x41C998` | O |

Button struct (relative to `0x9A0 + 0x4F8*i`):

| Off | Field | Tag |
|-----|-------|-----|
| `+0x000` u8 | index i | O |
| `+0x001` u8 | action id (section 3.7) | O |
| `+0x002` u8 | default/aux byte | O |
| `+0x003` u8 | shortcut HID key | O |
| `+0x007` u8 | shortcut modifier mask | O |
| `+0x008` u16 | `0x232` (562) default, unknown | O |
| `+0x00C` u8 | macro play mode | O / I |
| `+0x010` | `wchar macro_name[24]` | O |
| `+0x040` u32 | macro attribute (copied to library entry `+0x30`) | O |
| `+0x044` u16 | macro event count | O |
| `+0x430`, `+0x434` u32 | `-1` defaults, unknown | O |
| `+0x43C` u8 | macro loop count | O / I |
| `+0x440` | events[], 4 bytes each (`u8 key, u8 flag?, u16 delay_ms`) | O / I |

---

## 7. Unknowns / low confidence

| # | Item | Why unknown | Suggested resolution |
|---|------|-------------|----------------------|
| 1 | HID interface number of the config interface | Usage page `0x000B` (config) and `0x000A` (events) are known (section 1.2, O); the interface number is not | Confirm on hardware which interface `send_feature_report` succeeds on |
| 2 | Whether the DLL pads feature reports to the descriptor length, or strips/keeps `[0]` as report ID | DLL internals | USB capture of one `0x06` write (9 bytes, simplest) |
| 3 | ~~USB input-report bytes~~ RESOLVED in 4.3.2: `03 TT TT VV VV`, LE, report ID 3 only | - | - |
| 4 | Order of type/value in the 4-byte BLE notification | Decompiler dropped the argument order | Self-checking swap rule (4.3.3) |
| 5 | BLE service UUID that owns `FEE3`/`FEE4` | Code scans all services | `discoverServices(nil)` |
| 6 | Lookup table `@0x41C4A8` (`b0`,`b1` for most ids, all media/browser codes) | Control-character bytes are not visible in a string scan | Disassemble the table bytes, or USB-capture each action once. Likely: mouse buttons `b0=0x01` with a button bitmask, media `b0=0x03` with a consumer usage (I) |
| 7 | Macro delay encoding constants (`@0x4207B8` float, `@0x4207B0` double), flag `0x81`/`0x01`, byte `[28]` | Float constants not decoded | Capture one known macro |
| 8 | Cmd `0x05` `[4]`/`[5]` nibble semantics (deep-sleep split) | Unusual encoding; device semantics unseen | Capture with deep sleep = 1 and = 60 |
| 9 | `0x0C` semantics (reset versus begin-bulk) | No device feedback in code | Test with an unpaired spare unit first |
| 10 | Event `0x2000` polling value domain (1..8) versus cmd `0x06` index | Code stores the raw value; probably a vendor bug or a different encoding | Log the event after changing polling on the device |
| 11 | `0x5010` ack expected value | `sete` comparand lost | Log the value after a dongle write |
| 12 | Profile fields `0x8B0`, `0x8B8`, `0x954`, `0x950` (BLE mode not sent), `0x8E4` mirror | No UI or packet use found | Ignore for v1 |
| 13 | Default button action table (`@0x41C9E0`) | Binary bytes not decoded | Read from a fresh `pro.data`, or assume L, R, M, Back, Fwd |
| 14 | Firmware version / battery query commands | None exist in this app | Out of scope, or probe GetFeature IDs 1..0x10 carefully |
| 15 | Wired versus dongle PID assignment (`FA61` wired, `FA60` dongle) | Inferred from the watchdog logic | Confirm with `system_profiler SPUSBDataType` |
| 16 | Exact `pro.data` base directory and global-block layout beyond the first u32 | `sub_4015c0`, `sub_415b50` not traced | Not needed for a clean rewrite |
