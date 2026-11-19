# Software oficial dos mouses Attack Shark

Fonte: https://attackshark.com/pages/driver-download (links diretos em download.attackshark.pro, sem login). Coletado em 2026-11-19. Arquivos em
`/private/tmp/claude-501/-Users-lucasbrito/7bd6d189-2a0c-412c-9481-53bf99923cab/scratchpad/vendors/<modelo>/` (original + `extracted/`). Nada foi executado. Manual PDF salvo em cada pasta.
Colecao: https://attackshark.com/collections/mouse-support (3 paginas). Datas/versoes: so o Driver v4 Electron traz versao (R2 3.1.12, R3 3.2.20, X3PRO 3.2.24); os demais nao expoem versao.

## Tabela

| Modelo | Fonte | Local | sha256 do pacote | Familia | VID:PID | UI | Notas |
|---|---|---|---|---|---|---|---|
| X6 | https://download.attackshark.pro/ATTACKSHARK/X6/ATTACKSHARKX6.zip | vendors/X6/ | 4825cf3b69ba6ce679ea87b7cc465ac063178deef4ad19c39ffe9657e792071e | A | 1D57:FA60/FA61; BLE 045E (VID&02045E) | DuiLib (Delphi-installer Inno), x86 | X6.exe 637 KB; hiddriver_1/2 e hidapi identicos ao X3; BluetoothApis GATT; FEE3/FEE4 imm |
| X3 | https://download.attackshark.pro/ATTACKSHARK/X3/ATTACKSHARKX3.zip | vendors/X3/ | e7da2e8fa705ecaa0e8aede71daaa5e83984ea78c9fbbddbd5f65f917ec201dd | A | 1D57:FA60/FA61 | DuiLib, x86 | Referencia ja analisada |
| X3MAX | https://download.attackshark.pro/ATTACKSHARK/X3MAX/ATTACKSHARKX3MAX.zip | vendors/X3MAX/ | c39d9a97331de86a59f7ee51acae4aa017440afb4ba03d35c47d774f710e702a | B | 1D57:211D..2224, FA60, FA65 (+045E) | DuiLib, x86 | Instalador compartilhado (AttackShark.exe) |
| X3PRO | https://download.attackshark.pro/ATTACKSHARK/X3PRO/ATTACKSHARKX3PRO.zip | vendors/X3PRO/ | 80d4bd198fbc70b4ea9e3382079c1d990a3385d8f1e1ab4231d08d0022cd9867 | C | lista WebHID (3151,25AA,342D,374A...); sem 1D57 no bundle | Electron 3.2.24 (Driver v4, iot_manager) | Protocolo vem do servidor (grpc-web); HidSharp/common_hid_rs.exe |
| X11 | https://download.attackshark.pro/ATTACKSHARK/X11/ATTACKSHARKX11.zip | vendors/X11/ | 98a9075f2f00f1a37cfb6e147d0bbaad35a2a2242944d753bab85c9dcc52cc6e | B | idem B | DuiLib | instalador B |
| X11SE | https://download.attackshark.pro/ATTACKSHARK/X11SE/ATTACKSHARKX11SE.zip | vendors/X11SE/ | 001813624c643cd081c7e4b8f114ccbe576dd1a4a8f5ee3b924f2530af260a94 | B | idem B | DuiLib | instalador B |
| X11PRO V2 | https://download.attackshark.pro/ATTACKSHARK/X11PRO/ATTACKSHARKX11PROV2.zip | vendors/X11PRO/ | 22780bd0f759d6655cd1b53b463a65ac38c533b763b0cded99de2e5800d24ee3 | C | 1D57:201C,2120,2125,2155,2158,2224,2297,22A6,22A9,FA55,FA60,FA61 | Electron/Next.js, WebHID | asar 66 MB |
| X11 Ultra | https://download.attackshark.pro/ATTACKSHARK/X11Ultra/ATTACKSHARKX11Ultra.zip | vendors/X11Ultra/ | 7c443700bd35978c43bf07ac5dcc4fa724232678754a64a8ed4d4cb3114d4643 | D | nao extraido | InstallShield 16 + MSI (Mouse Hub) | payload cifrado |
| R1 | https://download.attackshark.pro/ATTACKSHARK/R1/ATTACKSHARKR1.zip | vendors/R1/ | 7d8ad5490535b9f0ac794c762f6c03559fa23c55f1a8121f9be17011da44491e | B | idem B | DuiLib | instalador B |
| R2 | https://download.attackshark.pro/ATTACKSHARK/R2/ATTACKSHARKR2.zip | vendors/R2/ | 8d25e91c71714775dca57b65a84a200d8d6f2d6d2d0ff21912171cbf6fb2fdf2 | C | lista WebHID Driver v4 (3.1.12) | Electron (Driver v4) | iot_driver.exe |
| R3 | https://download.attackshark.pro/ATTACKSHARK/R3/ATTACKSHARKR3.zip | vendors/R3/ | b4e8c6d0c8f7baedb88c1e4f15ff9a64b2801bdcc579c258f83c1b35518fe48b | C | lista WebHID Driver v4 (3.2.20) | Electron (Driver v4) | iot_manager |
| R5 Turbo | https://download.attackshark.pro/ATTACKSHARK/R5Turbo/ATTACKSHARKR5TURBO.rar | vendors/R5Turbo/ | 45d63eeed5606389188aefcddbdfbd2b34418408481b3fc48e63bbe184a4538f | D | nao extraido | InstallShield 16 + MSI (Mouse Hub) | RAR5; extraido com unar (7zz falha) |
| R5 Ultra | https://download.attackshark.pro/ATTACKSHARK/R5Ultra/ATTACKSHARKR5.zip | vendors/R5Ultra/ | c18d4cc35e5082bf8300df72c0b61b378e2c294ee8c9fe2e414b744acb6384f4 | C | WebHID (sem VID/PID literal) | Electron ("ATTACK SHARK GAMING", asar 30 MB) | navigator.hid + sendFeatureReport |
| R6 | https://download.attackshark.pro/ATTACKSHARK/R6/ATTACKSHARKR6.zip | vendors/R6/ | 94255398c87940f7ae0dea5648b6455649a4d6468580e8bd66deb721a368250b | E | tabela xvi_models.xlsx (VID/PID com/sem fio) | .NET (DriverCore.exe), 7z SFX | plataforma XMG; nao usa hiddriver.dll |
| R11 Ultra | https://download.attackshark.pro/ATTACKSHARK/R11ULTRA/ATTACKSHARKR11ULTRA.zip | vendors/R11ULTRA/ | 888f6ef75a8b93d98bff4d7a0e411753f95023956e9cca6a70f9f78955fca4fa | D | nao extraido | InstallShield 16 + MSI | mesmo exe do V5/X8 Ultimate |
| V2 Air | https://download.attackshark.pro/ATTACKSHARK/V2AIR/ATTACKSHARKV2AIR.rar | vendors/V2AIR/ | 3f4f071acf4fd5420dbc399a24770040602e82a4dacefd821e58fb0d4d5a3ed3 | D | nao extraido | InstallShield 16 + MSI (Mouse Hub) | mesmo exe do F1 Air/RS6 |
| V3 | https://download.attackshark.pro/ATTACKSHARK/V3/ATTACKSHARKV3.zip | vendors/V3/ | 4b7c3015de33c648f74223b580a55ddb962c857440a578ac4935ba3a0f16c161 | B | idem B | DuiLib | instalador B |
| V3 PRO | https://download.attackshark.pro/ATTACKSHARK/V3PRO/ATTACKSHARKV3PRO.zip | vendors/V3PRO/ | ff8077cb73f9c061ede7ba7a58471d4e6d6c40944ed095977453fefea38973e0 | B | idem B | DuiLib | instalador B |
| V5 | https://download.attackshark.pro/ATTACKSHARK/V5/ATTACKSHARKV5.zip | vendors/V5/ | 306cf30bd9933ebd51ce41529dd839adebd5034a468c8f698d1e4d8af06eb4c9 | D | nao extraido | InstallShield 16 + MSI | mesmo exe do R11 Ultra |
| V6 | https://download.attackshark.pro/ATTACKSHARK/V6/ATTACKSHARKV6.zip | vendors/V6/ | 29f117ad6873357d6d2896f6f5c804aca45fca51a3d5a400419d02ed917362f4 | B | idem B | DuiLib | instalador B |
| V8 | https://download.attackshark.pro/ATTACKSHARK/V8/ATTACKSHARKV8.rar | vendors/V8/ | 00807ec2eb36e9c3397aae1d1e31b1f4ddfef448a53e971887e9fa060557d52d | D | nao extraido | InstallShield 16 + MSI | exe 32.8 MB, ISO 2025-12 |
| X1 | https://download.attackshark.pro/ATTACKSHARK/X1/ATTACKSHARKX1.zip | vendors/X1/ | ce822ec67dc5755a19f7d8363d904fe04d9f7bb2d199a1d6ad96a0e165d366a3 | B | idem B | DuiLib | instalador B |
| X8 (Plus/SE/Pro) | https://download.attackshark.pro/ATTACKSHARK/X8Series/ATTACKSHARKX8.zip | vendors/X8Series/ | d84fc77c2fadcd897e65330b2a2aff7180bb26ca6cc9228f7ba608e09bf00a9e | B | idem B | DuiLib | instalador B; 3 manuais |
| X8 Ultra | https://download.attackshark.pro/ATTACKSHARK/X8Ultra/ATTACKSHARKX8ULTRA.zip | vendors/X8Ultra/ | ce8c346c9f3577eac5c04f37ff2b8c1cc0d39df75721c9fc9afcfb686e27e72a | D | nao extraido | InstallShield 16 + MSI (Mouse Hub) | mesmo exe do RS3 |
| X8 Ultimate | https://download.attackshark.pro/ATTACKSHARK/X8ULTIMATE/ATTACKSHARKX8ULTIMATE.zip | vendors/X8ULTIMATE/ | 1911665a090b66778fc02c23f8ed69ac3b96237c9a23af10190b4b73c2fa80e2 | D | nao extraido | InstallShield 16 + MSI | mesmo exe do V5 |
| G3 | https://download.attackshark.pro/ATTACKSHARK/G3/ATTACKSHARKG3.zip | vendors/G3/ | dc9450027bc070afbc6bb4b0ad4f600fe406efa3cf6d12cb3a56a23e57c22350 | B | idem B | DuiLib | instalador B |
| G3 PRO | https://download.attackshark.pro/ATTACKSHARK/G3PRO/ATTACKSHARKG3PRO.zip | vendors/G3PRO/ | 92e0374d240e7ce2c6744283a41c7115b698fb43835efab7f83ee2d993a980fb | B | idem B | DuiLib | instalador B |
| F1 | https://download.attackshark.pro/ATTACKSHARK/F1/ATTACKSHARKF1.zip | vendors/F1/ | 0a1ecab3ecba1b7d5f80d81382b472a766796d3404f274d11e469acc382a2c16 | C | 1D57:201C,2120,2122,2124..2126,212C,2155..215A,2224,222C,2297,22A6,22A9,2412..2416,FA55,FA60,FA61,FA65 | Electron/Next.js, WebHID | asar 72 MB |
| F1 Air | https://download.attackshark.pro/ATTACKSHARK/F1Air/ATTACKSHARKF1Air.rar | vendors/F1Air/ | 3f4f071acf4fd5420dbc399a24770040602e82a4dacefd821e58fb0d4d5a3ed3 | D | nao extraido | InstallShield 16 + MSI (Mouse Hub) | RAR5 |
| RS3 | https://download.attackshark.pro/ATTACKSHARK/RS3/ATTACKSHARKRS3.zip | vendors/RS3/ | ce8c346c9f3577eac5c04f37ff2b8c1cc0d39df75721c9fc9afcfb686e27e72a | D | nao extraido | InstallShield 16 + MSI (Mouse Hub) |  |
| RS6 Ultra | https://download.attackshark.pro/ATTACKSHARK/RS6/ATTACKSHARKRS6ULTRA.rar | vendors/RS6/ | 3f4f071acf4fd5420dbc399a24770040602e82a4dacefd821e58fb0d4d5a3ed3 | D | nao extraido | InstallShield 16 + MSI (Mouse Hub) | RAR5 |
| MEGA B2 | https://download.attackshark.pro/ATTACKSHARK/B2/ATTACKSHARKMEGA-B2.rar | vendors/B2/ | 16527aa28da5a0c596ad7c9898fe7fcaeb6df17172248e45cd73a9070bb0abcf | D | nao extraido | InstallShield 16 + MSI (Mouse Hub) | RAR5 |
| AJ159P (AJAZZ) | https://download.attackshark.pro/ATTACKSHARK/AJ159P/AJ159P.zip | vendors/AJ159P/ | 5dca0ee16751efb0b5ec785f0c7a235fe3fec6fbd1813d885eda583aaa62509b | F | 248A:5C2E/5D2E.. (config.xml, Inno) | MFC (mfc140u), x86 | AJAZZ Driver (X) 1.0.7.0; mouse_aj*.xml |

Hashes de instalador: o mesmo exe e reutilizado entre modelos (familia B = um unico `ATTACK SHARK x SOFT.exe`, sha256 prefixo b54043ca726a, 7.6 MB; V5=R11 Ultra=X8 Ultimate prefixo a92c927ff18f; RS3=X8 Ultra prefixo 8f0e66941aee; F1 Air=RS6=V2 Air prefixo 3f4f071acf4f).

## Familias

### A. DuiLib legado (X3, X6) - representante: X3 (ja analisado)
- X3.exe/X6.exe ~640 KB, DuiLib XML (res/*.xml), `hidapi.dll` sha f4e23fe71828, `hiddriver_1.dll` = `hiddriver_2.dll` sha f9efbf063021 (identicos entre X3 e X6), imports BluetoothApis (GATT), constantes imm 0x1D57 / 0xFA60 / 0xFA61 e UUID 0xFEE3/0xFEE4 nos dois.
- Strings: `Set_VIDPID`, `Open_DevMonitor`, `SetFeature`, `GetFeature`; "VID&02045E" (BLE HID) em ambos.
- O X6 e byte-a-byte a mesma arquitetura do X3: mesmo HID DLL, mesmo VID/PID, so muda o tema/res. Basta um backend.

### B. DuiLib multi-modelo "AttackShark.exe" (X3 MAX, X11, X11 SE, R1, V3, V3 PRO, V6, X1, X8, G3, G3 PRO) - representante: X3 MAX
- Mesmo instalador para os 11. `AttackShark.exe` 1.7 MB, DuiLib, 129 res xml, `hiddriver_1..18.dll` (todos sha f9efbf063021, o mesmo da familia A) + `hiddriver_4k_1.dll` (b28a38bb25a5, variante 4K/8K).
- VID 0x1D57 com PIDs 211D, 211F, 2120..2126, 212C, 2155, 215A, 2224, FA60, FA65 (e 045E para BLE). Mesmo API SetFeature/GetFeature + BluetoothApis.
- Mesma camada HID da A: reverter o X3 cobre o transporte; B acrescenta tabelas por modelo e o 4K.

### C. Electron / WebHID (X3 PRO, R2, R3, R5 Ultra, F1, X11 PRO V2)
- C1 "Driver v4" (R2, R3, X3 PRO): Electron, `iot_driver.exe`/`iot_manager`, HidSharp, protocolo baixado do servidor (grpc-web); `navigator.hid` + `sendFeatureReport` + reportId. Lista com ~100 VIDs de terceiros (0x3151, 0x25AA...). Mesma plataforma do site iotdriver.qmk.top.
- C2 Next.js em asar (F1, X11 PRO V2, R5 Ultra): WebHID puro; F1/X11 PRO listam 1D57:FA55/FA60/FA61/FA65 e as faixas 20xx-24xx. Os report IDs ficam no bundle JS (`_next/static/chunks/app/page-*.js`), prontos para ler (nao ofuscado de forma evidente).
- Representante: F1 (asar legivel, lista VID/PID completa, WebHID com reportId/usagePage). Util para obter o protocolo em JS em vez de engenharia reversa de binario.

### D. InstallShield 16 + MSI "Mouse Hub" (V5, V8, B2, F1 Air, R5 Turbo, RS3, RS6, V2 Air, X8 Ultra, X8 Ultimate, R11 Ultra, X11 Ultra)
- Setup.exe do InstallShield com MSI embutido e payload cifrado: 7zz, innoextract e unar nao abrem. Precisa de Windows (`setup.exe /extract_all:<dir>`) ou de rodar o instalador em VM. NAO extraido.
- Ha so 4 exe distintos (a92c927ff18f, 8f0e66941aee, 3f4f071acf4f, e os de B2/R5 Turbo/V8/X11 Ultra), logo 4-5 extracoes cobrem a familia.

### E. .NET "XMG" (R6)
- `DriverCore.exe` (.NET, 7z SFX), Config/xvi_models.xlsx com colunas VID/PID com e sem fio, BLVID/BLPID (bootloader), MI, polling. Sem hiddriver.dll nem hidapi. Fora do escopo do X6.

### F. AJAZZ MFC (AJ159P)
- Inno + MFC, VID 0x248A (AJAZZ/Telink), config.xml por modelo. Nao e Attack Shark HID; so de interesse residual.

## Manual / nao baixavel
- Sem software listado (so manual PDF, baixado): A2, G6, X5, R11 (o R11 Ultra tem instalador). Provavelmente usam os drivers web (https://controlhub.top/AttackShark/, https://inputdrv.com/Attack-Shark/, https://iotdriver.qmk.top/, https://www.xvalleyinno.top/AttackShark/): nao baixados.
- Familia D: baixado mas nao extraido (InstallShield cifrado).
- Nao-mouse ignorados: M71 (teclado Ximeng, baixado por engano, descartavel), teclados e headsets.
- Total em disco ~4.6 GB (inclui extracoes) (limite 6 GB).
