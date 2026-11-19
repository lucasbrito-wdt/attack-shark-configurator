# Family E (R6, platform "XMG") - Device Protocol Spec

Reverse-engineered from `DriverCore.exe` (.NET Framework 4.0 x86 WinForms/SunnyUI, assembly
DriverCore 2.0.7.7, sha256 `781dd570dc1e614d247445d04f81a1ed3660c8844bbe7dd42bc49ab37fba300b`),
plus `Config/xvi_models.xlsx` and `Config/XMG_R6/Language.ini`. No `hiddriver.dll`: HID access
is HidSharp 2.1 inside the exe. Static analysis only; nothing was executed.

**Obfuscation**: the assembly is obfuscated (renamed types and members, junk branch prologues,
Costura-embedded dependencies). String literals, enum names and the xlsx/ini config are NOT
encrypted. The protocol class is `ALRaXoB4KoEnWqpFoi` (extends `FWProtocolBase`); names below
are descriptive, not the obfuscated ones. IL was read with a custom pseudo-decompiler, so
control flow in large switch bodies was reconstructed from raw IL.

## 0. Conventions

| Tag | Meaning |
|-----|---------|
| **O** | Observed in IL, xlsx or ini. |
| **I** | Inferred from usage; verify with a USB capture. |
| `[n]` | Byte index in the 65-byte buffer. `[0]` is the report ID (always 0). Multi-byte fields say BE/LE. |
| NP | `NewProtocol=1` (set for R6): a profile byte is inserted at `[7]`, value moves to `[8]`, `[4]` grows by 1. |

## 1. Model registry (O)

`xvi_models.xlsx` has exactly one data row.

| Field | Value |
|-------|-------|
| Model / name | R6 |
| VID | `0x373E` |
| PID wired | `0x0021` |
| PID wireless (2.4 GHz dongle) | `0x0022` (golden dongle name "8K") |
| BLVID / BLPID | empty (no Bluetooth, no bootloader IDs) |
| MI | `MI02` (config interface) |
| Polling wired (Hz) | 125;250;500;1000 |
| Polling wireless (Hz) | 125;250;500;1000;2000;4000;8000 |
| UI folder | `XMG_R6` |
| DPI LED colors | ffff0000;ff0000ff;ff00ff00;ffff00ff;ffffff00;ffffffff (ARGB) |
| DefaultSleepTime | 15 min |
| EffectListID | empty |

`XMG_R6/Language.ini [App]`: PairType=0, NewProtocol=1, LightDeviceID=2, DPIMax=42000,
SensorMoveWakeupEnable=0. Mode: PID 0x0021 = wired, 0x0022 = wireless (O for IDs; the
wired/wireless labels come from the per-mode polling lists, O).

## 2. Transport

### 2.1 Interface selection (O)

Enumerate HID devices; keep those whose path contains `MI_02` and
`GetMaxFeatureReportLength() == 65`; then match VID/PID against section 1. On macOS there is no
`MI_` in the path: pick the interface of VID `0x373E` with a 65-byte feature report (I), or
usage page/usage if `hidapi` exposes them (not read from the exe).

### 2.2 Report format (O)

- All commands are **feature reports**, report ID 0, buffer length 65 (report ID + 64 data).
- Sequence: `SetFeature(buf)` -> sleep 30 ms -> `GetFeature(buf)` (reply in a fresh 65-byte
  buffer; `[0]=0`).
- Send-with-ack helpers re-send and re-read up to 10 times (5 in one helper) until
  `reply[1] == 0xA1`. Reply `[5]`/`[6]` echo the request category/command and are compared
  with the request. Macro chunk retries wait 200 ms. Constants: length 65, delays 30/30 ms.
- Input reports (events) are separate, section 5.
- **No checksum or CRC** in any packet builder (O). The whole frame is plain.

### 2.3 Frame

| Byte | Meaning | Tag |
|------|---------|-----|
| `[0]` | Report ID = 0 | O |
| `[1]`,`[2]` | 0 in requests. Reply `[1] == 0xA1` means ack | O (value), I (meaning) |
| `[3]` | Target: 0 dongle, 1 receiver/pair/PID query, 2 mouse (LightDeviceID=2). Default helpers force 2. RF address uses the device id | O usage, I semantics |
| `[4]` | Payload length (bytes from `[5]` on) | O |
| `[5]` | Category: 0 general, 1 sensor/DPI/options, 2 lighting/dongle LED, 3 buttons, 4 macro | O |
| `[6]` | Command. Bit `0x80` = get (read) variant | O |
| `[7..]` | Data. With NP, `[7]` = profile id (1..3) for profile-scoped commands | O |

### 2.4 Worked example (battery read)

Set: `00 00 00 02 02 00 83 00 ...` (65 bytes, zero padded)
-> `[3]=2` mouse, `[4]=2`, `[5]=0`, `[6]=0x83`.
Wait 30 ms, GetFeature. Reply `[7]` = charge status (1 = charging), `[8]` = percent.
Example reply `00 A1 00 02 02 00 83 01 4B ...` = charging at 75 percent (illustrative; layout O).

## 3. Commands

Profiles are 1..3. "get" = same frame with `[6] |= 0x80`. All data below is after `[6]`.
Unless a row says otherwise, `[3]=2`, `[5]` as listed, `[4]` as listed.

### 3.1 General (category 0)

| Command | `[3]` | `[4]` | `[5]` | `[6]` | Data / reply | Tag |
|---------|-------|-------|-------|-------|--------------|-----|
| FW version mouse | 2 | 0x10 | 0 | 0x81 | reply `[7..10]` formatted `{:02}.{:02}.{:02}.{:02}` | O |
| FW version dongle | 0 | 0x10 | 0 | 0x81 | same | O |
| Device PID query | 1 | 6 | 0 | 0x8B | `[7]=2`; reply PID = `([11]<<8)+[12]` | O |
| Battery | 2 | 2 | 0 | 0x83 | reply `[7]` status (1 charging), `[8]` percent | O |
| Get active profile | 2 | 1 | 0 | 0x85 | reply `[7]` | O |
| Set active profile | 2 | 1 | 0 | 5 | `[7]=id` | O |
| Reset (all) | 2 | 2 | 0 | 0 | `[7]=0xC0 [8]=1`, no read; sleep 1000 ms; then `[3]=0 [4]=2 [7]=0xC0` to the dongle | O (bytes), I (effect) |
| Reset profile | 2 | 1 | 0 | 13 | `[7]=profile`, no read | O |
| Pair status set | 1 | 1 | 0 | 12 | `[7]=status` | O |
| Pair status get | 1 | 0 | 0 | 0x8C | reply `[7]` | O |
| RF address set | id | 4 | 0 | 10 | `[7..10]` | O |
| RF address get | id | 4 | 0 | 0x8A | reply `[7..10]` | O |

Pairing: `SetPairStatus(1)`, poll `GetPairStatus` every 500 ms; status 2 or 3 = success, then
`SetPairStatus(0)` (O).

### 3.2 Sensor and options (category 1)

Profile-scoped, NP layout: `[7]=profile`, value at `[8]`, `[4]=2` (3 for sleep). Reply value at `[8]`.

| Command | `[6]` set/get | Value | Tag |
|---------|---------------|-------|-----|
| Polling rate | 0 / 0x80 | code: 8=125, 4=250, 2=500, 1=1000, 32=2000, 64=4000, 128=8000 Hz. Reply 0x10 is treated as 1 | O |
| DPI stages | 1 / 0x81 | see 3.3 | O |
| Active DPI stage | 2 / 0x82 | `[8]=stage` (4 for set `[4]=2`) | O |
| Angle snap | 4 / 0x84 | 0 or 1 | O |
| Sleep | 7 / 0x87 | `[8..9]` seconds BE16, 15..1800; 65535 = disabled; default 15*60 | O |
| LOD | 8 / 0x88 | 1 = 1 mm, 2 = 2 mm, 0x87 also offered when DPIMax >= 30000 (UI label "87") | O, I (0x87 meaning) |
| Debounce (cat 1) | 8 / 0x88 | UI 1..15 ms at `[8]` | O |
| Motion sync | 9 / 0x89 | 0 or 1 | O |
| Ripple | 10 / 0x8A | 0 or 1 | O |
| Sensor wake-up | 0x11 / 0x91 | 0 or 1; hidden for R6 (SensorMoveWakeupEnable=0) | O |

Note: debounce and LOD both appear at `[6]=8` in the decompiled builders; they are separate
getter/setter pairs and may share the command id. Treat debounce as unconfirmed until
captured (I).

### 3.3 DPI (category 1)

| Command | Frame |
|---------|-------|
| Set stages | `[4]=count*4+2 [5]=1 [6]=1 [7]=profile [8]=count [9+4i..]= X BE16, Y BE16` (i = 0..count-1, max 6) |
| Get stages | `[4]=10 [5]=1 [6]=0x81 [7]=profile`; reply `[8]=count`, stages from `[9]` |
| Set active stage | `[4]=2 [5]=1 [6]=2 [7]=profile [8]=stage` |
| Get active stage | `[6]=0x82` |

DPI is in steps of 50 (UI trackbar 1..840 when DPIMax=42000; DPI = value*50) (O). X and Y are
independent BE16 fields (O).

### 3.4 Lighting (category 2)

| Command | Frame | Tag |
|---------|-------|-----|
| DPI stage colors | `[4]=19 [5]=2 [6]=1 (get 0x81) [7]=profile [8..25]` = 6 x RGB | O |
| Light follows mouse | `[5]=2 [6]=3 (get 0x83)` value 0/1 | O |
| Dongle LED | `[3]=0 [4]=2 [5]=2 [6]=4 (get 0x84) [7]=profile [8]=0/1`; wireless mode only | O |
| Lighting effect | `[3]=2 [4]=5+n [5]=2 [6]=0 (get 0x80) [7]=profile [8]=zones [9]=effect [10]=dir [11]=speed [12..]=color` | O layout, unused on R6 |
| Brightness | branches only for PIDs 0x2011/0x2013; sends nothing for R6 | O |

Effects: 0 OFF, 1 Glorious, 2 SeamlessBreathing, 3 BreathingRGB, 4 SingleColor,
5 BreathingSingleColor, 6 Tail, 7 Rave, 8 Wave, 160 AnimationFrame (O). R6 has EffectListID
empty, so only DPI LED colors and follow-mouse are expected to be used (I).

### 3.5 Buttons (category 3)

Set/get: `[4]=5+size [5]=3 [6]=0 (get 0x80) [7]=profile [8]=buttonID [9]=0 [10]=functionID [11]=size [12..]=data`. Reply uses the same offsets (function at `[10]`, data from `[12]`).

ButtonID (O): 0 NA1, 1 Left, 2 Right, 3 Middle, 4 Backward, 5 Forward, 6-8 Button6-8, 9 NA2,
16 ScrollUp, 17 ScrollDown, 18 DPIUp, 19 DPIDown, 20 DPILoopUp, 21 DPILoopDown.

FunctionID and data (O):

| ID | Function | Size | Data |
|----|----------|------|------|
| 0 | OFF | 0 | - |
| 1 | ButtonCode | 1 | `[0]` code: 1 left, 2 right, 3 middle, 4 back, 5 fwd, 16/17 scroll |
| 2 | X_Click (double click) | 5 | `01 01 02 00 64` (I: count/button/interval 0x64) |
| 4 | Keyboard | 2 | `[modifier, keycode]` |
| 5 | Consumer | 2 | usage BE16 `[hi, lo]` |
| 6 | Power | 1 | 129 Power, 130 Sleep, 131 Wake |
| 7 | DPI | 1 or 5 | `[0]`: 1 up, 2 down, 5 lock (then X BE16, Y BE16), 6 cycle up, 7 cycle down |
| 8 | Profile | 1 | 1 up, 2 down, 3 loop up, 4 loop down |
| 12 | BatButton | 1 | 1 |
| 16 | MacroI | 3 | `[id_hi, id_lo, repeat]` |
| 17 | MacroII | 2 | `[id_hi, id_lo]` |
| 18 | MacroIII | 2 | `[id_hi, id_lo]` |

Modifier bitmask: LCtrl 1, LShift 2, LAlt 4, LGui 8, RCtrl 16, RShift 32, RAlt 64, RGui 128.

Consumer usages (hex): Eject B8, Mute E2, VolDown EA, VolUp E9, Prev B6, Stop B7, PlayPause CD,
Next B5, MediaSelect 183, Calc 192, MyComputer 194, MyExplorer 196, WWWHome 223, WWWRefresh 227,
Mail 18A, LightingUp 6F, LightingDown 70, SelectApp 1A3, Copy 21B, Cut 21C, Paste 21D.

### 3.6 Macros (category 4)

IDs 1..30. All BE. Target `[3]=2`.

| Command | Frame |
|---------|-------|
| Exists/size | `[4]=6 [5]=4 [6]=0x81 [7..8]=id`; reply `[9..12]` = size BE32 (nonzero = exists) |
| Delete | `[4]=2 [5]=4 [6]=2 [7..8]=id` |
| Allocate | `[4]=6 [5]=4 [6]=1 [7..8]=id [9..12]=total size` |
| Write chunk | `[4]=7+n [5]=4 [6]=3 [7..8]=id [9..12]=offset [13]=n [14..]=data`; 51 bytes max per chunk, retry 200 ms |
| Read chunk | `[6]=0x83`, same fields; reply data at `[14..]` |

Macro event bytes (O values; press/release direction is I):

| Event | Encoding |
|-------|----------|
| Delay | `0x20+1 byte`, `0x21+2`, `0x22+3`, `0x23+4` (BE value) |
| Key | `0x02 keycode` / `0x03 keycode` (press/release, I) |
| Modifier key | `0x09 / 0x0A + modifier bit` |
| Mouse move | `0x08 X int16 BE, Y int16 BE` |
| Mouse button | `0x01 button (1..5)` |
| Wheel | `0x10 + 0xFF or 0x01` |

## 4. Capabilities (R6)

| Feature | Wired | Wireless |
|---------|-------|----------|
| Polling | 125/250/500/1000 | up to 8000, dongle name "8K" caps; names 1K/4K/8K map to 1000/4000/8000 (O) |
| DPI | 1..6 stages, step 50, max 42000 | same |
| Sleep, debounce, LOD, angle snap, ripple, motion sync | yes | yes |
| Dongle LED | no | yes |
| Sensor wake-up | hidden | hidden |
| Bluetooth | none (no BLPID) | - |
| Lighting effects | none expected | none expected |

## 5. Device -> host events

Read with `HidDeviceInputReceiver` from HID interfaces of the same VID/PID whose path is not
`mi_00` (which interface exactly: I). Report ID `0x04`; byte 0 = report ID, byte 1 = event (O):

| `[1]` | Event | Data |
|-------|-------|------|
| 01 | DPI changed | - (re-read active stage) |
| 06 | Link status | `[2]`: 01 online, 00 offline (UI disabled in wireless) |
| 07 | LOD changed | - |
| 08 | Polling changed | - |

**Battery has no event**: poll `Battery` (home page timer 2000 ms, details page 1000 ms) (O).

## 6. Unknowns and risks

- Which non-MI_00 interface delivers input reports; macOS mapping of MI_02.
- Meaning of reply `[1]=0xA1`, target byte `[3]` semantics, and the NP offset rule: derived
  from call sites, not from a capture.
- Debounce vs LOD sharing `[6]=8` (section 3.2); LOD label "87"; polling reply 0x10 -> 1.
- Macro event press/release opcodes and X_Click data semantics.
- R6 lighting: likely only DPI LED colors; brightness is a no-op.
- Obfuscated assembly: field offsets were cross-read from IL, not tested on hardware.
