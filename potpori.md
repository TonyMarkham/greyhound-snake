# Potpourri — target-system design and session decisions

The cart-pole project's matured design details, settled in conversation
(2026-10-09/10) after M1 landed. Companion to:

- `unity-mujoco.md` — MuJoCo/MJCF export and joint authoring: facts,
  plan, and the landed M1–M3 decisions
- `mujoco-schema.md` — MJCF facts checked against the MuJoCo stable
  docs (the schema layer this file's sim-model references)
- `occt-to-unity.md` — importer pipeline, gap table G1–G14

Everything below is **design/decision, not code** except where noted.
Nothing here contradicts the bite plan; it fills the layer around it —
the physical system, the vision system, the training loop, and the
deployment architecture that M2–M4 and later bites serve.

## The target system — the physical robot

- **A cart-pole**: cart on **two parallel round rails** spanning **1 m**
  (linear bearings — the rails remove rotation about the axis, hence a
  single slide joint models the constraint) driven by a stepper motor
  through a timing belt; **500 mm pole** pivoted on the cart; two limit
  switches.
  The user designed, 3D-printed, and built it — CAD in **FreeCAD 1.1.1**
  (`cart-pole-asm`: cart + cart-asy, the pole sub-assembly `Assembly`
  with pole-target-holder/clamp, Pillow Block ×2, 5972K91_Steel Ball
  Bearing ×12, bearing-cap ×2, belt-retainer ×2, GT2 48T 8 Bore v3
  pulley, pully-gt2-idler-6mm-5b, Nema23_Stepper_Motor and
  5958N103_Wet-Environment Stepper Motor ×20, base-idler, base-motor,
  bars bar-rd-d16x1000/1001, bar-rd-d5x51, bar-rd-d8x240, bar-rd-d8x500,
  rod-clamp-16mm ×4, rod-clamp-5mm ×2, rod-clamp-8mm ×4, Screw ×2,
  spacer-8mm ×2 …).
- FreeCAD bbox ≈ **1086 × 1056 mm** — consistent with the 1 m rail span and
  the 500 mm pole hanging below. Verified working: Unity **6.7.0b2**
  imports `cart-pole-asm.stp` through the StepImporter, hierarchy and
  materials intact (screenshot 2026-10-10).
- The FreeCAD **joint tree** (`GroundedJoint`, `FixedJoint`s,
  `DistanceJoint`s) is design-intent metadata that STEP does not carry
  through XCAF — it is the **transcription source for the M2
  `StepJointSet`**: type, axis, origin copied from ground truth, no
  inference.
- The **marker mounts are CAD parts** (`pole-target-holder`,
  `pole-target-clamp`), so marker poses come from the assembly's node
  transforms, verified physically — the marker asset references CAD-exact
  placements.
- Pole 500 mm ⇒ small-signal pendulum period ≈ **1.2–1.4 s** ⇒ the whole
  deployed loop (camera → detector → policy → firmware) runs at
  tens-of-Hz comfortably; ~100 ms detector latency fine, ~500 ms is not.

## Markers, camera, and the vision geometry

- **4 markers, all unique**: two rail-end markers (static, collinear with
  the rail), pole-pivot marker, pole-end marker. 50 × 50 mm, built from
  unique **15 mm square/triangle** patterns, unambiguous regardless of
  rotation — the pole tags rotate continuously with θ, so
  rotation-invariant ID is load-bearing.
- **Pole-pivot marker is centered on the pivot axis** (CAD-exact): a tag
  centered on its rotation center has a rotation-invariant *centroid* —
  clean cart-position anchor; an offset tag would trace a circle with θ.
  Both pole tags rotate together, so the **pivot→tip pixel vector is a
  rigid in-image feature: θ = that vector's angle** at the v1 camera.
- **The marker sets are not coplanar** (the pole plane is offset from the
  rail line — otherwise the pole collides with the rail). Consequence:
  treat the 4 markers as **independent known 3D points** (rigid body +
  local offset each), never a shared-plane homography. One visible marker
  + known tag size ⇒ position + scale; ≥2 ⇒ full camera pose (PnP).
  Camera pose becomes a free variant at zero labeling cost.
- **Camera v1**: single fixed pose — eye-level with the rail, **500 mm
  horizontally centered**, face-on to the pole plane. Rail line horizontal
  in every frame. Per-marker calibration constants (end tags sit at
  ~707 mm working distance — √(0.5² + 0.5²) — pole tags nearer 500 mm).
  Keep a **small pose-jitter band** around the nominal (mount tolerance) —
  exact-pose training fails on the bench by millimeters.
- **Resolution floor**: over a ~1.4 m FOV, 320 px ⇒ ~3.5 px per 15 mm
  feature (ambiguous — and motion blur finishes it); **640 px ⇒ ~7 px per
  feature** (reliable). Low-res-for-speed floors at ~640 wide, or a
  narrower FOV crop. Verify by rendering the 4 tags at nominal distance
  with randomizer blur/angle and measuring ID accuracy. Precision at
  640 px ≈ 2 mm/px over the rail — ample.
- **Occlusion is the normal case**: the pole can hide either end marker;
  never rely on seeing both. Fusion hierarchy: both visible → full
  calibration; one → position + scale from that tag; **none → dead-reckon
  on the step-counter belief** until a tag reappears. Rendered training
  episodes get occlusion for free (the pole physically blocks the tag);
  the fast no-render loop needs an explicit visibility model from the
  offset geometry.
- **Low-res camera is deliberate**: detection difficulty scales with
  marker pixels, not image pixels; chunky tags stay detectable at low res,
  Pi-side inference gets fast, and latency (not resolution) is the budget
  that matters. Train **at the deployment resolution** — never hi-res
  downsampled.

## Controller architecture

- **Command contract**: the RPi sends the controller **one signed float** —
  sign is direction, magnitude is the value. The stepper's device domain
  is integer microsteps, so the firmware holds a **fractional-residual
  accumulator** (sum floats, emit whole microsteps, carry remainder —
  otherwise a steady 0.4-steps-per-tick stream truncates to zero). The
  **envelope clamp** sits next to the accumulator: firmware-side mirror of
  the MJCF `ctrlrange` — a buggy policy cannot drive the real cart past a
  switch, same rule as the sim. Document "1 unit = N microsteps" in the
  contract notes.
- **The policy is the controller — no PID anywhere in the loop.** The
  Arduino *cannot* close the pole loop (it sees neither pole nor camera);
  the plant is a stepper (command steps, it moves). A classical controller
  only earns places outside the loop: baseline comparison, and optionally
  a catch-the-pole mode at deployment ("zero safety in training" per the
  user; one clamp at the bench if ever).
- **Homing is a driver procedure**, not a control input: drive toward an
  end at calibration speed, register the switch, zero the step counter
  with the known offset. The switches exist to serve homing; homing serves
  the controller's coordinate system. Switches are **never** hit outside
  calibration — `ctrlrange` (sim) and the firmware clamp (hardware) make
  that structural; the margin between `range` and `ctrlrange` is the
  safety gap, visible in the XML.
- **Step counter = controller belief** = integration of issued commands,
  kept driver-side exactly like the real Arduino. Never an MJCF sensor
  (sensors read state, not command history). **Belief vs `data.qpos`
  divergence under `forcerange` saturation = simulated step loss**, for
  free.
- **Limit switches stay out of the simulation XML** (decided 2026-10-09;
  supersedes the earlier "virtual end-switch" idea — `jointlim` does not
  even exist in MuJoCo 3.15.0). The switches are hardware serving homing;
  the sim has no driver. Sim-side safety is structural: mechanical joint
  `range` ⊃ actuator `ctrlrange`, mirrored by the firmware envelope clamp
  (`mujoco-schema.md`, *Limit switches*). MJCF target: **2 joints (slide +
  hinge), zero sensors, permanently**; cameras excluded (Unity renders);
  no freejoint (rail fixed, 2 DOFs total).

## Embedded controller (Uno R4 WiFi) — embedded Rust candidate

- Board: **RA4M1** (Cortex-M4 main MCU: real-time stepping, limit-switch
  inputs, homing) + **ESP32-S3-MINI co-processor** (WiFi) over UART. The
  S3 is user-reflashable.
- Preferred architecture: firmware **logic against `embedded-hal` 1.0
  traits** + own transport and clock traits; two build targets — the real
  board (board-support crate) and an **x86 mock-HAL SIL binary** (in-memory
  pins, virtual injected clock, PTY/socket transport). Same logic, two
  targets; no mock-vs-real drift possible by construction. HIL-vs-SIL is a
  cargo target.
- **SIL shape**: built C/Rust binary spawned by the orchestrator; inputs =
  clock ticks + limit-switch booleans + commands; outputs = step events +
  status. **Virtual injected clock** (deterministic, faster than real
  time); wall-clock mode only for HIL-parity bench tests. Stall stays
  invisible to the firmware (open loop, like reality).
- **WiFi options** (no driver to write — Espressif's blob + `esp-wifi`
  covers S3): (1) keep the stock S3 bridge, implement its UART protocol;
  (2) reflash the S3 with `esp-hal`/`esp-wifi` and own a ~20-line UART
  protocol (cleanest all-Rust); (3) drop WiFi — USB serial to the RPi.
  Web endpoint = **naive HTTP endpoints** (the user's proven pattern);
  curl/browser can drive the SIL too (transport is a trait). WS only if
  status streaming is needed. If the RPi is always attached, option 3 is
  the honest minimum.
- **POC checklist** (user builds the toy, flashes the Uno R4): flash path
  (`cargo-binutils` → objcopy/bin → DFU; R4 uploads over USB), the four
  peripherals (GPIO out at step rate, GPIO in for switches, timer tick at
  ramp rate, UART/USB-CDC), one ISR latency sanity test. Structure the toy
  as `firmware-core` / `firmware-ra4m1` / `firmware-host` from day one —
  the POC is the architecture's first increment, not a throwaway.
- **Open verification item**: RA4M1 Rust ecosystem maturity (usable HAL vs
  PAC-only — unverified). Escape hatches: write the small BSP on the PAC,
  or swap the board (RP2040/ESP32-S3) touching zero logic lines.

## RPi controller

- Rust binary, two targets from one crate: `x86_64` (sim) +
  `aarch64` (Pi); only peripheral access is cfg-gated (`serialport` opens
  `/dev/pts/N` on sim, `/dev/ttyUSB0`/`serial0` on deploy — PTY baud calls
  are harmless no-ops).
- **Two modes from one binary**: `deploy` (standalone: camera → detector →
  policy → serial, config-file driven, no orchestrator dependency) and
  `sim` (orchestrator-supervised, PTY + policy). Deployment never depends
  on training infrastructure.
- **Inference split**: the cart-pole policy is a tiny MLP — Candle or burn,
  pure Rust, same binary. **The detector is the heavy leg** (object
  detection on a Pi) — design it as a swappable backend (in-process
  ONNX/Candle, Hailo AI-kit, or ship frames to a bigger box); a one-trait
  change.

## Sim control model

- **Stepper/belt = the high-friction-wheel fake**: `<position>` actuator on
  the slide joint, belt folded into `forcerange`/`ctrlrange`. 48-tooth
  GT2 belt (CAD: `GT2 48T 8 Bore v3` pulley) ⇒ pitch radius ≈ **15.28 mm**,
  96 mm/rev; NEMA-23 at 1.8° ⇒ 0.48 mm/step, **0.03 mm/step at ×16
  microstepping**; `F_max = τ_stall / 0.01528`. `kp` = tracking grip,
  joint damping = rail friction, ctrl targets advance in microstep
  increments. Rotor inertia negligible (NEMA-23 vs kg-scale cart) — the
  full-fidelity model (rotor hinge + soft joint equality ⇒ believable
  stall) only if calibration can't reproduce observed behavior.
- **Direction-reversal calibration is not trial and error**: the fake has
  four free parameters (`kp`, damping, `forcerange`, ramp); cart mass is
  exact from the BRep; the vision system itself measures real reversals
  (rail-end scale, pivot position) to fit the parameters. Datasheet
  step-rate limits bound the ramp.
- **Reward** (zero safety shaping — only poor scores for failure, good for
  success): `−w1·θ²` (upright) + `−w2·(x − x_center)²` (centered) +
  `−w3·|Δctrl|` (effort = the stepper ramp, physics cost not safety) +
  survival bonus. **Failure rules**: pole drop past threshold → terminate
  with a big negative; stop contact → choose terminate vs
  penalize-and-continue deliberately (recover-from-stop is real signal on
  this machine). **Gross-error rule: any measurement > 1 m from a visible
  rail-end marker = FAILURE** — on a 1 m rail that is physically
  impossible, so it is a detector-hallucination detector, evaluated on the
  observed (noisy) measurement for deployment parity; rejected
  measurements become labeled-invalid samples for the detector's own data.
- **Training observations = the deployed mix**: θ always from the detector
  (the pole has no encoder), x from belief + vision corrections, truth
  (`qpos`) behind the reward line only. Fast no-render control training
  uses the **calibrated detector noise model**; the real detector runs in
  the loop for validation episodes and its own training data. Never train
  on `qpos`.
- **Curriculum**: near-upright stabilization first; swing-up from hang
  optional later (hard under limited travel). Objective is regulation:
  upright + centered.
- **M2 consequence (landed)**: actuation is its own asset — `Add >
  Actuator` on a jointed part creates a `StepActuator` asset (in the
  set folder) targeting the joint by reference, carrying the
  `MjActuator` mirror of the MJCF `<position>` element verbatim:
  `type`, `ctrllimited` + `ctrlLo/Hi`, `forcelimited` + `forceLo/Hi`
  (the spec's own limit switches; explicit `false` disables
  clamping). Belt ratio/pitch radius and the calibration split
  (homing end, approach speed, zero offset) are deliberately **not**
  schema fields — transmission constants and calibration enter when
  their consumer exists, on the deployment side; `ctrlrange` remains
  the sim-side mirror of the firmware envelope clamp.

## Training loop (three streams, never mixed)

1. **Commands** (controller → sim): always sent; the sim is driven exactly
   like the Arduino. The belief *is* the integrated command stream (the
   actuator's `ctrl` target). Never inject belief as state.
2. **Observations** (detector → policy): the deployed mix above.
3. **Truth** (`qpos`): reward and logging only.

MuJoCo exports body positions → Unity runtime applies them to the imported
GameObjects → renders → detector → policy → commands. MJCF needs no
cameras, and eventually no meshes at all (below).

## Unity runtime and the training farm

- **One code path, a mode flag**: visible = same pipeline + window +
  debug overlays (GT boxes/keypoints, predictions vs truth, sim-state HUD);
  headless = render-to-`targetTexture` + frame writes. The debug view
  cannot diverge from what training sees.
- **Headless ≠ `-nographics`**: you still need the graphics device to
  render. Working combos: `-batchmode` alone with a GPU, or `xvfb-run`.
  **Not** the Linux *server build* target (strips graphics). Verify
  against the pinned Unity version — the user runs **6.7.0b2**.
- **Farm**: eventually ~16 parallel instances as **compiled standalone
  players** (not Editor instances — footprint, startup, licensing). One
  build, N processes, per-instance args (`--instance-id --port --seed`).
  Fixed sim cadence per instance (headless has no vsync — pin pacing for
  GT pairing). The new Unity CLI may be the build tool — verify its
  command surface against the pinned version.
- **Orchestrator (Rust/Axum)**: control plane = HTTP start/stop/status +
  WS state streams + `tokio::process` supervision (restart/backoff,
  heartbeats) for three worker kinds — MuJoCo sims, Unity runtimes,
  firmware-SIL. **Data plane separate**: sim state is tiny (WS fine);
  rendered frames do NOT ride WS — dataset writes / local UDS. Introduce-
  then-direct: the orchestrator pairs a sim env to a runtime and hands over
  endpoints; peers connect directly; the orchestrator keeps only a status
  channel. **Plain WebSocket + small JSON/MessagePack** — not SignalR
  (ASP.NET Core cannot host inside a Unity player; HttpListener WS is the
  Unity-side route). **Provenance per episode**: player build hash, MJCF
  hash, sim version, seed, density/scale config — cheap now, priceless
  when a trained model misbehaves.
- MJX symmetry: the physics side can batch the same way the render side
  wants to, later.

## Export transport (landed) and the bbox-geom option

- **Landed (M3, 2026-10-10)**: the exporter is C# in the Unity
  package — the Export button on the joint-set inspector writes the
  MJCF model directly from the `Mj*` payload mirrors and the imported
  prefab, with binary STL meshes, into `<step file>.stp~` beside the
  STEP file. The two transport options left open at M2 (a C ABI into
  the host, probe-side JSON feeding the Rust CLI) are rejected: both
  would bolt an interchange layer onto a data model that already
  carries MJCF's attribute names, and the CLI path would ship a
  second native binary. The Rust `greyhound-export-mjc` stays the
  rigid-tree reference implementation (`unity-mujoco.md`, *Bite M3*).
- **Open later option — bbox geoms for `simulate`**: MJCF meshes
  could become simple bounding-box geoms (`<geom type="box" size
  pos>` from mesh min/max) — a lightweight physics-side variant;
  Unity renders the real STEP geometry, so the simulation itself
  needs no STLs. Never decided; revisit when the training loop wants
  a leaner physics model. Inertials stay exact-BRep either way —
  they never come from geoms.

## Detector training (synthetic labeling loop)

- **The pain point deleted**: manual labeling. Marker poses are known CAD
  geometry; projecting them per frame gives pixel-exact boxes for free.
  The same Unity render loop is **three datasets from one run**: images +
  boxes (detector), state streams (controller), observation pairs (noise
  model).
- **Marker poses are authored data measured off the physical robot** (via
  the CAD marker parts) — placement error is label error; it is the only
  step with no synthetic shortcut. One render loop → randomize → render →
  project GT → labels. Unity's **Perception package** is the existing
  wheel (randomizers + bounding-box labelers + dataset export).
- **Randomization design**: lighting/exposure, motion blur, viewing angle,
  backgrounds (never train against only a void), and **deliberately
  oversample the hard band** — heavy occlusion, extreme angles, both
  markers hidden, blur during fast reversals. Camera pose as a variant
  costs zero labeling (PnP-style solve from known 3D points) — v1 keeps a
  small jitter band around the nominal; v2 opens the pose variant, plus
  optionally rendered-loop episodes with the camera randomizer so the
  policy trains under realistic observation degradation (curriculum:
  fixed → varied). Bounds = the deployment envelope, not imagination.
- **Close the sim-real gap with a small real-image fine-tune set** — the
  robot exists; a few hundred real frames (lighting, pole positions,
  occlusions) does most of the remaining transfer work.
- Delete-option on record: if the markers were ArUco/AprilTag-shaped,
  OpenCV detects them with **no training at all**. The user chose ML
  detection (robustness to blur/angle/partial occlusion) — noted here so
  the trade-off stays conscious.

## Product thesis and IP posture (for the pitch and the content)

- The knee-cap in Unity's previous robotics attempt: the **inject /
  annotate / define loop**, redone on every reimport. Services-led ingest
  ("we'll handle the model prep") made the pain invisible and never
  scaled — cost booked per engagement and, worse, iteration gated on a
  vendor queue. **NVIDIA buys Pixyz licenses specifically for its CAD
  ingest hole** (Pixyz Plugin for Omniverse — public record): the hole is
  budgeted at the industry's biggest player, and the paid filler only
  covers inject — the loop stays broken.
- This repo is the full loop with **domain-partitioned truth**: geometry =
  STEP (regenerates), intent = Unity assets keyed on stable part names
  (survive reimports), everything else derived. Manual work only what must
  be human — paid exactly once. Prep layer is backend-agnostic (MJCF now,
  URDF listed as the deferred second writer — the same ingest could feed
  Isaac's own importer). Unity Editor is the authoring instrument for
  spatial intent; Isaac asks for USD, Unity asks you to point at it.
- Incentive insight: features tied to per-seat + services revenue get
  poo-poo'd regardless of user value (the user fought for these features
  inside Pixyz and lost to business-model gravity). **Open source is where
  poo-poo'd features have no jurisdiction** — give away the ingest,
  monetize the loop's surface area (seats, compute, services).
- **IP posture**: the entire material was made as an unemployed
  mechanical engineer — no Unity IP. Git timestamps prove the timeline;
  the docs cite only public sources (OCCT headers, MuJoCo's published
  reference, Unity's public docs), so the content is demonstrably
  unencumbered. License choice is strategic: MIT/Apache-2.0 maximizes
  adoption (the pitch is "replaces per-seat licensing"); OCCT's
  LGPL-2.1-with-exception obligations are already handled in the
  distribution plan. If employment follows, the repo goes on the prior-IP
  schedule — standard practice.
- Background on record: 25 years mechanical engineering before managing
  the Pixyz eng teams at Unity; the robot was designed, 3D-printed, and
  built by the user; the vision system was **derived around the robot
  concept to force a unity mapping** (pixel angle = θ, tag centroid =
  position — no solve between pixels and state).

## Content plan (YouTube / LinkedIn hedge)

- Format decision: **film only after everything works** — "easier to make
  lectures when the lessons are already learned." The repo docs are the
  hindsight-immune record: they were written *before* each implementation
  and can be quoted against outcomes.
- **Result-first structure**: episode 1 = the pole standing under the
  trained policy; then "here's the part of that you just saw," part by
  part. Demo timestamps are the map (import → episode N, labels →
  episode M). A recurring 60–90 s low-detail "loop map" intro lights up
  the node each episode closes. Capture chronologically, publish
  deductively — and the user's call overrides in-flight filming.
- Failure arcs are the best content (SIGSEGV root-cause with 3/40 stats);
  frame criticism at workflows (the loop, services-ingest), never at
  people or products; position as "the loop is closable," not anti-Isaac.

## Session artifact status (2026-10-10)

- **M1 is landed in main** and green: `crates/greyhound-export-mjc`
  (31/31 tests, clippy clean), one-type-per-file fixed via
  `src/export/{mod,options,summary}.rs`, use-layout fixed, and the
  end-to-end tests serialized with `OCCT_LOAD_SERIAL` after a real race
  (SIGSEGV: 3/40 parallel runs, 0/15 serial; two concurrent OCCT
  load/unload cycles).
- Release binary verified on cart-asy: 43 bodies / 20 meshes / 39 geoms /
  0.255450 kg.
- **M2 is landed and Unity-validated** (M2a–d): the joint/actuator
  annotation asset family — creation menus, UIToolkit editors with
  the set overview, axis-pick scene tools with the locked-inspector
  claim, reimport survival (AssetPostprocessor validation + container
  prune) (`unity-mujoco.md`, *Bites M2a–M2d*).
- **M3 is landed and Unity-validated**: the C# MJCF exporter — the
  Export button writes `<step>.stp~` (XML + STLs) beside the STEP
  file; spec-verbatim limit switches (`limited`, `ctrllimited`,
  `forcelimited`); forms grey their ranges behind the switches;
  MuJoCo's own moving-body mass rule transcribed. Validated on
  `cart-pole-asm` (81 bodies, 29 meshes; cart COM/mass match the M1
  reference numbers). Pending: the MuJoCo compile/viewer smoke test.
- `../greyhound-snake-m1` worktree is stale (pre-fixes) and dismissed;
  removal on request. `next-step.md` is an old handoff predating G5/G13 —
  historical.
- MuJoCo not installed locally; **local from-source install doc is a
  future session** (pattern: `occt-linux.md` — prerequisites → configure →
  build → install → validation, checked against MuJoCo's build docs for a
  pinned tag). The generated `__WIP__/cart/cart-asy.xml` validates both
  the XML and the install (nbody 44, nmesh 20, ngeom 39, Σbody_mass
  0.2554502 kg).
- `meshdir="meshes"` resolves relative to the model file's directory —
  the export layout is relocatable; no absolute paths.

## Open verification items

1. RA4M1 Rust ecosystem maturity (HAL vs PAC-only) — POC or repo check.
2. S3 reflash tooling on the Uno R4 WiFi; `esp-hal`/`esp-wifi` version pin
   against primary sources.
3. 48-tooth belt: **GT2 48T confirmed in the CAD** (`GT2 48T 8 Bore v3`
   pulley) — 2 mm pitch stands; pitch radius 15.28 mm and the mm/step
   numbers stand with it. The "JT2" note is moot.
4. Unity 6.7.0b2 headless/batchmode behavior (batchmode without
   -nographics, RT rendering) on the pinned build.
5. New Unity CLI build commands for the pinned version.
6. MuJoCo from-source install doc session; validate with
   `__WIP__/cart/cart-asy.xml`.
7. Detector resolution check: render 4 tags at nominal distance with
   randomizer blur/angle at 640-wide; measure ID accuracy.

## References

- `unity-mujoco.md` — facts, plan, and landed M1–M3 decisions;
  `mujoco-schema.md` — MJCF schema facts; `occt-to-unity.md` —
  importer + gaps.
- MuJoCo XML reference (stable), modeling guide, Python docs — anchors
  in `mujoco-schema.md`.
- FreeCAD 1.1.1 `cart-pole-asm` (local design file, joint tree = M2
  transcription source); Unity 6.7.0b2 import screenshot (2026-10-10).
- Unity Perception package — randomizer/labeler wheel for the synthetic
  loop.