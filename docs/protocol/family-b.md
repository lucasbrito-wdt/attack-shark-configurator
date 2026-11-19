# ATTACK SHARK family B - multi-model `AttackShark.exe` protocol spec

Reverse-engineered statically from the DuiLib multi-model configurator shipped for X3 MAX, X11,
X11 SE, R1, V3, V3 PRO, V6, X1, X8, G3 and G3 PRO. Nothing was executed.

| Artifact | sha256 / note |
|----------|---------------|
| `AttackShark.exe` (x86 PE, DuiLib) | `e0d78c3aae2883fb3bd0e3e464d020a1f5d036b5ae9126f7c631bdefd3879daa` |
| `hiddriver_1..7,9..18.dll` | all identical (`f9efbf06...`), same as family A `hiddriver_x.dll`. No `hiddriver_8.dll` |
| `hiddriver_4k_1.dll` | `b28a38bb25a5672a9c774abc30ee000ba7a08f8b44ef98adcf96f9168ab97b34`, extra exports `Open_ReportDevice`, `ReadUSB`, `WriteUSB`, `Set_Device_Version`, `Close_ReportDevice` |
| `hidapi.dll` | used directly by the exe for the 2.4 GHz dongles |

Tags and notation follow [PROTOCOL.md](../PROTOCOL.md) section 0 (**O** observed, **I** inferred,
`sub_XXXXXX` = address in `AttackShark.exe`, `P+0xNNN` = profile offset). Anything not listed
here as changed is byte-identical to family A, and this spec points to the PROTOCOL.md section.

---

## 0. Answer first: battery over BLE

| Question | Answer | Tag |
|----------|--------|-----|
| Is there a "get battery" request command (USB or BLE)? | **No.** No command in any page builds a battery query. Battery is unsolicited only. | O |
| Does the app read GATT Battery Level `0x2A19`? | **No.** `BluetoothGATTGetCharacteristicValue` is not imported. Only `0xFEE4` notify is registered (`sub_40b4d0`), and writes go to `0xFEE3` (`sub_40acd0`). | O |
| BLE battery event format | FEE4 notification `Data[0..1]` = type u16 LE, `Data[2..3]` = value u16 LE (Ghidra-confirmed, `FUN_00497fb0`). Type = `0x4000 \| model_code`. | O |
| Who handles it over BLE | MS5..MS18 BLE handlers process `0x40xx`. **MS1..MS4 (codes `0x85`, `0x4D`, `0x10`, `0x55`) return immediately on `0x40xx` over BLE**: the vendor never displays battery for those models on BLE. | O |
| Our hardware (`FA61`/`FA60`/BLE `045E:0040`) | Matches the **R1 page (MS_3, code `0x10`)**, one of the ignored ones. The observed `10 10 02 00` = type `0x1010` = DPI-stage event of model `0x10` (section 4). | O |
| Conclusion | Over BLE, this firmware class is not expected to push battery; the vendor does not rely on it. Accept `0x4010` if it ever arrives (decode as section 4.2), otherwise show "unknown". The only BLE-side source is BAS `0x2A19`, which is unreliable (100/101 observed). The reliable battery path is the 2.4 GHz dongle (`0x4010` heartbeat every few s). | O (code), I (firmware) |

---

## 1. Model registry

### 1.1 Pages, IDs, model code

The app holds 18 page objects (`MS_1..MS_18`, one C++ object each, built in `sub_40d6b0` for MS1-6
and in per-page ctors for MS7-18). All share the 2.4 GHz dongle `1D57:FA60`. The **model code**
is the low byte of every event type (section 4) and equals the low byte of the BLE PID, except R1.

| MS | Model (UI) | Wired PID | BLE PID | Code | Wired DLL | USB event handler | BLE notify cb -> handler | Profile base |
|----|-----------|-----------|---------|------|-----------|-------------------|--------------------------|--------------|
| 1 | X6 | `2055` | `0085` | `0x85` | `hiddriver_1` | `sub_484bd0` | `sub_4868d0` -> `sub_486910` | `+0x5dba8` |
| 2 | X3 | `201B` | `004D` | `0x4D` | `hiddriver_2` | `sub_48df60` | `sub_48ee50` -> `sub_48ee90` | `+0x5dba8` |
| 3 | R1 | `FA61` | `0040` | `0x10` | `hiddriver_3` | `sub_496710` | `sub_497f60` -> `sub_497fb0` | `+0x5dba8` |
| 4 | X11 | `FA55` | `0055` | `0x55` | `hiddriver_4` | `sub_4a1690` | `sub_4a2fa0` -> `sub_4a3000` | |
| 5 | X3 MAX | `201C` | `004E` | `0x4E` | `hiddriver_5` | `sub_4ac710` | `sub_4ae150` -> `sub_4acab0` | `+0x5dbac` |
| 6 | X1 | `2111` | `00B1` | `0xB1` | `hiddriver_6` | `sub_4b61f0` | `sub_4b7c40` -> `sub_4b64d0` | |
| 7 | X11 PRO | `211D` | `00BE` | `0xBE` | `hiddriver_7` | `sub_4c0fd0` | `sub_4c3aa0` -> `sub_4c1930` | `+0x10517c` |
| 8 | X3 PRO | `211F` | `0001` | `0x01` | **`hiddriver_4k_1`** | `sub_4ccba0` | `sub_4cf890` -> `sub_4cd470` | `+0x105174` |
| 9 | V5 SE | `2121` | `0003` | `0x03` | `hiddriver_9` | `sub_4d7a90` | `sub_4d9950` -> `sub_4d7f70` | |
| 10 | X11 SE | `2125` | `0007` | `0x07` | `hiddriver_10` | `sub_421e10` | `sub_423d40` -> `sub_4220f0` | |
| 11 | X8 SE | `2120` | `0002` | `0x02` | `hiddriver_11` | `sub_42ea90` | `sub_4304a0` -> `sub_42ed70` | |
| 12 | X8 Plus | `2124` | `0006` | `0x06` | `hiddriver_12` | `sub_439730` | `sub_43b400` -> `sub_439a10` | |
| 13 | X8 PRO | `2126` | `0008` | `0x08` | `hiddriver_13` | `sub_444330` | `sub_446a50` -> `sub_444860` | |
| 14 | G3 | `2122` | `0004` | `0x04` | `hiddriver_14` | `sub_44f540` | `sub_450ae0` -> `sub_44f820` | |
| 15 | G3 PRO | `212C` | `000E` | `0x0E` | `hiddriver_15` | `sub_459890` | `sub_45ae30` -> `sub_459b70` | |
| 16 | V3 PRO | `2155` | `00E5` | `0xE5` | `hiddriver_16` | `sub_463bb0` | `sub_465150` -> `sub_463e90` | |
| 17 | V6 | `2224` | `0224` | `0x24` | `hiddriver_17` | `sub_46e400` | `sub_46f9a0` -> `sub_46e6e0` | |
| 18 | V3 | `215A` | `00EA` | `0xEA` | `hiddriver_18` | `sub_478c60` | `sub_47a200` -> `sub_478f40` | `+0x10517c` |

All VIDs: wired and dongle `0x1D57`; BLE `0x045E` (path match `VID&02045E` + `PID&xxxx`). O.
BLE callbacks are registered through `sub_40b4d0(cb)` from `sub_411440`. O.
Page object offsets in the main window: MS1 `0xbe0`, MS3 `0xcc518`, MS8 `0x384958`, MS13 `0x91d548`,
MS18 `0xeb9478` (others in the work notes). O.

### 1.2 Connection modes and how a page is chosen

| Link | Enumeration | Page selection | Tag |
|------|-------------|----------------|-----|
| Wired | Each page's `hiddriver_N.Set_VIDPID(0x1D57, wiredPID)`; usage page `0x0B`/usage `0` = config (SetFeature/GetFeature), `0x0A`/`0` = events. Identical to PROTOCOL.md 1.2. | By PID (1:1 with the page). | O |
| Wired X3 PRO (`hiddriver_4k_1`) | Three collections, all on `1D57:211F`: usage page `0xFF00` usage `1` ("OSD", input), `0x0B`/`0` ("FEATURE"), `0xFF01` usage `2` ("REPORT", output via `WriteUSB`). `FUN_10002960` (Ghidra, dll). | By PID. | O |
| Dongle `FA60` (shared) | `sub_4130c0`: `hid_enumerate(0x1D57, 0xFA60)`, opens every interface with usage page `0x0B`/usage `0` (config) and `0x0A` (events), up to 20; one reader thread each (`Sleep(10)` + `sub_4137b0`, `hid_read` 100 bytes). | **By model code**: the first input report's `buf[1]` (`sub_4137b0` -> `sub_413a20` binds the dongle handles to that page). Map: `85`->MS1, `4D`->2, `10`->3, `55`->4, `4E`->5, `B1`->6, `BE`->7, `01`->8, `03`->9, `07`->10, `02`->11, `06`->12, `08`->13, `04`->14, `0E`->15, `E5`->16, `24`->17, anything else->18. | O |
| 4K dongle `FA65` | `sub_415100` / `sub_415ec0`: `hid_enumerate(0x1D57, 0xFA65)`, collections usage page `0xFF00` usage `1`/`2` (and one variant with usage page `0x0A`), filtered on `bcdDevice` low byte `0x16` or `0x18`. Readers `sub_40d590..` / `sub_40d620..` -> `sub_4154e0` / `sub_4162b0`. | By `buf[1]` of the first report, **only** `0xBE` (X11 PRO), `0x01` (X3 PRO), `0x08` (X8 PRO). Others are ignored. | O |
| BLE | Windows GATT, service with `FEE3` (write) / `FEE4` (notify), path match `PID&<blePID>`. | By BLE PID. | O |

`option_device_N` (left device list) maps 1:1 to `MS_N` (`sub_4129a0`, "any device" flag `page+0x3E4`). O

---

## 2. Per-model capabilities

DPI is stored per stage as **units** `u` (slider value, `min=1`). Display formula `sub_40f9a0`:
`DPI = 50*u` if `u <= T`, else `(u - T/2) * 100`. So the step is 50 DPI up to `50*T`, then 100. O

| MS | Model | Slider max | T | DPI max | 0x04 encoding (3.1) | Polling options | Lighting UI | LOD / motion sync UI |
|----|-------|-----------|---|---------|---------------------|-----------------|-------------|----------------------|
| 1 | X6 | 520 | none | 26000 | A (PROTOCOL.md) | 125-1000 | yes | yes |
| 2 | X3 | 520 | none | 26000 | A (shares MS1 code) | 125-1000 | no | yes |
| 3 | R1 | 280 | 200 | 18000 | **T** (table) | 125-1000 | yes | no |
| 4 | X11 | 320 | 200 | 22000 | T | 125-1000 | yes | no |
| 5 | X3 MAX | 720 | 600 | 42000 | **H600** | 125-1000 | no | yes |
| 6 | X1 | 660 | 520 | 40000 | H520 | 125-1000 | no | yes |
| 7 | X11 PRO | 340 | 200 | 24000 | T | 125-1000 + 4K dongle 125-8000 | no | no |
| 8 | X3 PRO | 520 | none | 26000 | A (+4 profile offsets) | 125-8000 (7) | no | yes |
| 9 | V5 SE | 340 | 200 | 24000 | T | 125-1000 | no | no |
| 10 | X11 SE | 340 | 200 | 24000 | T | 125-1000 | no | no |
| 11 | X8 SE | 350 | 200 | 25000 | T | 125-1000 | no | no |
| 12 | X8 Plus | 660 | 520 | 40000 | H520 | 125-1000 | no | yes |
| 13 | X8 PRO | 660 | 520 | 40000 | H520 | 125-1000 + 4K dongle 125-8000 | no | yes |
| 14-18 | G3, G3 PRO, V3 PRO, V6, V3 | 350 | 200 | 25000 | T | 125-1000 | no | no |

- DPI stages: 8 sliders on every page; per-stage enable mask as family A. MS18 shows stages 1-6 only. O (XML)
- Buttons: 18 key slots on every page; button map packet as PROTOCOL.md 3.4. O
- Lighting UI = `check_light` not hidden in the page XML (I: other pages keep the packet but hide the panel).
- LOD / motion sync UI = `check_lift_of_distance` / `check_motion_sync` not hidden. O (XML), I (meaning).
- The `ver_prate_hs` panel (125..8000 Hz, 7 options) exists on MS7 and MS9-18, but only MS7, MS8 and MS13
  are ever bound to a 4K dongle, and MS9-18 apply code encodes only 4 rates (section 3.3). O
- BLE: every model has a BLE PID. Polling (cmd `0x06`) is never sent over BLE. O

---

## 3. Commands (host -> device): diff vs family A

Framing, checksum, report ID = `buf[0]` and the send/retry pattern are as in PROTOCOL.md 2.1/2.2.
`0x08` button map (PROTOCOL.md 3.4) and `0x09` macro (3.5, chunking 2.3) are unchanged on the
legacy transports. `0x0C` resync is unchanged: `0C 0A 01 FE 01 FE 00 00 00 00` (`sub_4ce410`,
`sub_479120`, `sub_496da0`). O

### 3.1 `0x04` Sensor + DPI (len `0x38`)

Header `04 38 01`; checksum hi, lo = sum of `[3..49]` at `[50]`, `[51]`; `[52..55]` = 0 (all variants). O

**Variant A** (MS1/MS2 `sub_4859d0`, MS8 `sub_4ce810`): identical to PROTOCOL.md 3.1
(`enc = u-1`, lo bytes `[8..15]`, hi bytes `[16..23]`). MS8 reads every field at profile offset +4
(`[3]`=`P+0x8B4` LOD, `[4]`=`+0x944`, `[6]`=`+0x948`, `[7]`=`+0x94C`, colours from `+0x964`) and
sends `[49] = 0x03` instead of `0x01`. O

**Variant T** (table, R1 `sub_4970d0`, V3 `sub_4794f0`, also `sub_4a2050`, `sub_4c2a60`,
`sub_4d8900`, `sub_422df0`, `sub_42f320`, `sub_44fdd0`, `sub_45a120`, `sub_464440`, `sub_46ec90`):

| Byte | R1 (MS3) | V3 (MS18) | Tag |
|------|----------|-----------|-----|
| 3 | `P+0x944` (I: angle snap) | `P+0x948` (I: motion sync) | O (source) |
| 4 | `P+0x940` (I: ripple) | `P+0x944` (I: angle snap) | O (source) |
| 5 | stage enable mask (bit i = stage i) | same | O |
| 6 | bit i = stage i has `u > 220` | bit i = `221 <= u <= 340` | O |
| 7 | same as `[6]` | bit i = `u >= 341` | O |
| 8..15 | sensor code per stage (below), `0` if `u == 0` | same | O |
| 16..23 | half-step flag: `1` if `201 <= u <= 220` or `u >= 301`, else `0` | same, but `0` for `u >= 341` | O |
| 24 | active stage + 1 | same | O |
| 25..48 | 8 x RGB from `P+0x960` (R1) / `P+0x964` (V3), stride 4 | same | O |
| 49 | `0x02` | `0x02` | O |

Sensor code (tables at `0x4eb674`/`0x4eb4e4`/`0x4eb994` for R1, `0x4ea044`/`0x4e9eb4`/`0x4ea364`/`0x4ea048`
for V3; contents identical where ranges overlap; `ceil(a/b)` = integer ceiling). O (table dump), formula verified against every entry:

| Units `u` | DPI | Code | Exceptions |
|-----------|-----|------|------------|
| 1..200 | 50..10000 | `ceil(2*DPI/85) - 1` | `u=6` -> `6` (formula gives 7) |
| 201..300 | 10100..20000 | `ceil(DPI/85) - 1` | none |
| 301..340 | 20100..24000 | table, u=301..340: `235 118 118 119 119 121 121 122 122 123 123 124 124 125 125 127 127 128 128 129 129 130 130 131 131 132 132 134 134 135 135 136 136 137 137 138 138 139 139 141` | `u=301` -> 235 looks like a vendor off-by-one |
| 341..350 | 24100..25000 | `189 190 191 191 192 194 194 195 196 199` | |

I: the sensor resolution register is in 42.5-DPI steps (`code+1`), with `[6]`/`[7]`/`[16..23]` selecting
x2 / x4 / x3 multipliers. Verify on hardware with the read-back of section 3.6.

**Variant H** (MS5 `sub_4ad3a0` with `T=600`; MS6 `sub_4b6dc0`, MS12 `sub_43a380`, MS13 `sub_4459d0` with `T=520`):

| Byte | Value | Tag |
|------|-------|-----|
| 3 | `(P+0x8B4 << 4) \| P+0x940` (LOD, ripple) | O (source), I (names) |
| 4 | `(P+0x944 << 4) \| P+0x948` (angle snap, motion sync) | same |
| 5 | stage enable mask | O |
| 6 | bit i = stage i has `u > T` (high range) | O |
| 7 | `0x00` | O |
| 8..15 / 16..23 | `enc` lo / hi, `enc = u - 1` if `u <= T` (DPI/50 - 1), else `enc = DPI/100 - 1` | O |
| 24 | active stage + 1 | O |
| 25..49 | MS5: all `0`. MS6/12/13: RGB from `P+0x960`..., `[49] = 0x02` | O |

### 3.2 `0x05` Lighting (len `0x0F`)

Same layout as PROTOCOL.md 3.2 in every page. Two differences. O
- Brightness goes in the low nibble of `[5]` when mode is **1 or 5** (family A: 1 or 9). Otherwise it is `8`.
- MS7-18 read profile offsets +4: mode `P+0x950`, RGB `P+0x960..962`, debounce `P+0x95C`. Sleep is still `P+0x930`.

### 3.3 `0x06` Polling (len 9): `06 09 01 v ~v 00 00 00 00`

| Pages | `v` | Tag |
|-------|-----|-----|
| MS1-6, MS9-12, MS14-18 (e.g. `sub_496da0`, `sub_479120`) | as family A: idx 0..3 -> `08 04 02 01` (125/250/500/1000 Hz). Idx 4..6 (hs panel) fall through to `0x02` on MS18 | O |
| MS7 X11 PRO `sub_4c26a0`, MS13 X8 PRO `sub_445610` | **dual nibble**: `v = ((hs+1) << 4) \| (ls+1)`. `ls` = `P+0x938` (0..3 = 125..1000 Hz), `hs` = `P+0x93C` (0..6 = 125, 250, 500, 1000, 2000, 4000, 8000 Hz; the 4K-dongle rate). Defaults 3/3 (`v = 0x44`) | O |
| MS8 X3 PRO `sub_4ce410` | **interval in 250 us units**: idx 0..6 = 125..8000 Hz -> `20 10 08 04 02 01 40` (8000 Hz uses `0x40`). Idx is clamped to 3 (1000 Hz) while on the `FA60` dongle (`sub_4cf950`) | O (bytes), I (unit) |

### 3.4 4K transport framing (new)

The 4K-capable pages MS7 (`sub_4c1f50`), MS8 (`sub_4cda60`) and MS13 (`sub_444e80`) wrap every command
when they are on the 4K dongle `FA65` or wired through `hiddriver_4k_1`. O

| Path | Write | Tag |
|------|-------|-----|
| 4K dongle | `hid_write(handle, buf, 0x41)` (output report) | O |
| Wired X3 PRO | `hiddriver_4k_1` function at `page+0x50` (I: `WriteUSB`, usage page `0xFF01`), 0x40 bytes | O (call), I (export) |
| `FA60` dongle | unchanged: `hid_send_feature_report(page+0x11de24, pkt, len)`, 10 tries, 500 ms | O |
| BLE | FEE3 write (`sub_40acd0`) and 500 ms gap, skipped for cmd `0x06` | O |

Normal command `pkt[0..len)`:

| Byte | Value |
|------|-------|
| 0 | `0x04` (report ID) |
| 1 | `len + 5` (total frame length) |
| 2 | `0x00` (sequence) |
| 3..len+2 | `pkt` as built in 3.1-3.3 |
| len+3, len+4 | sum of bytes `[0..len+3)`, hi then lo |

Macro `0x09` (111-byte packet, macro encoder `sub_4ceb70`, header `09 6F <btn>`) goes as 2 frames, 1000 ms apart:
frame 0 = `04 40 00` + `pkt[0..59)` + sum`[0..62)` hi, lo; frame 1 = `04 39 01` + `pkt[59..111)` + sum`[0..55)` hi, lo. O
On `FA60` the same macro uses 3 feature chunks `09 40 <btn> <idx>` + 60 bytes, with chunk 2 header `09 0C` (as PROTOCOL.md 2.3). O

### 3.5 Macro `0x09` in newer pages

MS8 `sub_4ceb70` builds a 111-byte (`0x6F`) packet, not family A's 131. MS18 calls `sub_42f810`, which uses the same encoder shape (`F6`/`FE` markers); its header was not checked.
- `[0..3]` = `09 6F <btn> <macro slot>`.
- Then 2 or 4 bytes per event. A key event is `(delay | 0x80 if release) , key`. A long delay (>= 128) is `delay % 100`, then `delay / 100`, then `0x03`. A mouse event is `F6` + x/y.
- `<n> FE` ends a block at more than 98 bytes. The continuation goes as the next block index.
- `[0x6D..0x6E]` = checksum.

The exact event table is **not fully decoded** (section 5). O (shape), I (fields)

### 3.6 Read-back (new): request `0xA0` + GetFeature

The page re-reads its device state on connect (`sub_4989f0`, R1; equivalents in every page). O

| Step | Bytes | Tag |
|------|-------|-----|
| Request | SetFeature `A0 <id> <len> 00 <flag> 00 00 00` (8 bytes), up to 10 tries, 100 ms. Then GetFeature `0xA0` (8 bytes): ok when `resp[1] == 1` (`sub_499220`) | O |
| Version | request `id=0x0B len=8`, GetFeature `0x0B` (8): `ver = resp[2] << 8 \| resp[3]` -> `page+0x67634` | O |
| Bulk (if `ver & 0xFF00 != 0x1000`) | GetFeature `0x0A`, 0x80 bytes, parsed as the BLE `A0` blob below | O |
| Split (if `ver & 0xFF00 == 0x1000`) | `0x06` len 9 (`resp[3]` polling: 1->idx3, 2->2, 4->1, 8->0); `0x05` len 15 (`sub_4996e0`); `0x04` len `0x38` (`sub_4993c0`); `0x08` len `0x3B` (`sub_499790`). Each has 5 retries and checks `resp[0]==id`, `resp[1]==len` | O |
| BLE | write FEE3 `A1 08 00 00 00 00 00 00`, `A2 08 ..`, `A0 08 ..` 100 ms apart. Replies come as FEE4 notifications whose `Data[0]` is `A0`/`A1`/`A2`/`A9` (section 4.3) | O |

These are the only "get" operations. None returns battery. O

---

## 4. Device -> host events

### 4.1 Framing

| Link | Raw | Tag |
|------|-----|-----|
| Wired / `FA60` / `FA65` | input report `03 TT TT VV VV ...`, type = `b[1] \| b[2]<<8`, value = `b[3] \| b[4]<<8`. Report ID must be 3. Each report whose `b[1]` equals the bound model code re-arms the 6000 ms link watchdog (timer `0x200 + 0x10*page`, 4K: `0x2d4`...). | O (`sub_413fe0`, `sub_415790`) |
| BLE FEE4 | `Data[0..1]` type LE, `Data[2..3]` value LE. If `Data[0]` is `A0`/`A1`/`A2`/`A9`, the payload is a read-back reply instead (4.3). | O |

**Type = `category | model_code`.** The handler returns unless `(type & 0xFF) == code`, then switches on
`type & 0xFF00`. So family A's `0x1000`/`0x2000`/... appear on the wire as e.g. `0x1010`, `0x2010`. O

### 4.2 Categories (all models)

| Category | Value | Meaning / app action | Tag |
|----------|-------|----------------------|-----|
| `0x10` | 1..8 | Device changed DPI stage: `P+0x934 = v-1`, OSD popup (msg `0x50b`). **`10 10 02 00` = model `0x10` switched to stage 2.** | O |
| `0x20` | 1..8 | Polling changed: `P+0x938 = v` (index semantics as family A). MS7/MS13: low nibble = `ls+1` -> `P+0x938`, high nibble = `hs+1` -> `P+0x93C`. MS8: triggers a profile resync. | O |
| `0x40` | lo = state, hi = level | Battery + heartbeat, watchdog re-arm. Codes `0x85`/`0x4D`/`0x10`: level 1..10 = 10..100 %. All others: level 0..100 = %, `> 100` ignored. Wireless: state `3` = charging (animation, "Charging..."), else show level. Wired (`page+0x3DC`/`0x3E0`): state `2` = full ("100%"), else charging animation. **BLE: ignored for `0x85`/`0x4D`/`0x10`/`0x55`**; MS5-18 BLE: state 2 = 100 %, 3 = charging, else level. | O |
| `0x50` | status | Write ack: USB `sete` into the ack flag (`page+0x3F8`/`0x424`). BLE: ack = `(value & 0xFF) == 0`. | O |
| `0x60` | 1..10 | Stored at `page+0x174`/`0x180`, never shown (as family A). | O |
| `0x70` | `v` | Light mode = **`(v >> 4) & 0xF`** -> `P+0x94C` (`0x950` on MS7+). Family A used raw `v`. | O |
| `0x80` | 1..5 | Save / resync only. | O |

Family A's state byte (`lo != 0` = awake) has no equivalent in family B. B uses `lo` as a charge state (2 full, 3 charging). I: state 0/1 = discharging/awake.

### 4.3 BLE read-back replies (`FUN_00497fb0` R1, `sub_47a250` V3)

| `Data[0]` | Payload | Tag |
|-----------|---------|-----|
| `A0` | state blob. `Data[1]` polling: `1`->idx3, `2`->2, `4`->1, else 0. Then sensor block (`sub_4993c0`), lighting (`sub_4996e0`) and button map (`sub_499790`) at the same offsets as the USB bulk read. Saves and hides "query_ring". | O |
| `A1` | firmware version `Data[1] << 8 \| Data[2]` | O |
| `A2` | ignored by every page | O |
| `A9` | multi-part block: `Data[1]` = index, the rest goes to a 200-byte slot (`+0x65c9c + idx*200`, V3 `+0x11de8c`). Marker `0xFE` -> the app sends `A9 08 <n-1>` and arms a 1000 ms timer (`0x224`/`0x326`). | O |

### 4.4 Test vectors

| Wire | Decoded |
|------|---------|
| FEE4 `10 10 02 00` | model `0x10`, DPI stage -> 2 (`P+0x934 = 1`) |
| FEE4 `10 10 01 00` | model `0x10`, DPI stage -> 1 |
| USB `03 10 40 01 07` | model `0x10`, battery state 1, level 7 -> 70 % |
| USB `03 4E 40 00 55` | X3 MAX, battery 85 % |
| USB `03 BE 20 42 00` | X11 PRO polling ls idx 1 (250 Hz), hs idx 3 (1000 Hz) |
| USB `03 4E 70 30 00` | X3 MAX light mode 3 |

---

## 5. Unknowns

| # | Item | Why it matters | How to settle |
|---|------|----------------|---------------|
| 1 | **`0x04` DPI encoding on our `FA61` device.** Family A (`X3.exe`) sends `enc = u-1`. Family B's R1 page (same PID `FA61`, code `0x10`) sends variant T table codes. One of them is stale for this firmware. | A wrong DPI packet silently sets a different DPI. | Send one variant T packet, then read back with `A0 06/04` (3.6) or compare the cursor speed. Family A values below 10000 DPI are close to `T` codes x 1.18. |
| 2 | Whether the `FA61` firmware ever pushes `0x4010` over BLE | Battery on BLE | Log FEE4 for more than 10 min across low-battery thresholds and while charging. The vendor ignores it anyway. |
| 3 | Byte names in `0x04[3]`/`[4]` for variant T (angle snap / ripple / motion sync) | UI toggles | Toggle one at a time and read back via `A0`. |
| 4 | Variant T multiplier semantics of `[6]`/`[7]`/`[16..23]` | DPI above 10000 | Hardware check. The table is exact regardless. |
| 5 | MS8 polling unit (250 us) and the `0x40` code for 8000 Hz | Exact bytes are O; meaning is I | USB capture with the vendor app. |
| 6 | Macro event table for the 111-byte `0x09` (`sub_4ceb70`, `sub_42f810`) | New-model macros | Decompile further if macros on MS7+ are needed. |
| 7 | `FA65` variant with usage page `0x0A` and `bcdDevice 0x16` (`sub_415100`) vs `0xFF00`/`0x18` (`sub_415ec0`): which handle is `page+0x11de2c` vs `+0x11de34` | 4K dongle open | Enumerate a real 4K dongle. |
| 8 | Battery `lo` state values other than 2/3 | Charge UI | Observe on dongle while charging/discharging. |
| 9 | Exact wired-4K output path (`page+0x50` = `WriteUSB` assumed) | X3 PRO wired | Ghidra the X3 PRO ctor that fills `page+0x40..0x60`. |
