# ATTACK SHARK family C: Electron apps (F1, X11 PRO V2, R5 Ultra, X3 PRO, R2, R3)

Tags: **O** = observed in shipped app code (static read, nothing executed); **I** = inferred; **U** = unknown.
Sources: vendor Electron apps, read-only static analysis. No device was touched, no vendor code was run.

Family C is NOT one protocol. Three unrelated wire protocols are shipped by apps that look alike:

| Part | Models | App | Platform | VID | Wire protocol |
|---|---|---|---|---|---|
| C1 | F1, X11 PRO V2 | "BK-MouseHub" Next.js, asar | BK, same as family A (X3, 0x1D57) | 0x1D57 | WebHID feature reports, report ID = command ID, 16-bit BE sum |
| C2 | R5 Ultra | "ATTACK SHARK GAMING", asar | XVI/JXC (same as R6 / M5 Ultra, see family-e.md) | 0x373E | 64-byte feature report 0, `dev/len/cat/cmd` frames |
| C3 | X3 PRO, R2, R3 | "Driver v4" iot_manager, no asar | Rongyuan (RY) QMK-like vendor channel | 0x3151 | 9-byte or 64-byte feature report 0, `255 - sum(b[0..6])` at `[7]` |

C3 is NOT WebHID. The renderer talks gRPC-web to local Rust services (127.0.0.1:3838 `hid.HidService`, :6015 `iot_manager.IotManager`) that
do raw HID. The protocol bytes are still built in the JS bundles, so they are fully documented below. Remote servers (api2.qmk.top:3816,
aidshk.qmk.top, news.rongyuan.tech) only serve bit-image gallery, AI lighting canvas and installer downloads; they carry no device protocol (O).

---------------------------------------------------------------------------------------------------

# C1. BK-MouseHub (F1, X11 PRO V2) - same family as family A

All offsets below are in the buffer passed to `sendFeatureReport(buf[0], buf.slice(1))`, i.e. `buf[0]` is the report ID = command ID (O).

## C1.1 Model registry

| Item | Value |
|---|---|
| Electron device filter | any HID device with VID 0x1D57 (O) |
| requestDevice filters | built per (VID, PID) with `collectionType` "comm" and "listener" (O) |
| Comm interface (config R/W) | usagePage 0x000B usage 0; fallback 0xFF00 usage 1 (O) |
| Listener interface (input events) | usagePage 0x000A usage 0; fallback 0xFF01 usage 2 (O) |
| Shared 2.4G receiver | 0x1D57:0xFA60 for all models except X8Pro (0xFA65) |
| Shared USB PID | 0x1D57:0xFA61 (X3, X6, R1) |

Identification: byte `o[0]` of a listener input report is the **deviceId** (O). R1/X3/X6 all report 0x10; they are split by version report
(cmd 0x0B, bytes [4..7] all 128 = X3, all 129 = X6) (O).

| Model | deviceId | USB PID | Dongle PID | Mode 0 / 1 |
|---|---|---|---|---|
| **F1** | 22 | 0x2416 | 0xFA60 | wired / 2.4G |
| **X11 PRO V2** | 169 | 0x22A9 | 0xFA60 | wired / 2.4G |
| X3 | 128 | 0xFA61 | 0xFA60 | wired / 2.4G |
| X6 | 129 | 0xFA61 | 0xFA60 | wired / 2.4G |
| R1 | 16 | 0xFA61 | 0xFA60 | wired / 2.4G |
| X3Max | 78 | 0x201C | 0xFA60 | |
| X11 / X11 SE / SE contour / SE V2 | 85 / 7 / 20 / 232 | 0xFA55 / 0x2125 / 0x2414 / 0x2158 | 0xFA60 | |
| X2PRO | 18 | 0x2412 | 0xFA60 | |
| X8Pro / X8Plus / X8SE | 8 / 6 / 2 | 0x2126 / 0x2124 / 0x2120 | 0xFA65 (listener FF00/1, usage FF01/2) / 0xFA60 / 0xFA60 | X8Pro 8K |
| G3 / G3Pro | 4 / 14 | 0x2122 / 0x212C | 0xFA60 | |
| V3 PRO / V3 / V2SE / V6 / M2 / INNO X3V2 | 229 / 234 / 151 / 36 / 44 / 166 | 0x2155 / 0x215A / 0x2297 / 0x2224 / 0x222C / 0x22A6 | 0xFA60 | |

## C1.2 Transport

| Item | Value |
|---|---|
| Write config | `sendFeatureReport(buf[0], buf.slice(1))`; 2 retries, 120 ms wait (O) |
| Read config | step 1 send read request (below); step 2 poll `receiveFeatureReport(0xA0)` up to 6x at 160 ms until `(o[0]==0xA0 && o[1]==1) \|\| o[0]==1`; step 3 wait 120 ms; step 4 `receiveFeatureReport(cmd)`; prepend report ID if the browser stripped it; retry up to 6x with 300 ms (O) |
| Read request | feature report 0xA0, 8 bytes: `[A0, cmd, lenLo, lenHi, profile, 0, 0, 0]` (O) |
| Queue | serialized; a 300 ms isWriting guard suppresses status events (type 14) (O) |
| Ack (listener) | `[deviceId, 0x50, status, cmd]`; status 0 = OK. Cmds 5, 6, 8 wait up to 1200 ms for ack, retry 2x at 180 ms; cmd 4 waits 1500 ms, 3 attempts at 180 ms (O) |
| Checksum | 16-bit big-endian sum of `buf[3 .. end-of-payload]`, stored right after the payload (see per command) (O) |
| 8K framing (X8Pro dongle FA65) | output report 4, 64 bytes: `[04, 5+len, frameIdx, data..., sumHi, sumLo]` (59 payload bytes per frame, BE sum over `[0..3+len-1]`); replies C0 (frame ack, `[1]==1` OK), C1 (cmd ack), C2/C3 (reject); 50 ms between frames, 3000 ms timeout (O). Not used by F1 / X11 PRO V2 (I) |

Common header (O): `[0]` = cmd = report ID, `[1]` = total frame length (cmd 4: 56, 5: 15, 6: 9, 8: 59), `[2]` = profile (1..5).
Worked example, cmd 6 set polling 1000 Hz, profile 1 (value 1, `[4] = ~1 & 0xFF`): `06 09 01 01 FE 00 00 00 00`.

## C1.3 Commands (report ID = cmd)

| cmd | Function | Size | Layout |
|---|---|---|---|
| 0x04 | DPI + sensor | 56 | `[1]`=56, `[2]` profile; `[3..4]` sensor flags (below); `[5]` stage mask `[1,3,7,15,31,63,127,255][count-1]`; `[6]` X high-speed flags, `[7]` Y high-speed flags; `[8+i]` DPI low byte and `[16+i]` DPI high byte for stage i (0..7), one value per stage (X = Y); `[24]` active stage index+1; `[25+3i..27+3i]` RGB per stage; `[49]` indicator type 1/2/3; `[50..51]` sum(3..49) BE |
| 0x05 | Lighting | 15 | `[2]` profile; `[3]` mode<<4 (X2PRO: mode&15); `[4]` `(speed&15) \| (sleep2 & 0xF0)`; `[5]` `(brightness&15) \| ((sleep2&15)<<4)`; `[6..8]` RGB; `[9]` sleep level 1 minutes; `[10]` keyRespond/2; `[11..12]` sum(3..10) BE |
| 0x06 | Polling | 9 | `[1]`=9, `[2]` profile; `[3]` value; `[4]` `~[3]&0xFF` (no sum) |
| 0x08 | Buttons | 59 | `[2]` profile; 18 slots at `[3+3i..5+3i]` = {type, code1, code2}; `[57..58]` sum(3..56) BE |
| 0x09 | Macro | see C1.6 | |
| 0x0B | Firmware version (read) | 8 | `[2].[3]` = major.minor |
| 0x0C | Active profile (read) | 9 | `[2]` id 1..5, `[3]` `~id` |
| 0xA0 | Read request | 8 | see C1.2 |
| - | Factory reset | | no dedicated command: re-send cmds 8, 5, 6, 4 with defaults, 666 ms between each (O) |
| - | Pairing | | no command found (U) |

Sensor flags in cmd 0x04 (O):

| deviceId set | `[3]` | `[4]` |
|---|---|---|
| normal | angle snap | ripple |
| {8, 6, 22, 44, 129} ("isX8Paw3395", includes **F1 = 22**) | `(LOD<<4) \| ripple` | `(angleSnap<<4) \| motionSync` |

Read-back mirrors it: `lod = (o[3]>>4)&15`, `motionSync = o[4]&15` (O). X11 PRO V2 (169) has no LOD (default 1) and no motion sync (O).

Polling values (O): normal 125=8, 250=4, 500=2, 1000=1. X8Pro 1..7 = 125, 250, 500, 1000, 2000, 4000, 8000 with nibble duplicated `(v<<4)|v` (read `(o[3]>>4)&15`);
wired capped at 1000. **F1 and X11 PRO V2: normal table, 1000 Hz max (I, from their supportedReportRate usb list)**.

DPI value encoding by deviceId (strategy per device, O). **F1 (22) and X11 PRO V2 (169) use the direct strategy**:

| Strategy | deviceIds | Encoding of each stage |
|---|---|---|
| Y direct | 151, 166, **169**, 18, **22** | `raw16 = dpi` (clamped to 65535); lo = `[8+i]`, hi = `[16+i]`; max 30000 (169), 40000 (22) |
| J | 128, 129 | `raw16 = round(dpi/50)-1` (lo `[8+i]`, hi `[16+i]`); max 26000 |
| Q | 8, 6 | <=26000: `(dpi-50)/50`; above: X high-speed bit set, `floor((dpi-1)/100)`; max 40000 |
| q | 44 | as Q, both `[6]` and `[7]` bits |
| G | 78 | `r=(dpi-50)/50`; if `r>=600`: high-speed bit, `r=((r-599)*50+30000)/100`; max 42000 |
| $ | 36, 229, 234, 4, 14 | table 1..235 (to 12000), x2 up to 24000, x3 up to 25000 via `[6]/[7]` bits |
| X | others | table 1..235 = 50..10000 step 50, 4x flag up to 40000 |

All strategies share the layout: low byte array `[8..15]`, high byte array `[16..23]`, high-speed bits in `[6]` (and `[7]` for strategies q and $). The decoder
reads `x = (hi<<8 | lo)`, X = Y (O).

## C1.4 Light modes and per-model defaults

| Model | Light modes | Notes |
|---|---|---|
| F1 (22) | none (supportedLight empty) | DPI max 40000, 6 stages, defaults [1200, 2400, 3200, 5600, 8000, 40000]; all sensor options (angle, ripple, motion sync, LOD, keyRespond) |
| X11 PRO V2 (169) | 0 static, 1 breathing, 2 mono cycle, 3 rainbow, 4 marquee, 5 color flow, 6 APM, 7 off | DPI max 30000, defaults [1200, 2400, 3600, 4800, 8000, 30000]; no LOD, no motion sync |
| X2PRO (18) | 0 off, 1 static, 2 breathing, ... 11 | speed inverted (9-speed), DPI max 12800 |

## C1.5 Button action types (code1 / code2) (O)

| type | Meaning | type | Meaning |
|---|---|---|---|
| 1 | disabled | 17 | keyboard (code1 = modifier mask, code2 = HID usage) |
| 2/3/4 | left / right / middle | 18 | macro (`code2 = keyValue+1`; X8Pro `code1=0`) |
| 5 / 6 | back / forward | 21 | open player |
| 7 | double click | 22/23 | prev / next |
| 9/10/11/12 | wheel up / down / left / right | 24/25 | play / stop |
| 13 | DPI cycle | 26/27/28 | mute / vol+ / vol- |
| 14 / 15 | DPI+ / DPI- | 29..38 | calc, mail, favorites, web fwd, web back, web stop, my computer, refresh, home, search |
| 16 | easy aim (sniper) or mouse button in combo | 41 | light mode key |
| 48 | consumer, usage = `code1 \| code2<<8` (233 vol+, 234 vol-, 226 mute, 205 play, 183 stop, 182 prev, 181 next) | 60/0 | mode key |

F1 default map (I): slots 0..2 = L, R, M; 3 = 0; 4 = back; 5 = forward; 16 and 17 = types 9 and 10 (wheel). 18 slots total.

## C1.6 Macro (cmd 0x09) (O)

| Variant | Layout |
|---|---|
| Normal (F1, X11 PRO V2) | 512-byte buffer: `[0]` type, `[1..3]` RGB, `[4]` count&255, `[24..25]` event count BE, events from `[26]`, sum([0..125]) at `[126..127]`. Sent as 3 packets of 64 bytes: `[09, 4+len, slot+1, chunkIdx, data...]`, chunks carry 60, 60, 8 bytes; 666 ms between packets |
| X8Pro | one 111-byte command `[09, 111, slot(1..32), type, R, G, B, count, nEvents, events..., sum(3..108) @109..110]`; USB split in 64 + 52 bytes, 50 ms gap |

Event = pair `(delay|0x80 if key-up, code)`; delay unit 10 ms when >127; delays >1270 ms add an extra `(n, 3)` pair; max 50 events; deviceId 166 and 8 use a 1 ms unit.
Mouse buttons in macros are codes 241..245 via map {0:241, 1:243, 2:242, 3:244, 4:245}.

## C1.7 Device to host events (listener interface, input report)

`o[0]` = deviceId, `o[1]` = event type (O).

| Type | Meaning | Layout |
|---|---|---|
| 0x40 / 0x41 | Battery | `[2]` state: 1 discharging, 2 full, 3 charging (charge flag = state 2 or 3); `[3]` level. deviceIds 16, 128, 129: level 1..10, percent = `level*10`; everyone else raw percent (O) |
| 0x10 | connection / id | |
| 0x43, 0x80 | other (U) | |
| 0x50 | config ack | `[2]` status, `[3]` cmd |

Online watchdog: 20 s offline timeout, checked every 10 s (O). 8K wrapper (report 4) passes only types 0x10, 0x40, 0x41, 0x43, 0x50, 0x80.

---------------------------------------------------------------------------------------------------

# C2. R5 Ultra (XVI/JXC platform, "ATTACK SHARK GAMING")

Same XVI/JXC platform as R6 / M5 Ultra / aurora dongle: see `family-e.md` for those PIDs. Share one implementation.

## C2.1 Model registry

| Model | VID | PID wired | PID 8K dongle | Bootloader PIDs | Notes |
|---|---|---|---|---|---|
| R5 Ultra | 0x373E | 0x0046 | 0x0047 | 0xB046 / 0xB047 | `IsNewProtocol=1` (profile byte present), DPI max 42000, LOD 0.7 / 1 / 2 mm |

Interface choice (O): command interface = the HID device whose feature report has `reportCount == 64`; event interface = the device with input report ID 4.
Polling range: wired 125..1000, wireless up to 8000, dongle 1K / 4K / 8K (O). Usage page / usage not extracted (U).

## C2.2 Transport

| Item | Value |
|---|---|
| Send / read | `sendFeatureReport(0, u8[64])` / `receiveFeatureReport(0)` (O) |
| Request layout | `[0]=0 [1]=0 [2]=dev [3]=payloadLen [4]=cat [5]=cmd [6..]=data`; dev 2 = mouse, 0 = dongle, 1 = device-info (O) |
| Get | `cmd \| 0x80` (O) |
| Response | status at `[1]` (0xA1 = OK), cat `[4]`, cmd at `[6]` (hidIndex 0, report ID prepended) or `[5]` (hidIndex 1); detect by `r[6]==cmd` vs `r[5]==cmd` (O) |
| Retry | status > 0xA1: resend; < 0xA1: poll up to 30x then resend; 5 outer attempts (O) |
| Numbers | big-endian multi-byte (O) |
| New protocol | `[6]` = profile (1..5), value from `[7]` (O) |
| Inter-command delay | configurable "commonDelay", default not confirmed (I ~30 ms) |

## C2.3 Commands (cat / cmd; `P` = profile byte at `[6]`)

| Function | Layout | Notes |
|---|---|---|
| Firmware version | dev 2 or 0, len 0, cat 0, cmd 0x81 | bytes at `[7..10]` (hidIndex 0) |
| Battery | `[2]=2 [3]=2 [5]=0x83` | response `o[1]==0xA1 && o[4]==2 && o[6]==0x83`: **`[7]` = charge flag, `[8]` = percent**. Polled, no event (O) |
| Profile set / get / list | cat 1, cmd 0x05 / 0x85 / 0x86 | |
| Polling | `[3]=2 [4]=1 [5]=0x00 [6]=P [7]=code` | get 0x80, value at `[8]`; codes 125=8, 250=4, 500=2, 1000=1, 2000=32, 4000=64, 8000=128; read value 16 normalized to 1 |
| Debounce | `[3]=2 [5]=0x08 [6]=P [7]=ms` | get 0x88 |
| Sleep | `[3]=3 [5]=0x07 [6]=P [7..8]=seconds BE` | get 0x87; UI sends minutes*60 |
| LOD | cat 1, cmd 0x08 | `[7]` = 1 or 2, or `(mm*10) \| 0x80`; get 0x88 |
| Angle snap / ripple / motion sync / tracking / hyper | cat 1, cmds 0x04 / 0x0A / 0x09 / 0x13 / 0x0B | value layout I (1 byte at `[7]`) |
| DPI stage get | `[3]=10 [4]=1 [5]=0x81 [6]=P [7]=6` | `[8]` = count, then 4 bytes per stage (X BE, Y BE) from `[9]` |
| DPI stage set | `[3]=26 [5]=0x01 [6]=P [7]=count [8..]` | same 4-byte stage layout |
| Active DPI stage | set `[5]=0x02 [6]=P [7]=stage`, get 0x82 | |
| DPI stage colors | `[3]=19 [4]=2 [5]=0x01 [6]=P [7..]` RGB | get 0x81 |
| DPI max / sensor model | cat 1, cmds 0x8C / 0x8F | read-only |
| Light effect | `[2]=dev [3]=5+n [4]=2 [5]=0 [6]=P [7..10]=params [11..]=RGB` | modes 1, 2, 4, 5, 6; breathing speed values 28, 48, 68, 88, 108, 128 for speeds 1..6; mode mapping I (no setter call site in UI) |
| Lightness | `[3]=3 [4]=2 [5]=0x02 [6]=P [7]=zone [8]=value` | UI converts 0..255 to percent |
| Button set | `[3]=0 [4]=3 [5]=0 [6]=P [7]=btnIdx [8]=0 [9]=actionType [10]=paramLen [11..]=params` | get 0x80 |
| Macro alloc | `[3]=6 [4]=4 [5]=1 [6..7]=slot BE [8..11]=size BE` | slots 1..3 |
| Macro data write | `[3]=0 [4]=4 [5]=3 [6..7]=slot [8..11]=offset [12]=len [13..]` | 51-byte chunks (read: 50) |
| Macro size / data get / delete | `[5]=0x81` / `0x83` / `[3]=2 [4]=4 [5]=2 [6..7]=slot` | |
| Device VID/PID | `[2]=1 [3]=6 [5]=0x8B [6]=2` | VID `[9..10]`, PID `[11..12]` |
| Reset profile | `[3]=1 [4]=0 [5]=0x0D [6]=P` | |
| Factory reset | `[3]=2 [4]=0 [5]=0 [6]=0xC0 [7]=1` | variant `[7]=2` exists |
| Enter bootloader | `[2]=dev [3]=1 [6]=0xB0` | out of scope, do not implement |
| Pairing | button action type 15 only; no pair command frame (U) | |

Button action types (O): 0 off, 1 button code, 2 X-click, 4 keyboard (params modifier, HID usage), 5 consumer (usage BE), 6 power, 7 DPI, 8 profile, 9 FN,
10 tilt, 12 battery button, 13 report rate, 14 LOD, 15 pairing, 16/17/18 macro I/II/III (params slot BE + repeat), 19 lighting, 20 HP mode.
Button codes: 1 left, 2 right, 3 middle, 4 back, 5 forward, 6..8 buttons 6..8, 16/17 scroll up/down, 18/19 DPI up/down, 20/21 DPI loop up/down,
28/29 polling loop, 30/31 polling up/down, 33/34 LOD up/down, 35/36 LOD loop, 48..50 dongle buttons.
DPI action params: 1 stage up, 2 stage down, 5 lock (+ X BE, Y BE twice), 6/7 cycle up/down. Profile / polling / LOD loop params: 1 up, 2 down, 3 loop up, 4 loop down.
Modifier bits: 1 LCtrl, 2 LShift, 4 LAlt, 8 LGUI, 16 RCtrl, 32 RShift, 64 RAlt, 128 RGUI.
Consumer usages: 184 eject, 226 mute, 234 vol-, 233 vol+, 182 prev, 183 stop, 205 play/pause, 181 next, 387 media select, 402 calculator, 404 my computer,
406 explorer, 547 WWW home, 551 refresh, 394 mail, 111/112 brightness up/down, 419 select app, 539 copy, 540 cut, 541 paste.
Macro event bytes: 0x02 key down, 0x03 key up, 0x09 modifier down, 0x0A modifier up, 0x01 mouse button (bitmask 1 L, 2 R, 4 M, 8 back, 16 fwd, 0 release),
0x10 wheel (1 up else down), 0x20 / 0x21 / 0x22 / 0x23 delay of 1 / 2 / 3 / 4 bytes (ms).

## C2.4 Events (input report ID 4)

| Bytes | Meaning |
|---|---|
| `04 01 stage Xhi Xlo Yhi Ylo` or `04 04 ? stage X(BE) Y(BE)` | DPI change |
| `04 02 profile` | profile change |
| `04 06 00` / `04 06 01` | dongle link lost / restored |
| `04 07 v` | LOD change; `v<128` = mm, else `(v-128)/10` |
| `04 08 code` | polling change |

Battery has no event on this platform; poll the battery command (O).

---------------------------------------------------------------------------------------------------

# C3. Driver v4: X3 PRO, R2, R3 (Rongyuan platform)

## C3.1 Model registry

All VID 0x3151 (12625), company "AttackShark". A runtime **device id** (u32) is read from the device (cmd 0x8F, C3.5) and matched to the bundled definition
`id`; the definition `name` then selects the protocol class (O). The model is never keyed on PID: 0x4026 (R2, R3), 0x5043 (R2, R3) and 0x503A (X3 PRO, R3) are each shared.
Id lookup (O): USB and 2.4G both send 0x8F; 0xFFFFFFFF is mapped to -1 and falls back to a PID table (e.g. PID 0x402A -> def with PID 0x402B).

| Model | def id | PID | Def name | Sensor | DPI max | Class (by name) | Default DPI (idx) |
|---|---|---|---|---|---|---|---|
| X3 PRO (older) | 1660 | 0x402A | mouse_x3_8k | PAW3395 | 26000 | Pan1080 8K (O) | 800,1600,2400,3200,5000,26000 (2) |
| X3 PRO (newer) | 3015 | 0x503A | mouse_ry6608_qfn48_x3pro_3395_8k | PAW3395 | 26000 | CH585 (O) | same; lowLatency 808, noShowFPS20000 |
| R2 | 1893, 3009 | 0x4026 | mouse_pan1080_r2b_3950_1k_8k | PAW3950 | 42000 | Pan1080 | 1200,2400,3200,5600,8000,42000 (1) |
| R2 (newer) | 3016 | 0x5043 | mouse_ry6601_qfn48_r2_1k_8k | PAW3950 | 42000 | CH585 | same; lowLatency 109 |
| R3 | 1643 | 0x4026 | mouse_r3_1k_8k | PAW3395 | 26000 | Pan1080 | 800,1200,1600,3200,5600,26000 (2) |
| R3 (newer) | 3309, 3310 | 0x5043 | mouse_ry6601_qfn48_yyx_r3_3395_3m_1k_8k | PAW3950 (as listed) | | CH585 | |
| R3 (newest) | 3485 | 0x503A | ry6608_qfn48_yyx_r3_3395_8k_8k_3m | PAW3395 | | CH585 | |

Count 6 stages, layer 8, step 50, min 50 for all. Boot-mode PIDs (usagePage 0xFF01, usage 1, interface 0, O, not needed): bootMouse 0x402B, 0x5036, 0x503B, 0x5044; bootDongle 0x503E, 0x5046.

Device filters (O), all VID 0x3151:

| Role | PID | usagePage / usage / interface | reportId / size / type |
|---|---|---|---|
| Wired mouse (USB) | 0x4026, 0x402A, 0x503A, 0x5043 | 0xFFFF / 2 / 2 | 0 / 64 / feature |
| 2.4G dongle | many (0x4011, 0x4014, 0x4017, 0x4021, 0x4027, 0x402D, 0x4038, 0x5006..0x505A); which dongle pairs with which Attack Shark mouse is U | 0xFFFF / 2 / 2 | 0 / 64 / feature |
| Bluetooth | none for these models in listing (U) | usagePage 0xFF55/0xFF66, usage 514, interface -1 | output report, `0x55` prefix |

Report ID 0 and size 64 are the defaults applied when a filter has no `reports` list (O). Implementers must still verify against the real HID descriptor.

## C3.2 Transport (O)

| Item | Value |
|---|---|
| Report | feature report, report ID 0, 64-byte payload. App builds `[reportId, ...payload, zero pad to 64]`; payload longer than 64 is refused (O) |
| Command header | `[0]` command ID, `[1..6]` args, `[7]` checksum (Bit7 mode), `[8..]` data |
| Short command | payload shorter than 9 is zero-padded to 9; longer is left as is |
| **Checksum** | `b[7] = 0xFF - ((b[0]+b[1]+...+b[6]) & 0xFF)` (Bit7 mode, used by every mouse command). Includes the command byte. Bit8 mode (`b[8]`, sum of b[0..7]) exists but no mouse command uses it. This is NOT the family A 16-bit BE sum: do not reuse `checksum16` |
| Write then read | send, wait ~10 ms (`SEND_TIME`), `getFeatureReport(0)`, wait ~10 ms (`READ_TIME`); BT: 100 / 200 ms. After every set the app waits `vendorSleep()` (default 100 ms; reset 300 ms) |
| Read response | report ID stripped: **`r[0]` is the command echo**, args follow. Response checksum is not validated by the app (O) |
| Concurrency | one command at a time; background status polling stopped during a transaction |
| Bluetooth | write is an output report, payload prefixed with `0x55`; replies as input reports `0x55 ...` (O) |
| 2.4G dongle | commands to the mouse are relayed by the dongle, see C3.6 |

Worked examples (arithmetic verified, `[7]` marks the checksum byte):

| Command | Bytes `[0..8]` |
|---|---|
| CH585 set polling 1000 Hz (`08 01`) | `08 01 00 00 00 00 00 F6 00` |
| CH585 set polling 8000 Hz (`08 81`) | `08 81 00 00 00 00 00 76 00` |
| Pan1080 set polling 1000 Hz, profile 0 (`04 00 01`) | `04 00 01 00 00 00 00 FA 00` |
| Get version/id (`8F`) | `8F 00 00 00 00 00 00 70 00` |

## C3.3 Two protocol classes (selected by definition `name`)

The two classes have DIFFERENT command ID tables. Implement both, key on the runtime id -> def name.

### Class CH585 (CommonMSCH585): ids 3015, 3016, 3309, 3310, 3485 (newer X3 PRO / R2 / R3)

`P` = byte `[1]` where noted. Set IDs are the low half, get IDs = set | 0x80 for most (O, table below is exact).

| Function | Set ID | Get ID | Set layout | Get response |
|---|---|---|---|---|
| Key matrix (one key) | 0x00 | | `[1]` = slot index, `[8..11]` = 4-byte action (no profile byte) | |
| Key matrix (all) | 0x01 | 0x81 | `[1]` profile, `[2]` chunk 0/1, then 56 bytes from `[8]`. Matrix = 64 bytes (16 slots x 4): chunk 0 = bytes 0..55, chunk 1 = bytes 56..63 zero-padded to 56 | get `[1]` profile, `[2]=1` |
| Current profile | 0x02 | 0x82 | `[1]` profile | `r[1]` |
| Macro | 0x03 | 0x83 | `[1]` slot, `[2]` chunk idx, `[3]` 56, `[4]` 1 if last chunk; `[8..]` 56 bytes x up to 5 | get `[1]` slot, `[2]` chunk 0..3; end when 4 zero bytes seen |
| Debounce | 0x04 | 0x84 | `[1]` ms | `r[1]` |
| Wheel button release time | 0x05 | 0x85 | `[1]` value | `r[1]` |
| 2.4G sleep | 0x06 | 0x86 | `[1..2]` LE16 | `r[1] \| r[2]<<8` |
| Bluetooth sleep | 0x07 | 0x87 | `[1..2]` LE16 | same |
| **Polling rate** | 0x08 | 0x88 | `[1]` code | `r[1]` |
| Gun/recoil macro | 0x09 | 0x89 | chunked like macro | |
| Battery | | 0x8A | | declared in the table, no caller in any bundle (O); do not use |
| Mouse version + ID | | 0x8F | | see C3.5 |
| **DPI params** | 0x10 | 0x90 | `[1]` profile (>7 -> 0), `[2]` active stage index, `[3]` count, `[8+2i..9+2i]` X LE16, `[24+2i..25+2i]` Y LE16, `[40+3i..42+3i]` RGB (8 stages) | `r[2]` current, same arrays |
| Silent height (lift-off) | 0x11 | 0x91 | `[1]` value | `r[1]` |
| Line repair (angle snap) | 0x12 | 0x92 | `[1]` 0/1 | `r[1]==1` |
| Ripple repair | 0x13 | 0x93 | `[1]` 0/1 | `r[1]==1` |
| **Motion sync** | 0x14 | 0x94 | `[1]` 0/1 | `r[1]==1` |
| Sensor FPS 20000 | 0x15 | 0x95 | `[1]` 0/1 | `r[1]==1` |
| Light params | 0x17 | 0x97 | see C3.4 | |
| Sensor angle | 0x18 | 0x98 | `[1]` value | `r[1]` |
| Key low latency | 0x0C | 0x8C | `[1]` value | |
| True 8K over 2.4G | 0x0D | 0x8D | `[1]` 0/1 | |
| Countdown | 0x0B | 0x8B | `[1]` value | |
| Factory reset | 0x0E | | no args | |
| All params 0 | 0x1F | 0x9F | `[8]` profile, `[9]` debounce, `[10]` scroll release, `[11..12]` sleep 2.4G LE, `[13..14]` sleep BT LE, `[15]` rate code, `[16]` silent height, `[17]` line repair, `[18]` wave repair, `[19]` motion sync, `[20..27]` per-slot rate list, `[28]` FPS 20000, `[40]` key low latency | same |

Report-rate codes (both classes, `_encodeReportRate`): 8000=0x81, 4000=0x82, 2000=0x84, 1000=0x01, 500=0x02, 250=0x04, 125=0x08 (O).
Wired USB rate is limited by the registry `reportRate` (0x4026 and 0x5043: 1000; 0x402A and 0x503A: 8000); higher rates on 1K/8K parts need the dongle (I).

### Class Pan1080 (CommonMsPan1080): ids 1660, 1643, 1893, 3009 (older X3 PRO / R2 / R3)

| Function | Set ID | Get ID | Layout |
|---|---|---|---|
| Version / info | | 0x80 | `r[1] \| r[2]<<8`; 8K subclass uses 0xAD for USB version and 0x80 for RF |
| Reset | 0x02 | | no args |
| Wireless sync | 0x01 | | |
| **Polling (quick)** | 0x04 | none | `[1]` profile, `[2]` code; only 1000=1, 500=2, 250=4, 125=8 (anything else sends 0) |
| **Polling (full, 2K/4K/8K)** | 0x53 | 0xD3 | via option param 0 `[9]`: 8000=0x81, 4000=0x82, 2000=0x84, 1000=1, 500=2, 250=4, 125=8 (same encode as CH585); read back from option0 `[9]` |
| Profile | 0x05 | 0x85 | `[1]` profile |
| Key matrix (bulk write) | 0x09 | | `[1]` profile, `[2]`=64, `[3]`=0, `[4]` chunk 0/1, then 56 bytes from `[8]` (chunk 0 = matrix 0..55, chunk 1 = 56..63 zero-padded) |
| Key matrix (read) | | 0xD0 | `[1]` profile; response is the matrix (bulk read uses 0xD0, not 0x89) |
| Key matrix (single key) | 0x50 | | `[1]` profile, `[2]` slot, `[8..11]` action |
| Macro | 0x16 | 0x96 | same chunk scheme as CH585 (`[1]` slot, `[2]` chunk, `[3]` 56, `[4]` last, `[8..]`) |
| Option param 0 (general) | 0x53 | 0xD3 | 64 bytes: `[8]` profile, `[9]` rate code (1/2/4/8 or 129/130/132), `[10]` debounce, `[12..13]` flags LE16 (bit0 light on, bit1 wheel light off, bit2 smooth, bit3 led select, bit4 power save), `[14]` buttonChange, `[15]` wheelToButton, `[16]` buttonToWheel, `[24..31]` light block, `[40..47]` sleep (4 LE16: bt, deepBt, 2.4G, deep2.4G), `[50]` X sensitivity, `[51]` Y sensitivity, `[52]` lift-off cutoff, `[53]` angle snap, `[54]` FPS, `[54..]` battery colors RGB, `[60]` charging switch |
| **DPI params (option 1)** | 0x54 | 0xD4 | `[1]` profile, `[2]` active stage (>8 -> 0), `[3]` count, `[8+2i]` X LE16, `[24+2i]` Y LE16, `[40+3i]` RGB; get: same, current at `r[2]` |
| Countdown | 0x55 | 0x55 | `[8]` value |
| Gun / recoil | 0x60 | 0xE0 | |
| Boot check | | 0xF7 | `r[7]==1 && r[8]==1` = in boot (USB only) |

Pan1080 has no separate LOD / motion sync / ripple commands: only angle snap, FPS and lift-off cutoff inside option param 0 (O); other options are U.

## C3.4 Lighting and key matrix encodings (O)

CH585 light command (64 bytes): `[0]=0x17 [7]=chk [8]=0 if on else 1 [9]=mode [10]=4-speed [11]=brightness [12]=(option<<4)|(7 normal | 8 dazzle) [13..15]=RGB`.
Pan1080 uses an 8-byte block at `[24..31]` of option param 0: `[0]=mode [1]=4-speed [2]=brightness [3]=(option<<4)|(7|8) [4..6]=RGB`.

| Mode byte | Meaning |
|---|---|
| 0 off, 1 always on, 2 breathe, 3 neon, 4 wave | all classes |
| 5 snake (CH585) / dazzling (Pan1080) | differs |
| 6 laser, 7 music follow, 8 screen color, 9 music follow 2 | Pan1080 only |

Speed byte = `4 - speed`. White 0xFFFFFF is sent as 0xFAFFFA (16449530) and decoded back (O).
Color shortcut table (low nibble of `[12]` when not 7 or 8): 0 red, 1 orange, 2 yellow, 3 green, 4 cyan, 5 blue, 6 magenta.

Key matrix: 16 slots x 4 bytes `{type, a, b, c}`, default (R2/X3 PRO): L `01 00 F0 00`, R `01 00 F1 00`, M `01 00 F2 00`, forward `01 00 F4 00`, back `01 00 F3 00`,
slot 5 DPI `14 00 00 00` (or `0A 05 00 00` pair, per model), then reserved, then wheel entries `01 00 F9 01` / `01 00 F9 FF` / `01 00 F5 01` / `01 00 F5 FF`.

| Type byte | Meaning |
|---|---|
| 0 | keyboard combo `{0, modifier, key, key2}` (e.g. Ctrl+C `00 00 E0 06`); `00 00 00 00` / `00 00 03 00` = disabled |
| 1 | mouse: F0 left, F1 right, F2 middle, F3 back, F4 forward (O, matches macro codes); F5..F9 wheel left/right/up/down and X/Y move are I (the vendor table reuses 245 for both wheel-left and scroll +/-1) |
| 2, 3, 6, 8, 10, 13, 14, 18, 19, 22 | functions (consumer `03 00 94 01` = Computer; `08 00 03 00` = profile loop; `0A 05 00 00` = pair; `0D 01 00 00` = LED mode loop; 22 = snap/recoil) |
| 9 | macro `{9, mode, index, 0}`; mode 0 repeat N, 1 on/off, 2 touch repeat |
| 20 | DPI: `14 00 00 00` cycle, `14 00 01 00` DPI+ (O). The vendor DPI- entry `00 02 00 14` looks like a table typo: do not implement (I) |
| 22, 23, 24 | snap / MDS / mouse-tap (`24 k1 k2 time/10`) |

Macro stream (O): first 2 bytes repeat count LE16, then events. Each event = `[code, (down?code+128:code)]`, delay in the second byte when <=127, else
`[code, down?0x80:0x00, delayLo, delayHi]`; mouse move = `F9, delay or 0, dx, dy`; mouse buttons F0..F4 as per type-1 table; 256-byte buffer, 56-byte chunks.

## C3.5 Version, id, battery, events

| Item | Value |
|---|---|
| Version + ID (0x8F) | request `8F`; `r[0]==0x8F`. ID = `getUint32(r, 1, LE)` (this is the id matched against definitions, O). CH585 firmware version = `r[5] \| r[6]<<8`; `r[11]==1` = light sync support (USB only, O). Pan1080 version via 0x80: `r[1] \| r[2]<<8` |
| Battery, USB | hard-coded `100`, online (O); no command used |
| Battery, 2.4G | from the dongle status poll (C3.6): mouse battery = `r[2]` of cmd 0xF7 response. Scale: percent presumed (I); **no charge flag exists in app code (O); not obtainable from this protocol** |
| Battery, Bluetooth | write `[0x77]`; reply type `0x77`: `r[1]` = battery, online; type `0x88` = offline (O). Polled every 5 s |
| Battery cmd 0x8A | in the CH585 table; grep of all dist/js chunks found no caller, so no battery command is used. Wired = 100, 2.4G = dongle 0xF7 `[2]`, no charge flag anywhere in the app (O) |
| Device to host events | none on USB; Pan1080/CH585 are poll-only. BT: 0x55 prefixed input reports |
| Firmware update | 20480-byte (CH585) or 65536 (Pan1080) blocks via `upgrade_usb`; out of scope |

## C3.6 2.4G dongle relay (O)

Same feature report, report ID 0. F7, F6, FE and FC are sent raw (no checksum, not padded to 9); F1 and 8F use the Bit7 checksum (O).

| ID | Function | Layout |
|---|---|---|
| 0xF7 | Status poll | reply: `[0]==1` can read, `[1]` keyboard battery, `[2]` mouse battery, `[3]==0` keyboard online, `[4]==0` mouse online, `[5]==1` can send, `[6]` bitmask 1 = keyboard, 2 = mouse, 3 = both, `[8]==1` RF bootloader |
| 0xF6 | Select target | `[1]` 10 keyboard, 5 mouse, 13 other |
| 0xFE | Length notice | `[1]` length |
| 0xFC | Read notice | `[0xFC]` only; reply read with a following get |
| 0xF1 | Dongle id | reply `[0]==0xF1`, `[1..2]` LE16 |
| 0x8F | Peer id | reply `[1..4]` LE32 |

Flow for a mouse command through a dongle: poll 0xF7 up to 5x at 100 ms until `mouse.isCanSend`, send 0xF6 `[5]`, send the command, then to read poll
0xF7 until `isCanRead`, send 0xFC, then get feature. 10 ms spacing (`INTERVAL_24G_TIME`).

---------------------------------------------------------------------------------------------------

# Capability matrix (what is exposed by the shipped UI/code)

| Capability | F1 | X11 PRO V2 | R5 Ultra | X3 PRO | R2 | R3 |
|---|---|---|---|---|---|---|
| DPI max / stages | 40000 / 6 | 30000 / 6 | 42000 / U | 26000 / 6 | 42000 / 6 | 26000 / 6 |
| Polling max | 1000 (I) | 1000 (I) | 8000 (dongle) | 8000 wired (O) | 1000 wired, 8000 dongle (I) | 1000 wired, 8000 dongle (I) |
| LOD | yes | no | yes (0.7/1/2) | silent height (I) | silent height | silent height |
| Angle snap | yes | yes | yes | yes (line repair) | yes | yes |
| Ripple | yes | yes | yes | yes (wave repair) | yes | yes |
| Motion sync | yes | no | yes | yes | yes | yes |
| Debounce | via keyRespond | via keyRespond | yes | yes | yes | yes |
| Lighting | none | 8 modes | yes | yes | yes | yes |
| Battery | event 0x40/0x41, percent + charge | same | poll, charge + percent | 2.4G only, charge U | same | same |
| Macros | 32 (normal 3-packet) | same | 3 slots | yes | yes | yes |
| Profiles | 5 | 5 | 5 | up to 5 (`layer` 8) | | |
| Pairing | none found | none found | button action only | button action `0A 05 00 00`; Pan1080 cmd 0x01 "wireless sync" has no caller (U) | same | same |

# Unknowns

1. C1: events 0x43 and 0x80; F1 polling above 1000 Hz; header byte `[1]` semantics were read from the builders (length), not from a capture.
2. C2: usage page / usage of the R5 Ultra command and event interfaces; default `commonDelay`; value layout of angle snap / ripple / motion sync / tracking setters; light effect mode mapping.
3. C3: battery charge flag and scale on 2.4G (no field read), cmd 0x8A caller; which dongle PID pairs with which Attack Shark mouse; pairing command; whether Pan1080 models
   have separate LOD, motion sync and ripple commands; Bluetooth for these models; X sensitivity semantics in option param 0; native services (`common_hid_rs.exe`) were
   NOT analysed (only needed if the gRPC layer adds framing, which the JS suggests it does not).
4. C3 R3 PAW3950 vs PAW3395 sensor naming for ids 3309/3310 is as listed by the vendor definition table (O) but looks like a vendor typo (I).
5. Nothing here was validated on hardware. Every layout is from static reading; write commands must be dry-run tested on a spare device first.
