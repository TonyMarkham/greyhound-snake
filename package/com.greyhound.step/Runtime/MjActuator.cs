using System;

namespace Greyhound.Step
{
    // The serialized mirror of the MJCF <position> actuator element —
    // the full 3.15.0 attribute surface, names verbatim, nothing
    // Unity-specific. Field defaults equal the spec defaults, so a
    // fresh asset is a fresh spec element; the exporter writes every
    // attribute verbatim.
    //
    // Deliberately unmirrored, each for a stated reason — no silent
    // omissions: `class` is the defaults-class indirection and we emit
    // no <default> tree, so omitting the attribute means "main" and no
    // other value is expressible here. `jointinparent` is semantically
    // identical to `joint` for hinge/slide transmissions (the
    // parent-frame distinction only matters for ball/free gear axes),
    // and the authoring model carries exactly one joint target. The
    // other transmission attributes (`tendon`, `cranksite`,
    // `slidersite`, `site`, `refsite`) are alternative transmissions —
    // the authoring model is joint-transmission by construction (the
    // spec requires exactly one transmission attribute).
    // `nsample`/`interp` are not part of <position>'s attribute list at
    // 3.15.0. `user` only exists when the model declares
    // nuser_actuator > 0, which we never do.
    //
    // ctrlrange is in joint units (meters for a slide target, radians
    // for a hinge); forcerange clamps in N or N·m; lengthrange clamps
    // the transmission length. `lengthrange` "0 0" is the unset state —
    // a zero-width feasible length is meaningless — so the exporter
    // omits the attribute then. `kv` and `dampratio` are exclusive,
    // as are `inheritrange` and an authored `ctrlrange`; the form
    // warns about both conflicts. ctrllimited/forcelimited emit
    // verbatim as "true"/"false": explicit false disables clamping
    // even with a range present, which keeps the file valid under
    // autolimits without heuristics.
    [Serializable]
    public sealed class MjActuator
    {
        // The element selector (not a spec attribute): which MJCF
        // actuator shortcut this asset mirrors. v1 emits <position>.
        public StepActuatorType type;

        public int group;

        public float delay;

        public bool ctrllimited;

        public float ctrlLo;

        public float ctrlHi;

        public bool forcelimited;

        public float forceLo;

        public float forceHi;

        public float lengthrangeLo;

        public float lengthrangeHi;

        public float[] gear = { 1f, 0f, 0f, 0f, 0f, 0f };

        public float cranklength;

        public float kp = 1f;

        public float kv;

        public float dampratio;

        public float timeconst;

        public float inheritrange;

        public float damping;

        public float armature;
    }
}
