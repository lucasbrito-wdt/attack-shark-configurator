---
title: Plano de design - ATTACK SHARK X3 Configurator
status: proposta em arena (duas direções)
fonte-de-verdade: docs/PROTOCOL.md (seções 3, 4, 5 e 6)
---

# Plano de design — configurador do mouse ATTACK SHARK X3

> Preenchido antes da primeira linha de código. Duas direções disputam na arena
> (`.design-arena/variantes/claude-a.html` e `claude-b.html`). A arquitetura de informação,
> o modelo de estados, a tipografia-base de cada direção e a curva de movimento estão
> decididos aqui; a escolha entre A e B é do usuário.

## 0. Projeto

- **Stack e versões (alvo):** Tauri v2 + Vue 3 (Composition API, `<script setup lang="ts">`) + Vite + Tailwind CSS v4 (CSS-first, sem `tailwind.config.js`) + `motion-v` para movimento. Hoje o repositório é um app Rust (egui/DuiLib-like) em reestruturação para `crates/`; nenhuma camada web existe ainda.
- **Design system / tokens existentes:** nenhum na camada web. O app atual reproduz o skin do fabricante (PNG 1127x760, vermelho/preto). Esse skin não é herdado: é exatamente o que o app novo substitui.
- **Componentes que já resolvem parte disto:** nenhum componente Vue existe. O que se reaproveita é o **modelo**: a tabela de ações (PROTOCOL 3.7), os grupos de escrita com dirty flags (3), a semântica de bateria (4.4), a struct de perfil (6.2) e as strings do `lan.xml` (5).
- **Convenções a seguir:** código, nomes de token e identificadores em inglês; texto de interface em inglês (o app do fabricante é em inglês e o usuário pediu strings em inglês). Formulários validam na borda (DPI 50..26000 passo 50; sleep 1..60; response 2..25). Loading e erro sempre na mesma superfície: o **dock de aplicação** (ver seção 4).
- **Restrições da stack:** fontes self-hosted via `@fontsource-variable/*` (OFL), sem CDN em runtime no app final. Janela Tauri com `titleBarStyle: Overlay` no macOS (semáforos sobre a sidebar) e decorações nativas no Linux. Janela padrão 1180x760, mínima 960x640. Protótipos usam `@tailwindcss/browser@4` e Motion vanilla por CDN **só no protótipo**.

## 1. Assunto

- **O que é, concretamente:** o configurador desktop (macOS + Linux) do mouse gamer sem fio ATTACK SHARK X3 — sensor óptico de até 26000 DPI, 8 estágios de DPI com cor por estágio, polling 125..1000 Hz, três links (cabo USB, receptor 2.4G e Bluetooth LE, onde o sistema o vê como "X6 Mouse2"), cinco botões remapeáveis, macros, LED indicador sob o scroll e bateria reportada em degraus de 10 %.
- **Para quem:** o jogador que comprou um mouse de US$ 30 a 50 e não tem Windows, ou não quer o app Windows do fabricante. Conhece DPI e polling de nome; não conhece "ripple control" nem "motion sync" — a interface explica cada um em uma frase.
- **O único trabalho desta tela:** deixar o mouse exatamente como a pessoa quer e **provar que ele está assim** — o que está no mouse, o que ainda não foi enviado e o que o mouse mudou sozinho.
- **Vocabulário do assunto:** carcaça (shell) em ABS fosco, scroll emborrachado, LED indicador de estágio, sensor óptico (um CMOS que fotografa a superfície milhares de vezes por segundo), lift-off, skates de PTFE, placa com máscara de solda verde, contatos dourados (ENIG), switches, debounce, dongle, heartbeat, estágio, "counts per inch".

## 2. Cor

Duas direções, cada uma com cinco matizes de origem declarável. As cores de estágio do DPI **não são paleta**: são dado do usuário (bytes RGB do pacote `0x04`), então o protótipo as guarda como trios RGB, igual ao protocolo.

### Direção A — "Shell & Stage Light" (o lado de fora do mouse)

| Token | Hex | De onde vem |
| --- | --- | --- |
| `--color-shell` | `#E9EDF1` | O ABS branco fosco da carcaça do X3 sob luz ambiente: branco frio, nunca creme. Chão da janela. |
| `--color-skate` | `#FBFCFD` | Os skates de PTFE da base: o branco mais limpo do objeto. Superfícies elevadas (sidebar, painéis). |
| `--color-wheel` | `#1B232B` | A borracha do scroll e o cabo: grafite azulado, não preto tintado. Texto e ação primária. |
| `--color-led` | `#0FB4D2` | O LED ciano sob o scroll no render do fabricante. Foco, seleção e progresso. Para texto e traço usa-se a sombra `--color-led-deep` `#076F83`. |
| `--color-sensor` | `#C42B37` | O brilho vermelho da iluminação do sensor visto sob o mouse. Exclusivo para erro e bloqueio. |

Derivados (não contam como matiz): `--color-wheel-muted` `#4E5B67` (texto secundário), `--color-shell-deep` `#D3DAE1` (bordas e trilhos).

Contraste verificado com `scripts/contraste.mjs`:
- corpo `#1B232B` sobre `#E9EDF1`: **13.50:1** (AAA); sobre `#FBFCFD`: **15.47:1**.
- secundário `#4E5B67` sobre `#E9EDF1`: **5.92:1** (AA).
- erro `#C42B37` sobre `#FBFCFD`: **5.45:1** (AA).
- `#087A90` sobre `#E9EDF1` deu 4.25:1, por isso o texto em ciano usa `#076F83`.

### Direção B — "Inside: Solder Mask & Sensor Vision" (o lado de dentro do mouse)

| Token | Hex | De onde vem |
| --- | --- | --- |
| `--color-mask` | `#0D3B33` | A máscara de solda verde da placa do mouse. Chão da janela, em gradiente para `#0A2C27`. |
| `--color-silk` | `#E8F1EA` | A serigrafia branca das legendas da placa. Texto. |
| `--color-pad` | `#E2BC5C` | Os pads e contatos com banho de ouro (ENIG). Ação primária, uma por tela. |
| `--color-lens` | `#7FD9C0` | O verde-água da lente acrílica do sensor quando iluminada. Seleção, foco e o campo de visão do sensor. |
| `--color-fault` | `#FF8A7A` | O LED vermelho de status da placa. Exclusivo para erro e bloqueio. |

Derivados: `--color-via` `#0A2C27` (poços e campos), `--color-silk-muted` `#A9C9BC` (texto secundário).

Contraste verificado:
- corpo `#E8F1EA` sobre `#0D3B33`: **10.77:1** (AAA).
- secundário `#A9C9BC` sobre `#0D3B33`: **6.96:1** (AA).
- texto do botão `#0D3B33` sobre `#E2BC5C`: **6.85:1** (AA).
- erro `#FF8A7A` sobre `#0D3B33`: **5.42:1** (AA).

## 3. Tipo

- **A: Mona Sans (OFL, variável em peso 200..900 e largura 75..125).** Escolhida porque o eixo de largura permite um único sistema fazer três papéis sem segunda família: título largo e leve (o "calmo" pedido pela referência), corpo em largura normal, números de DPI em largura 125 com `tabular-nums` para que 800 e 26000 não dancem. Não é a grotesca que eu usaria por inércia (Inter, Geist): tem desenho mais humano nas curvas, combina com a carcaça arredondada.
- **B: Archivo (OFL, variável em peso 100..900 e largura 62..125).** Grotesca de origem técnica (catálogo, instrumento). Display em largura 125 e peso 200 para os números grandes; rótulos em largura 100 e peso 560; dados pequenos em largura 112. A largura expandida lê como instrumento sem cair na fonte "gamer" quadrada.
- **Papéis:** display (o número de DPI, o nome do dispositivo) só uma vez por tela; corpo 14 px; legenda 11.2 px. Distinção por largura e peso, nunca por caixa alta.
- **Escala:** razão 1.25 (terça maior) a partir de 14 px: 11.2 / 14 / 17.5 / 21.9 / 27.3 / 34.2 / 42.7, e o numeral de herói em `clamp()` até 83 px. Tamanhos de display em `clamp()` no `@theme`. Medida de linha das explicações limitada a 60ch (corpo), nunca acima de 75.
- **Self-hosting no app:** `@fontsource-variable/mona-sans/wdth.css` ou `@fontsource-variable/archivo/wdth.css`, importados no CSS de entrada.

## 4. Composição

### Arquitetura de informação e navegação (comum às duas direções)

Sidebar à esquerda com sete destinos, na ordem em que a pessoa pensa no mouse (de "como ele está" para "o que ele faz"):

1. **Overview** — o mouse, o link, a bateria e o estágio ativo. Abre aqui.
2. **DPI** — os 8 estágios, valor, cor e estágio ativo.
3. **Tracking** — polling, lift-off, ripple control, angle snap, motion sync.
4. **Buttons** — os 5 botões e o tempo de resposta (debounce).
5. **Macros** — biblioteca, gravação e edição.
6. **Lighting** — modo, cor, brilho e velocidade do LED.
7. **Power** — bateria, sleep e deep sleep.

Por que é intuitivo:
- É o padrão de app de preferências do macOS (sidebar + conteúdo), então não há sistema novo a aprender; atalhos `Cmd/Ctrl + 1..7` e `Cmd/Ctrl + Enter` para aplicar.
- Cada destino corresponde a **um controle físico do mouse** (sensor, botões, LED, bateria), não a um pacote do protocolo. O agrupamento técnico (pacotes `0x04`, `0x05`, `0x06`, `0x08`, `0x09`) aparece só no dock, traduzido.
- **Perfis** ficam no topo da sidebar como um seletor (como contas em apps de e-mail): é contexto de tudo abaixo, não um destino. O popover concentra add, rename, delete, import, export e reset, com confirmação nos destrutivos e a regra "Keep at least one profile".
- **O dock de aplicação** fica fixo no rodapé do conteúdo em todas as telas e é a única ação primária: ele lista o que mudou em linguagem humana ("DPI and tracking", "Buttons"), mostra a escrita grupo a grupo com ack e retry, e explica bloqueios ("The mouse is asleep. Move it or click to wake it, then apply."). Como o mouse não devolve a configuração (PROTOCOL 4.1), o app precisa dizer o que está no mouse e o que está só no perfil.

Estados do dock: em sincronia, N mudanças pendentes, gravando (grupo X de N, tentativa 2 de 3), aplicado, falhou (com Retry), bloqueado por sono, sem mouse.

Estados globais desenhados: link (cabo, 2.4G, Bluetooth, nenhum), bateria (desconhecida até o primeiro heartbeat, 10..100 %, "Charging" no cabo, sem número inventado), acordado ou dormindo, mudança de estágio feita pelo mouse (toast + animação de assinatura), polling bloqueado no Bluetooth (o comando `0x06` não vai por BLE).

### Wireframes — Direção A (Shell & Stage Light)

Elemento fixo atravessando telas: **o mouse é um objeto único que viaja** entre slots de cada tela (grande no Overview, de lado no DPI, central no Buttons, pequeno no Lighting). A luz do LED vai junto.

Overview (1180x760):
```
+----------------+---------------------------------------------------------------+
| o o o          |                                                               |
| [Default    v] |        .-~~~~-.  (poça de luz na cor do estágio)               |
|                |       /        \               ATTACK SHARK X3  (display leve) |
| > Overview     |      |  MOUSE   |              Connected over 2.4G receiver     |
|   DPI          |      |  hero    |              ---------------------------      |
|   Tracking     |      |   (o)LED |              (i) Link     2.4G receiver      |
|   Buttons      |      |          |              (i) Battery  80 %               |
|   Macros       |       \        /               (i) Sensor   1600 DPI, stage 2  |
|   Lighting     |        `-____-'                [1][2][3][4][5][6] estágios     |
|   Power        |                                                               |
| [X6 Mouse2 80%]| [ dock: Everything on the mouse matches "Default"   (Apply) ] |
+----------------+---------------------------------------------------------------+
  sidebar 232, translúcida | grid assimétrico 5fr/6fr, texto alinhado à esquerda
```

DPI:
```
+----------------+---------------------------------------------------------------+
| sidebar        | DPI stages                                   [+ Add stage]    |
|                | +---------+  +---- escada de luz -----+  +-- estágio 2 ---+  |
|                | | mouse   |  | (o) 1   800        [on]|  | 1600  DPI      |  |
|                | | (o)=====|==| (O) 2  1600  active   |  | [-][slider][+] |  |
|                | |  LED    |  | (o) 3  2400        [on]|  | cor: o o o o o |  |
|                | |         |  | (o) 4  3200        [on]|  | [Make active]  |  |
|                | +---------+  | ...  7  off            |  | [Turn off]     |  |
|                |              +------------------------+  +----------------+  |
|                | [ dock ]                                                      |
+----------------+---------------------------------------------------------------+
  o feixe de luz sai do LED e pousa na linha ativa; a 960 px o mouse sai e a escada vira lista
```

Tracking:
```
| sidebar | Tracking                                                             |
|         | Polling rate   [125 Power Saving][250 Office][500 Gaming][1000 E-sports]|
|         |                (Bluetooth: "Bluetooth sets its own rate...")          |
|         | Lift-off distance  ( 1 mm )( 2 mm )   desenho do sensor sobre a mesa   |
|         | Ripple control   [toggle]  uma frase do que faz                        |
|         | Angle snap       [toggle]  uma frase                                   |
|         | Motion sync      [toggle]  uma frase                                   |
|         | [ dock ]                                                              |
```

Buttons:
```
| sidebar |  Front side button  4 ---.            .--- 1  Left button           |
|         |                          |  MOUSE     |                              |
|         |  Rear side button   5 ---'  central   '--- 2  Right button          |
|         |                              3 Wheel click                           |
|         |  +-- painel do botão selecionado ------------------------------+    |
|         |  | [Mouse][DPI][Keyboard][Multimedia][Browser][Macro][Other]   |    |
|         |  | lista de ações / captura de atalho / escolha de macro       |    |
|         |  +-------------------------------------------------------------+    |
|         |  Key response time  [slider 4..50 ms]                                |
|         | [ dock ]                                                             |
```

Macros:
```
| sidebar | +-- biblioteca --+ +-- editor ------------------------------------+ |
|         | | Quick swap   * | | Quick swap        [Record]  14 of 46 slots   | |
|         | | Build ramp     | | linha do tempo  ||  |  ||   |   ||           | |
|         | | Copy and paste | | 1  v 2 pressed   0 ms                        | |
|         | | [+ New macro]  | | 2  ^ 2 released  42 ms  [ms]                 | |
|         | +----------------+ | [Add key][Add delay][Up][Down][Delete][Clear]| |
|         |                    | Playback ( 1 time )( Until any key )( Held ) | |
|         |                    +----------------------------------------------+ |
```

Lighting e Power:
```
| sidebar |  mouse pequeno com o LED no modo     | Mode   [lista de 7 modos]    |
|         |                                      | Color  o o o o o o [custom]  |
|         |                                      | Brightness [1..8] (só Static)|
|         |                                      | Speed  [Slow..Fast]          |

| sidebar |  Battery  80 %  (degraus de 10 %)  | Sleep after       [ 1 min ]   |
|         |  ou "Waiting for the mouse..."     | Deep sleep after  [10 min ]   |
```

### Wireframes — Direção B (Inside: Solder Mask & Sensor Vision)

O chão da janela inteira é um **campo de visão do sensor** em canvas atrás de painéis de vidro verde. Herói sangra pela borda inferior.

Overview:
```
+---------------+----------------------------------------------------------------+
| o o o         |  ::::::::::: campo do sensor (células de luminância) :::::::::: |
| Profile       |  1600                         ::::::::::.------.:::::::::::::: |
| [Default  v]  |  DPI on stage 2 of 6          :::::::::/ MOUSE  \::::::::::::: |
|               |                               ::::::::|  preto   |:::::::::::: |
| Overview  ==  |  ATTACK SHARK X3              ::::::::|  (o)     |:::::::::::: |
| DPI           |  Connected through the 2.4G   ::::::::|          |:::::::::::: |
| Tracking      |                               ::::::::|  sangra  |:::::::::::: |
| ...           | [Link 2.4G][Battery 80%][Polling 1000 Hz]  ----- borda -------- |
| [dock]        |                                                                |
+---------------+----------------------------------------------------------------+
  grid 7fr/5fr: números à esquerda, objeto à direita cortado pela borda
```

DPI:
```
| rail | DPI stages                                                         |
|      | [frame][frame][frame][frame][frame][frame][ off ][ off ]           |
|      |   800   1600*  2400   3200   5000  26000   +      +               |
|      |   cor    cor   cor    cor    cor    cor                           |
|      | +-- Stage 2 -------------------------------------------------+    |
|      | |  1600 DPI    [-]  [------slider------]  [+]   cores [ ]      |    |
|      | +------------------------------------------------------------+    |
```
Cada "frame" é um quadradinho do campo do sensor renderizado na densidade daquele estágio.

Buttons: mouse preto à esquerda com hotspots; à direita, **busca de ação** (um campo "Find an action" com a tabela inteira filtrável) em vez de abas — o contraste de interação com A.

Macros: **vista de analisador lógico**: uma linha por tecla, pulsos de pressionar/soltar no tempo, intervalos clicáveis para editar o delay; inspector à direita.

- **Quebra estrutural escolhida (A):** um objeto fixo (o mouse) que atravessa todas as telas e muda de papel — herói, fonte de luz da escada de DPI, mapa de botões, prévia do LED. Pertence ao assunto porque o app inteiro é sobre um objeto físico.
- **Quebra estrutural escolhida (B):** o campo do sensor em tela cheia por trás de tudo, e o herói que sangra pela borda inferior. Pertence ao assunto porque é literalmente o que o sensor vê.
- **Alinhamento:** à esquerda em ambos. Centralizado só no herói de A.
- **O que fica fora do grid:** o mouse (A) e o campo do sensor (B).

## 5. Assinatura

- **A — Stage Light.** O LED sob o scroll emite a cor do estágio ativo: a luz atravessa o plástico da carcaça, forma uma poça no chão da janela e, na tela de DPI, vira um feixe que pousa na linha do estágio ativo. Quando o próprio mouse troca de estágio (evento `0x1000`), o LED pisca, a luz troca de cor a partir do ponto do LED e o feixe desliza até a nova linha.
- **B — Sensor Vision.** O chão da janela é a imagem que o sensor fotografa da mesa: uma matriz de células de luminância. A densidade das células segue o DPI do estágio ativo; quando o mouse troca de estágio sozinho, uma varredura re-resolve o quadro. Acordado: o quadro desliza devagar, como se rastreasse. Dormindo: congela e escurece. Sem mouse: ruído.
- **Manifestações:** A — poça de luz no Overview, feixe na escada de DPI, prévia do modo no Lighting, mini-LED no chip do dispositivo da sidebar. B — campo de fundo, miniatura por estágio no DPI, estado de carregamento (varredura), sono (congelado).
- **Curva de easing de assinatura:** `--ease-actuation: cubic-bezier(0.2, 0.8, 0.12, 1)` — o curso de um switch: pré-curso rápido, ponto tátil e um fim firme sem overshoot. Movimento que responde a arraste usa a mola `click spring` (stiffness 520, damping 38) no `motion-v`.
- **Por que isto não existe em outro produto:** apps de periféricos usam RGB como decoração (ondas de arco-íris, fundos que pulsam); aqui a luz é **estado** — ela só tem a cor do estágio ativo e só se move quando o estágio muda. Em B, nenhum configurador mostra a superfície pelo olho do sensor, e aqui ela codifica DPI e estado do link.

## 6. Revisão contra os clichês

1. **Creme quente + serifa + terracota:** não. A é branco frio de ABS; nenhuma serifa; vermelho só para erro, e é o vermelho do sensor, não terracota.
2. **Quase preto + acento ácido/vermelho:** era o risco de B. Corrigido: o chão é verde de máscara de solda saturado (`#0D3B33`), não preto, e o acento é ouro ENIG, não neon. O skin do fabricante (preto + vermelho) é justamente esse clichê e foi abandonado.
3. **Jornal, fios finos, zero radius:** não. Raios variam por hierarquia (tecla 10 px, painel 22 px em A; pad 6 px, placa 16 px em B).
4. **Kit de cards de SaaS:** evitado. Overview não é grade de cards: é um objeto e três linhas de status. Sombras tingidas com a cor do grafite (A) ou da máscara (B), nunca `rgba(0,0,0,.1)`.
5. **Cromo de template:** sem eyebrow em caixa alta, sem "A · B · C", sem seta em botão, sem near-black, sem monospace decorativo (números usam `tabular-nums` da própria família).

**O que mudou depois desta revisão:** (a) B começou como "abismo azul-escuro com ciano" (marca Attack Shark = tubarão) — caía no clichê 2 e não vinha do mouse; virou a placa por dentro. (b) A tinha um botão de ação redondo e brilhante no centro inferior, igual à referência; virou o dock de aplicação, que é funcional (lista os grupos sujos) e não copia o CleanMyMac. (c) A lista de ações em B era em abas como A; virou busca, para que as duas direções discordem também na interação.

## 7. Riscos

- **A aposta mais ousada:** A — um app de mouse gamer **claro**, branco como a carcaça, quando todo o segmento é preto com RGB. B — um fundo vivo (canvas) atrás de toda a interface.
- **O que a torna defensável:** A — o X3 vendido em branco é o objeto; a luz do estágio só brilha de verdade sobre branco, e o app parece um utilitário nativo do macOS, não um "launcher gamer". B — o campo é baixo contraste (opacidade 0.2 a 0.35, só perto do sensor), congela com `prefers-reduced-motion`, pausa com a janela oculta, e carrega informação (DPI e estado).
- **O que eu corto primeiro se não funcionar:** A — a poça de luz no chão (mantém só a luz dentro da carcaça e o feixe da escada). B — o deslizamento contínuo do campo (mantém o quadro estático e só a varredura na troca de estágio).

## 8. Iteração depois dos screenshots

- **A:** zero dos numerais cortado (`tabular-nums` da Mona Sans) trocado por `lining-nums`; glow do SVG recortado em retângulo corrigido com `overflow: visible`; chips de estágio e segmentado sem quebra a 960; Power ganhou a linha do tempo de descanso (Awake, Asleep, Deep sleep), que antes era uma tela esparsa; desenho do lift-off refeito em perfil lateral.
- **B:** anéis dos pads do dock recortados pelo `overflow` viraram `border`; valor de DPI com `field-sizing: content` (o "DPI" ficava solto); botão primário desabilitado deixou de ser ouro lavado e virou poço neutro; thumb dos toggles ancorado à esquerda; pulso das macros ganhou linha de base para ler como analisador lógico; a 960, o editor de DPI fica em duas colunas, os chips do Overview viram grade de 4 colunas sem invadir o mouse, e o inspetor de evento das Macros desce para a própria linha.
- **Auditoria:** `design-audit.mjs` 0 erro e 0 aviso nos dois HTMLs e no `tokens.css` (o "truncagem" era falso positivo num comentário "Restore"). `contraste.mjs`: todo par usado para texto está acima de 4.5:1; `line` só aparece como borda.
