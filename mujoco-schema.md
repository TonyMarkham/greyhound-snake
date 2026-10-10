# MuJoCo XML schema — joints and actuation for the cart-pole

Facts doc (not a bite doc) covering the MJCF schema
surface the repo needs next: the cart's **horizontal slide joint**, the
pole's **horizontal hinge joint**, and the **stepper-motor drive** of the
cart (position servo on the slide; `potpori.md` *Sim control model*).
Checked 2026-10-09 against the MuJoCo **stable** docs — XML reference and
modeling guide — which track release **3.15.0** (verified via the GitHub
releases API). Sources: `XMLreference.rst` and `modeling.rst` from
readthedocs' `_sources` (full text, not summarized renders). Companions:
`unity-mujoco.md` (M1–M4 plan), `potpori.md`
(the physical system this schema work serves).

Conventions as settled in `unity-mujoco.md`: SI units (meters, kg) by
convention (gravity default `(0, 0, −9.81)` sets the length scale),
`angle="radian"` in our exports, Z-up right-handed world.

## Element map (what we will emit)

```
<mujoco>
  <compiler .../>          already emitted (M1)
  <option .../>            NOT emitted today (defaults); timestep/integrator decisions come with actuation
  <worldbody>
    <body name="cart" pos quat>
      <joint name="cart_slide" type="slide" .../>     ← cart on rail
      <geom type="mesh" .../> <inertial .../>          already emitted (M1)
    </body>
    <body name="pole" pos quat>
      <joint name="pole_hinge" type="hinge" .../>     ← pole pivot
      ...
    </body>
  </worldbody>
  <actuator>
    <position name="cart_drive" joint="cart_slide" .../>  ← stepper/belt fake
  </actuator>
</mujoco>
```

`<option>` is not required (gravity/timestep defaults below), but once
actuation exists the integrator choice becomes relevant (see Position
servo). Bodies without joints stay welded; joints create the only DOFs
(`xmlref #body-joint` intro). Joints cannot be defined in the world body.

## body — the frame the joints live in

| Attribute | Value | Notes |
|---|---|---|
| `pos` | `real(3)`, default `(0,0,0)` | Body frame position **relative to the parent** (`xmlref #body-pos`) — the pivot/anchor enters here: the pole body's `pos` is the pivot point in cart coordinates |
| `quat` / `axisangle` / `xyaxes` / `zaxis` / `euler` | orientation forms | `quat` is (w, x, y, z) w-first (M1 already relies on this) |
| `mocap` | `[false,true]` "false" | Only for world children without joints; not our case |

A body without a joint is rigidly welded to its parent; multiple joints in
one body apply their transformations **in order** (`xmlref #body-joint`
intro). We use one joint per body — the pole is a child body of the cart
with its own hinge, not a second joint on the cart body.

## body/joint — full attribute surface

All of `pos`/`axis` are in **the frame of the body where the joint is
defined** (the child body's frame) (`xmlref #body-joint-pos`,
`#body-joint-axis`).

| Attribute | Value | Notes |
|---|---|---|
| `type` | `[free, ball, slide, hinge]` "hinge" | We use `slide` and `hinge` |
| `pos` | `real(3)` "0 0 0" | Joint position in the child body's frame. For **slide** joints it affects *rendering only* — "for simulation purposes only the direction is needed" (`xmlref #body-joint-type`) |
| `axis` | `real(3)` "0 0 1" | Rotation axis (hinge) / translation direction (slide). Auto-normalized; a zero-length axis (‖·‖ ≤ 1e-14) is a **compile error**. Default is vertical — a horizontal joint must set it explicitly |
| `range` | `real(2)` "0 0" | Joint limits. **Radians or degrees for hinge** per `compiler angle`; **meters for slide**. Presence of `range` with `autolimits="true"` (default) enables limits — never emit `limited` (M1 rule, unchanged) |
| `limited` | `[false,true,auto]` "auto" | Do not emit (see `range`) |
| `damping` | `real` (up to 3 components) "0" | Linear damping `f(v) = −a·v` in joint units (N·s/m slide, N·m·s/rad hinge). Components 2–3 give quadratic/cubic polynomial damping. The Euler integrator handles joint damping **implicitly** (`xmlref #body-joint-damping`) — one reason plain `damping` is safe |
| `frictionloss` | `real` "0" | Dry (Coulomb) friction, constant force magnitude opposing motion; needs a constraint — set only if wanted |
| `armature` | `real` "0" | Reflected rotor inertia added to the joint (geared transmissions multiply by gear²; here the "gear" is belt pitch radius). Docs: positive armature "significantly improves simulation stability" even when small (`xmlref #body-joint-armature`) — relevant to the stepper-rotor question in `potpori.md` |
| `ref` | `real` "0" | Joint value corresponding to the initial configuration; runtime transform = `qpos − ref`. The calibration **zero offset** maps here (or to the authored pose), not to `ctrl` |
| `springdamper` | `real(2)` "0 0" | [timeconst, damping-ratio]; when both positive it *overrides* `stiffness`/`damping` with auto-computed values — do not set accidentally |
| `stiffness` | `real` "0" | Spring toward `springref`; off for our joints |
| `margin` | `real` "0" | Soft-limit activation distance (limits are constraints; `margin` + `solreflimit` shape them) |
| `actuatorfrcrange` | `real(2)` "0 0" | Clamp of **total** actuator force at the joint, after transmission (scalar joints only). Third clamping layer — see Actuation |
| `group`, `user` | bookkeeping | unused |

**qpos semantics**: one scalar per slide/hinge joint, stored in kinematic
tree order in `mjData.qpos`/`qvel`. In the 2-DOF model: `qpos[0]` = cart
offset along the rail (m), `qpos[1]` = pole angle from the authored pose
(rad). `qpos = 0` reproduces the authored XML configuration exactly.

### slide (cart on the rail)

One translational DOF along `axis` through `pos`. `range` is in **meters**
of travel. The joint `pos` is only cosmetic — anchor the body frame so the
cart's COM/geometry lands right, and put the origin wherever the authored
joint origin sits (M2/M3 decision, but the schema imposes no constraint).

A slide is a **constraint, not a bearing**: the joint is the only freedom
between parent and child (the rest is weld), so rotation about the axis is
impossible by construction. Dual-rail hardware (the cart-pole's two
parallel round rails) is exactly what makes the 1-DOF idealization true —
it needs **one** slide joint. A second parallel/collinear slide does not
add stability; it adds a redundant collinear DOF (the one translation
splits into `q1 + q2`). If a mechanism genuinely spins while sliding
(cylindrical joint), compose `slide` + coaxial `hinge` on the same body —
same-body joints apply their transformations in order (`xmlref
#body-joint` intro).

### hinge (pole pivot)

Rotation about `axis` through `pos`, `range` in **radians** (with
`angle="radian"`). The pivot point must be the pole body frame's origin
relationship: `pos` shifts the axis within the body frame, and the body's
`pos` relative to the cart places the pivot in cart coordinates. Axis
horizontal and perpendicular to the swing plane (e.g. `"0 1 0"` if the
rail runs along x in the pole-swing xz-plane). An unpowered hinge needs no
actuator — passive dynamics come from gravity + `damping`/`frictionloss`.

### freejoint (root mobility)

Unchanged from M1: `<freejoint/>` iff `rootMobility="free"`. 3.15.0 adds
`align` (default via `compiler alignfree`, itself default false) — an
inertial-frame alignment optimization for simple free bodies; it silently
rewrites body poses, which would scramble our CAD-exact frames — keep
`alignfree` at its false default (`xmlref #body-freejoint-align`,
`#compiler-alignfree`).

## compiler / option facts touching joints

- `autolimits` `[false,true]` default **true**: infers `limited`,
  `ctrllimited`, `forcelimited`, `actlimited` from the presence of the
  matching `range` attribute (`xmlref #compiler-autolimits`). With
  `autolimits="false"` specifying a range without its `*limited` is an
  error — another reason we always emit `autolimits="true"`.
- `angle` `[radian,degree]`, **"degree" for MJCF** — M1 already emits
  `radian`; this is what makes hinge `range` radians.
- `coordinate` `global` **is no longer supported** — local coordinates
  only (`xmlref #compiler-coordinate`); all frames parent-relative.
- `inertiafromgeom` `auto` (explicit `<inertial>` wins — M1 rule).
- `fusestatic` MJCF default **false** (static subtrees stay separate
  bodies — bodies referenced by joints are never fused anyway; note the
  fuser also skips referenced bodies, `xmlref #compiler-fusestatic`).
- `option/timestep` default **0.002 s**; `option/integrator`
  `[Euler, RK4, implicit, implicitfast, discrete]` default **Euler**;
  `option/gravity` `(0, 0, −9.81)` (`xmlref #option-timestep`,
  `#option-integrator`, `#option-gravity`). The `potpori.md` "fixed sim
  cadence" pins these — emit `<option>` once actuation lands, so cadence
  is explicit rather than defaulted.

## Actuation — the model and the `<position>` shortcut

MJCF has exactly **one** actuator element internally (`general`); `<motor>`,
`<position>`, `<velocity>`, `<intvelocity>`, `<damper>`, `<pid>`, … are
*shortcuts* that set `general`'s parameters, and saved XMLs always write
`<general>` (`modeling guide`, *Actuator shortcuts*). Force generation is
affine:

```
scalar_force = gain_term · (act or ctrl) + bias_term
biastype=affine:  bias_term = biasprm[0] + biasprm[1]·length + biasprm[2]·velocity
gaintype=fixed:   gain_term = gainprm[0]
```

(`xmlref #actuator-general-gaintype`, `#actuator-general-biastype`).
`length` is the transmission coordinate: for a **joint** transmission on a
scalar joint, `length = qpos · gear[0]` (`xmlref #actuator-general-joint`)
— with `gear` at its default `"1"`, length is just the joint value in
**meters (slide) or radians (hinge)**, and so is `ctrl`.

### `<position>` (the stepper/belt fake)

Sets: `gaintype fixed, gainprm = kp`, `biastype affine, biasprm =
(0, −kp, −kv)` (`xmlref #actuator-position` header table). Net servo law,
before clamping:

```
force = kp · (ctrl − qpos) − kv · qvel        [slide: N; hinge: N·m]
```

| Attribute | Default | Notes |
|---|---|---|
| `kp` | "1" | Position feedback gain — N/m on a slide transmission. The calibration-fit "tracking grip" of `potpori.md` |
| `kv` | "0" | Damping applied *by the actuator* (N·s/m). 3.15.0 attribute; docs recommend `implicitfast`/`implicit` integrators when nonzero (`xmlref #actuator-position-kv`) |
| `dampratio` | "0" | Alternative to `kv` in damping-ratio units (`2√(kp·m)` with the reflected mass m at `qpos0`); 1 = critically damped. **Exclusive with `kv`** (`xmlref #actuator-position-dampratio`) |
| `timeconst` | "0" | Optional first-order command filter (`filterexact` dynamics); 0 = unfiltered |
| `inheritrange` | "0" | Auto-derive `ctrlrange` from the joint's `range` (× scale); **exclusive with `ctrlrange`**. We author the operating range explicitly, so unused — but it is the knob if the envelope ever just follows travel |
| `gear` | `"1 0 0 0 0 0"` | Transmission scaling; first element only for scalar joints. Belt pitch radius could fold in here — see the worked example for why we keep gear=1 |
| `ctrlrange` | "0 0" | Clamp on `ctrl` input — the **operating-range envelope** |
| `forcerange` | "0 0" | Clamp on output force — the **motor capability** (±F_max) |
| `joint` | — | Exactly one transmission attribute per actuator; ours is always `joint="cart_slide"` |
| `damping`, `armature` | "0" | Actuator-contributed joint damping / reflected rotor inertia, both scaled by gear². Docs recommend placing a target's whole damping/armature in **one** actuator (`xmlref #actuator-general-damping`, `#actuator-general-armature`) — or leave them on the joint; do not split |

**Clamping layers** (modeling guide, *Force limits*): `ctrlrange` clamps
the input; `forcerange` clamps the actuator output; `joint
actuatorfrcrange` clamps the total force reaching the joint. Docs
explicitly: position actuators use `forcerange` "to keep the forces within
bounds" and "usually also require control range clamping to avoid hitting
joint limits" — exactly the `potpori.md` belt-folded-into-ranges design.
With a one-actuator-per-joint model the three collapse to the same clamp
at full saturation; the safety gap (ctrlrange ⊂ joint range) lives in the
XML visibly, per `potpori.md`.

`<motor>` (direct torque/force = gainprm[0]·ctrl) and `<intvelocity>`
(velocity setpoint, integrated) exist as alternatives; the settled choice
(`<position>` on the slide) stands — no need to re-litigate.

## Worked example — the 2-DOF target (`potpori.md` MJCF target)

Schematic (body placement is M3's conjugation work; axis letters assume
the rail along x, pole swinging in the xz-plane — conjugated from the
Unity-authored frames by the M3 exporter):

```xml
<compiler angle="radian" meshdir="meshes" autolimits="true"
          inertiafromgeom="auto" balanceinertia="false"/>
<option timestep="0.002" integrator="implicitfast"/>   <!-- cadence explicit -->
<worldbody>
  <body name="cart" pos="...">
    <joint name="cart_slide" type="slide" axis="1 0 0"
           range="-0.45 0.45"      <!-- mechanical range, m -->
           damping="8.5"           <!-- rail friction, N·s/m (calibration-fit) -->
           armature="1.2e-4"/>     <!-- rotor reflected inertia, kg -->
    <geom type="mesh" mesh="..."/> <inertial .../>
    <body name="pole" pos="...">
      <joint name="pole_hinge" type="hinge" pos="0 0 0" axis="0 1 0"
             range="-3.1 3.1"      <!-- rad, or tighter mechanical stop -->
             damping="0.001"/>     <!-- bearing friction, N·m·s/rad -->
      <geom type="mesh" mesh="..."/> <inertial .../>
    </body>
  </body>
</worldbody>
<actuator>
  <position name="cart_drive" joint="cart_slide"
            kp="180"               <!-- tracking grip, N/m (calibration-fit) -->
            forcerange="-32.7 32.7"<!-- ±τ_stall/0.01528 m -->
            ctrlrange="-0.40 0.40"><!-- operating range, m: safety gap vs joint range -->
  />
</actuator>
```

Number provenance (all from `potpori.md`, *Sim control model*): pitch
radius 15.28 mm (48-tooth GT2 — **to be confirmed**, open item 3), 96
mm/rev, NEMA-17 1.8° ⇒ 0.48 mm/step, **0.03 mm/step at ×16 microstepping**
⇒ the controller advances `ctrl` in 3e-5 m increments; `F_max =
τ_stall / 0.01528` (the −32.7 above assumes τ_stall ≈ 0.5 N·m — verify
against the actual driver's datasheet). kp/damping are **calibration-fit
outputs**, not datasheet constants — the schema just carries where they
land. All slider-joint force quantities are N; all hinge quantities N·m.

Per-part mechanical facts to respect when filling ranges: rail 1 m span,
pole 500 mm ⇒ small-signal pendulum period ≈ 1.2–1.4 s (potpori.md);
switches exist only for homing — never hit outside calibration, hence
ctrlrange strictly inside the mechanical range (the visible safety gap).

## Mesh assets — what the exporter relies on

- `<asset><mesh name file>` accepts binary STL, OBJ, or MuJoCo's binary
  MSH. The asset `scale` attribute exists, but baking units into the
  exported file keeps the STL standalone-correct, so we do that instead.
- **Recentering**: MuJoCo pre-processes every mesh — translated to its
  centre of mass and rotated to principal axes — and composes those
  offsets into the referencing geom's pose. Net effect: with a default
  geom pose the mesh renders exactly at the STL's authored coordinates
  in the body frame — which is why no geom `pos`/`quat` is needed.
  Validation trap: `mjModel.mesh_vert` holds the *centered* vertices,
  not the file's coordinates.
- STL specifics: facet normals are not read as vertex normals (MuJoCo
  generates its own); repeated vertices are removed and faces
  re-indexed. Mesh geoms are convex-hulled for collision.
- Degenerate triangles with area below `mjMINVAL` are a compile error.

## Limit switches — deliberately not in the simulation XML

Decided 2026-10-09: the physical limit switches are **hardware, not model**
— they never appear in the exported MJCF, in any form (no sensors, no
contact-based stops, no homing machinery). The switches exist to serve
homing, and homing is a driver procedure on the real machine (`potpori.md`
*Controller architecture*); the sim has no driver and no firmware to home.
Sim-side safety is structural instead: the joint's mechanical `range` is
the physics stop, `ctrlrange` sits strictly inside it as the policy
envelope, and the firmware envelope clamp is the deployment mirror of
`ctrlrange` — the margin between the two ranges stays visible in the XML.
The MJCF target therefore stands at **2 joints (slide + hinge), 0
sensors**, and stays there: the `potpori.md` "virtual end-switch" idea is
retired by this decision.

## Corrections / flags for existing docs

- `potpori.md` "Sim control model" writes `<sensor><jointlim
  joint="cart_slide"/>` as a possible virtual end-switch. Two problems,
  both settled: there is no `jointlim` sensor in 3.15.0 (the family is
  `jointlimitpos/vel/frc`), and — decided 2026-10-09 — limit switches are
  not part of the simulation XML at all (see *Limit switches* above), so
  `potpori.md`'s sensor line should read "no sensor of any kind".
- `potpori.md` says "joint damping = rail friction" — confirmed sound:
  joint `damping` is linear viscous (implicitly integrated by Euler) and
  `frictionloss` is the dry/stiction analog if the rail needs it. Do not
  confuse either with `<position kv>` (actuator-side damping, subject to
  `forcerange` clamping, *not* implicitly integrated by plain Euler).
- `potpori.md`'s fake-wheel parameters list (`kp`, damping, `forcerange`,
  ramp) maps cleanly onto the schema above; the schema-side "ramp"
  equivalent is either `timeconst` (actuator-side command filter) or the
  controller-side step-rate limit — deployment-parity says the latter
  (the sim must be driven exactly like the Arduino).

## References

- XML reference (stable, 3.15.0):
  <https://mujoco.readthedocs.io/en/stable/XMLreference.html>
  (anchors used: `#body`, `#body-joint`, `#body-joint-*` (pos, axis,
  range, limited, damping, frictionloss, armature, ref, springdamper,
  margin, actuatorfrcrange), `#body-freejoint`, `#compiler-autolimits`,
  `#compiler-angle`, `#compiler-coordinate`, `#compiler-fusestatic`,
  `#compiler-inertiafromgeom`, `#compiler-alignfree`, `#option-timestep`,
  `#option-integrator`, `#option-gravity`, `#actuator`,
  `#actuator-general` + `#actuator-general-*` (ctrlrange, forcerange,
  gear, joint, damping, armature, dyntype, gaintype, biastype),
  `#actuator-position` + `#actuator-position-*` (kp, kv, dampratio,
  timeconst, inheritrange))
- Modeling guide: <https://mujoco.readthedocs.io/en/stable/modeling.html>
  (*Actuator shortcuts*, *Force limits*)
- Release pin: google-deepmind/mujoco latest release **3.15.0** (GitHub
  releases API, 2026-10-09); readthedocs "stable" tracks it
- Repo: `unity-mujoco.md` (facts, plan, and landed M1–M3 decisions),
  `potpori.md` (physical system, sim control model, M2 actuation fields)
